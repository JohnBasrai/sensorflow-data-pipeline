// src/routes/health.rs
//! API health check endpoint for the Sensorflow backend.
//!
//! This module defines the `/health` route used by container orchestrators
//! (e.g., Docker, Kubernetes) and CI pipelines to verify that the service is
//! running and able to respond to HTTP requests. It is a sibling module in the
//! `routes` directory and follows the Explicit Module Boundary Pattern (EMBP):
//! - Internal to this file: endpoint handler(s) and related types
//! - Exports to the gateway (`mod.rs`): a subrouter containing the `/health` route
//!
//! The gateway merges this subrouter into the top-level API router so that
//! `main.rs` does not need to know about individual endpoints.

use axum::{routing::get, Json, Router};
use serde::Serialize;

/// JSON response body for the `/health` endpoint.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

/// Handle `GET /health`.
///
/// Returns a static JSON object indicating the API is reachable and
/// functioning. This endpoint is deliberately lightweight and does not
/// touch the database or other external services.
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Create a subrouter containing the `/health` route.
///
/// This router is generic over the application state so it can merge cleanly
/// with the gateway router, regardless of the state type (e.g., `(PgPool, Config)`).
///
/// # Returns
/// A [`Router<S>`] with a single GET `/health` route.
///
/// # Type Parameters
/// - `S`: Application state type shared by all routes in the gateway.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/health", get(health))
}
