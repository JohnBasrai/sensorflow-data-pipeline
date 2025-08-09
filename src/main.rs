//! Application entry point for the `codemetal-sensorflow` backend service.
//!
//! This binary orchestrates the full startup sequence for the sensor data
//! pipeline API, including:
//! - Loading configuration from environment variables or `.env`
//! - Initializing structured logging/tracing
//! - Establishing a PostgreSQL connection pool
//! - Creating the database schema if it does not exist
//! - Mounting all API routes via the `routes` gateway (EMBP pattern)
//! - Binding the Axum HTTP server and serving requests
//!
//! # Environment Variables
//! - `DATABASE_URL` (**required**) – PostgreSQL connection string
//! - `DB_POOL_MAX` (optional) – maximum number of DB connections (default: 5)
//! - `AXUM_LOG_LEVEL` (optional) – log verbosity (default: `debug`)
//! - `AXUM_SPAN_EVENTS` (optional) – span event mode for tracing
//!
//! This module follows the Explicit Module Boundary Pattern (EMBP) by
//! delegating schema setup to `schema`, configuration parsing to `config`,
//! and route registration to `routes`.
use std::{env, io::IsTerminal, net::SocketAddr};

use axum::Router;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

use anyhow::Result;

mod config;
mod models;
mod routes;
mod schema;

pub use config::Config;

// These are not used here but they are imported to be used by routes/*.rs, that way
// refactoring is eaiser since router/*.rs do not have knowledge of config.rs, only
// of their parent module (main.rs)
pub use models::{RawSensorReading, SensorReading};

// ---

#[tokio::main]
async fn main() -> Result<()> {
    // ---
    init_tracing();
    dotenv().ok();

    let cfg = config::load_from_env()?;
    cfg.log_config();

    tracing::info!("Attempting to connect to database: {}", cfg.db_url);

    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_pool_max)
        .connect(&cfg.db_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database '{}': {}", cfg.db_url, e))?;

    tracing::info!("Successfully connected to database");

    schema::create_schema(&pool).await?;

    // Build app from routes gateway (EMBP)
    let app: Router = routes::router(pool.clone(), cfg);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ---

/// Initialize the global tracing subscriber for structured logging.
///
/// This function configures the [`tracing_subscriber`] with:
/// - Log target, file, and line number output enabled
/// - Color output controlled by TTY detection and `FORCE_COLOR` env var:
///   - `FORCE_COLOR=1|true|yes`: force colors on
///   - `FORCE_COLOR=0|false|no`: force colors off  
///   - unset or other values: auto-detect TTY
/// - Span event emission mode controlled by the `AXUM_SPAN_EVENTS` env var:
///   - `"full"`       : emit ENTER, EXIT, and CLOSE events with timing
///   - `"enter_exit"` : emit ENTER and EXIT only
///   - unset or other values: emit CLOSE events only (default)
/// - Log level controlled by the `AXUM_LOG_LEVEL` env var
///
/// This should be called once at application startup before any logging
/// or tracing macros are invoked. It installs the subscriber globally
/// for the lifetime of the process.
fn init_tracing() {
    // ---
    let span_events = match env::var("AXUM_SPAN_EVENTS").as_deref() {
        Ok("full") => FmtSpan::FULL,
        Ok("enter_exit") => FmtSpan::ENTER | FmtSpan::EXIT,
        _ => FmtSpan::CLOSE,
    };

    // Determine if we should use colors
    let use_color = match env::var("FORCE_COLOR").as_deref() {
        Ok("1") | Ok("true") | Ok("yes") => true,
        Ok("0") | Ok("false") | Ok("no") => false,
        _ => std::io::stdout().is_terminal(),
    };

    // Use RUST_LOG if available, otherwise fall back to AXUM_LOG_LEVEL
    let env_filter = if env::var("RUST_LOG").is_ok() {
        EnvFilter::from_default_env()
    } else {
        let level = match env::var("AXUM_LOG_LEVEL").ok().as_deref() {
            Some("trace") => "trace",
            Some("debug") => "debug",
            Some("info") => "info",
            Some("warn") => "warn",
            Some("error") => "error",
            _ => "debug",
        };
        EnvFilter::new(format!("{level},sqlx::query=warn"))
    };

    tracing_subscriber::fmt()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(span_events)
        .with_env_filter(env_filter)
        .with_ansi(use_color)
        .compact()
        .init();
}
