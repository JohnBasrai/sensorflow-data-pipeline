use axum::Router;
use sqlx::PgPool;

use crate::Config;

mod get_readings;

// ---

pub fn router(pool: PgPool, config: Config) -> Router {
    // ---
    Router::new()
        .merge(get_readings::router())
        .with_state((pool, config))
}
