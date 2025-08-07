//! Configuration loader for the `codemetal-sensorflow` backend service.
//!
//! This module centralizes all runtime configuration values and their defaults,
//! loading from environment variables (with optional `.env` file support
//! provided by the caller). By consolidating configuration logic here, we
//! avoid scattering `env::var` calls throughout the codebase, improving
//!
use std::env;

use anyhow::{anyhow, Result};

/// Parse an optional integer environment variable with a default value.
macro_rules! parse_env_u32 {
    ($var_name:expr, $default:expr) => {
        env::var($var_name)
            .ok()
            .map(|v| v.parse::<u32>())
            .transpose()
            .map_err(|e| anyhow!("Invalid {}: {}", $var_name, e))?
            .unwrap_or($default)
    };
}

/// Parse a required string environment variable.
macro_rules! require_env {
    ($var_name:expr) => {
        env::var($var_name)
            .map_err(|_| anyhow!("{} must be set in .env or environment", $var_name))?
    };
}

/// Strongly typed application configuration.
///
/// All fields are immutable after loading, ensuring a consistent configuration
/// snapshot for the lifetime of the application.
#[derive(Debug, Clone)]
pub struct Config {
    // ---
    /// PostgreSQL connection string.
    pub db_url: String,

    /// Maximum number of database connections in the pool.
    pub db_pool_max: u32,

    /// Sensor data API base URL.
    pub api_url: String,

    /// Maximum number of API pages to fetch (safety limit).
    pub api_max_pages: u32,
}

/// Load configuration from environment variables with defaults.
///
/// Required:
/// - `DATABASE_URL` – PostgreSQL connection string
/// - `SENSOR_API_URL` – Sensor data API base URL
///
/// Optional:
/// - `DB_POOL_MAX` – max DB connections (default: 5)
/// - `API_MAX_PAGES` – max API pages to fetch (default: 100)
///
/// Returns an error if any required variable is missing or invalid.
pub fn load_from_env() -> Result<Config> {
    // ---
    let db_url = require_env!("DATABASE_URL");
    let api_url = require_env!("SENSOR_API_URL");
    let db_pool_max = parse_env_u32!("DB_POOL_MAX", 5);
    let api_max_pages = parse_env_u32!("API_MAX_PAGES", 100);

    Ok(Config {
        db_url,
        api_url,
        db_pool_max,
        api_max_pages,
    })
}

impl Config {
    /// Log the loaded configuration for debugging purposes.
    ///
    /// Masks sensitive information like database passwords while showing
    /// all configuration values that were loaded.
    pub fn log_config(&self) {
        // ---
        // Mask the password in the database URL for security
        let masked_db_url = if let Some(at_pos) = self.db_url.rfind('@') {
            if let Some(colon_pos) = self.db_url[..at_pos].rfind(':') {
                format!(
                    "{}:****{}",
                    &self.db_url[..colon_pos],
                    &self.db_url[at_pos..]
                )
            } else {
                self.db_url.clone()
            }
        } else {
            self.db_url.clone()
        };

        tracing::info!("Configuration loaded:");
        tracing::info!("  DATABASE_URL   : {}", masked_db_url);
        tracing::info!("  SENSOR_API_URL : {}", self.api_url);
        tracing::info!("  DB_POOL_MAX    : {}", self.db_pool_max);
        tracing::info!("  API_MAX_PAGES  : {}", self.api_max_pages);
    }
}
