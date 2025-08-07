//! Database schema management for `codemetal-sensorflow`.
//!
//! Ensures required tables and indexes exist before serving requests.
//! Applied once on startup from `main.rs` (EMBP: single gateway call).

use anyhow::Result;
use sqlx::PgPool;

// ---

/// Create or update the database schema (idempotent).
///
/// Creates the `sensor_data` table for transformed readings and `mesh_summary`
/// table for aggregations. Safe to call on every startup; no-op if objects already exist.
///
/// Errors are propagated if any SQL execution fails.
pub async fn create_schema(pool: &PgPool) -> Result<()> {
    // ---
    let mut tx = pool.begin().await?;

    // Core table for transformed readings served by `/sql/readings`
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sensor_data (
            id                SERIAL PRIMARY KEY,
            mesh_id           TEXT        NOT NULL,
            device_id         TEXT        NOT NULL,
            timestamp_utc     TIMESTAMPTZ NOT NULL,
            timestamp_est     TIMESTAMPTZ NOT NULL,
            temperature_c     REAL        NOT NULL,
            temperature_f     REAL        NOT NULL,
            humidity          REAL        NOT NULL,
            status            TEXT,
            temperature_alert BOOLEAN,
            humidity_alert    BOOLEAN
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Summary table for mesh aggregations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mesh_summary (
            mesh_id               TEXT PRIMARY KEY,
            avg_temperature_c     REAL NOT NULL,
            avg_temperature_f     REAL NOT NULL,
            avg_humidity          REAL NOT NULL,
            reading_count         INTEGER NOT NULL
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Basic indexes for common queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sensor_data_mesh_id
            ON sensor_data (mesh_id);
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sensor_data_device_id
            ON sensor_data (device_id);
        "#,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
