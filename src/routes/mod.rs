use axum::Router;
use sqlx::PgPool;

use crate::Config;

mod health;
mod readings;

// ---

pub fn router(pool: PgPool, config: Config) -> Router {
    // ---
    Router::new()
        .merge(readings::router())
        .merge(health::router())
        .with_state((pool, config))
}
