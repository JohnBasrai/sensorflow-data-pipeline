use axum::{
    extract::Query, extract::State, http::StatusCode, response::IntoResponse, routing::get, Json,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::{error, info};

use crate::{Config, RawSensorReading, SensorReading};

// ---

pub fn router() -> Router<(PgPool, Config)> {
    // ---
    Router::new().route("/sql/readings", get(handler))
}

/// Handle `GET /sql/readings`.
/// Validates params (422 on bad `timestamp_range`), ingests once if the DB is empty,
/// then loads from Postgres, applies filters (`device_id`, `mesh_id`, `timestamp_range`, `limit`),
/// and returns the readings as JSON.
async fn handler(
    Query(params): Query<ReadingsQuery>,
    State((pool, config)): State<(PgPool, Config)>,
) -> impl IntoResponse {
    // ---
    info!("GET /sql/readings - Starting pipeline");

    // 0) Validate timestamp_range (422 on bad input)
    if let Some(raw) = params.timestamp_range.as_deref() {
        if parse_timestamp_range(raw).is_none() {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiError {
                    error: "invalid timestamp_range",
                    hint:  r#"use RFC3339 "start,end" (e.g. 2025-03-21T00:00:00Z,2025-03-22T00:00:00Z)"#,
                }),
            ).into_response();
        }
    }

    let api_url = &config.api_url;
    let api_max_pages = config.api_max_pages;

    // 1) Ingest once if empty
    if let Err(e) = ensure_data_loaded(&pool, api_url, api_max_pages).await {
        error!("Ingest failed: {}", e);
        // TODO: Production would distinguish upstream (502) vs internal (500) errors
        return (StatusCode::INTERNAL_SERVER_ERROR, Json("ingest failed")).into_response();
    }

    // 2) Load from DB with filters applied at database level
    let readings = match load_filtered_readings(&pool, &params).await {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to load readings: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json("load failed")).into_response();
        }
    };

    info!("Pipeline complete, returning {} readings", readings.len());
    (StatusCode::OK, Json(readings)).into_response()
}

// ---

/// Fetch all pages from the upstream sensor API.
///
/// Starts at `base_url`, follows `next_cursor` until exhausted or `max_pages` reached,
/// and returns the concatenated `RawSensorReading` list. Logs each page at `debug` level.
///
/// Notes:
/// - Uses a new `reqwest::Client` per call (cheap). Consider reusing if hot-path.
/// - Silently skips JSON items that fail to deserialize (logs at `debug`).
/// - Stops early when `max_pages` is hit to protect the backend.
async fn fetch_sensor_data(
    base_url: &str,
    max_pages: u32,
) -> Result<Vec<RawSensorReading>, Box<dyn std::error::Error>> {
    // ---

    // New client per call; fine here, could reuse if calling this often.
    let client = reqwest::Client::new();

    let mut all_data = Vec::new();
    let mut cursor: Option<String> = None;
    let mut page_count = 0;

    // https://use-the-index-luke.com/sql/partial-results/fetch-next-page

    // keep fetching pages until max_pages or no more data
    loop {
        // Guardrail: don’t hammer upstream forever.
        if page_count >= max_pages {
            tracing::debug!(
                "Hit page limit of {}, stopping pagination. Fetched {} records so far.",
                max_pages,
                all_data.len()
            );
            break;
        }
        page_count += 1;

        // Build URL, use cursor if we have it
        let url = if let Some(ref cursor) = cursor {
            format!("{base_url}?cursor={cursor}")
        } else {
            base_url.to_string()
        };

        tracing::debug!("Fetching page {} from: {}", page_count, url);

        // Fetch + parse the page payload as generic JSON.
        let response: serde_json::Value = client.get(&url).send().await?.json().await?;

        tracing::debug!("Page {} raw response: {}", page_count, response);

        // Extract "results" array; skip page if missing/malformed.
        if let Some(data) = response.get("results").and_then(|d| d.as_array()) {
            tracing::debug!(
                "Page {} found data array with {} items",
                page_count,
                data.len()
            );

            // Deserialize each item; keep going on per-item errors.
            for (i, item) in data.iter().enumerate() {
                match serde_json::from_value::<RawSensorReading>(item.clone()) {
                    Ok(reading) => {
                        all_data.push(reading);
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to parse item {} on page {}: {} - Raw item: {}",
                            i,
                            page_count,
                            e,
                            item
                        );
                    }
                }
            }
        } else {
            tracing::debug!(
                "Page {} response missing 'results' field or not an array",
                page_count
            );
        }

        // Advance pagination; stop when there is no next cursor.
        cursor = response
            .get("next_cursor")
            .and_then(|c| c.as_str())
            .map(String::from);

        tracing::debug!("Page {} next_cursor: {:?}", page_count, cursor);

        if cursor.is_none() {
            tracing::info!(
                "No more pages, stopping. Total records fetched: {}",
                all_data.len()
            );
            break;
        }
    }

    tracing::info!(
        "Finished fetching {} total records from {} pages",
        all_data.len(),
        page_count
    );
    Ok(all_data)
}

/// Insert one normalized reading into `sensor_data`.
///
/// - Uses a parameterized `INSERT`
/// - No string interpolation → safe from SQL injection; `sqlx` handles quoting & types.
/// - Executes via the provided `PgPool`; returns `sqlx::Error` on constraint/type failures.
/// - For bulk ingest, wrap calls in a single transaction or accept a generic `Executor`.
async fn store_sensor_reading(pool: &PgPool, reading: &SensorReading) -> Result<(), sqlx::Error> {
    // ---
    sqlx::query(
        r#"
        INSERT INTO sensor_data (
            mesh_id, device_id, timestamp_utc,
            temperature_c, humidity, status,
            temperature_alert, humidity_alert
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(&reading.mesh_id)
    .bind(&reading.device_id)
    .bind(reading.timestamp_utc)
    .bind(reading.temperature_c)
    .bind(reading.humidity)
    .bind(&reading.status)
    .bind(reading.temperature_alert)
    .bind(reading.humidity_alert)
    .execute(pool)
    .await?;

    Ok(())
}

/// Recompute per-mesh aggregates from `sensor_data` and upsert into `mesh_summary`.
/// Aggregates all history (AVG temps/humidity, COUNT) and uses ON CONFLICT(mesh_id) to update.
async fn update_mesh_summaries(pool: &PgPool) -> Result<(), sqlx::Error> {
    // ---

    // Run one SQL that groups sensor_data by mesh_id and calculates:
    //     - avg_temperature_c,
    //     - avg_humidity
    //     - reading_count
    //
    // Write into table mesh_summary using ON CONFLICT (mesh_id) DO UPDATE
    // (so each mesh has one row that gets updated).
    //
    // Scope: aggregates all rows in sensor_data (no time window).
    sqlx::query(
        r#"
        INSERT INTO mesh_summary (mesh_id, avg_temperature_c, avg_humidity, reading_count)
        SELECT 
            mesh_id,
            AVG(temperature_c) as avg_temperature_c,
            AVG(humidity) as avg_humidity,
            COUNT(*) as reading_count
        FROM sensor_data 
        GROUP BY mesh_id
        ON CONFLICT (mesh_id) DO UPDATE SET
            avg_temperature_c = EXCLUDED.avg_temperature_c,
            avg_humidity = EXCLUDED.avg_humidity,
            reading_count = EXCLUDED.reading_count
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Query parameters for filtering sensor readings
#[derive(Debug, Deserialize)]
pub struct ReadingsQuery {
    // ---
    #[serde(alias = "device", alias = "deviceId", alias = "deviceID")]
    device_id: Option<String>,

    #[serde(alias = "mesh", alias = "meshId", alias = "meshID")]
    mesh_id: Option<String>,

    /// Timestamp range filter (e.g., "2025-03-21T00:00:00Z,2025-03-22T00:00:00Z")
    #[serde(alias = "ts_range", alias = "timestampRange")]
    timestamp_range: Option<String>,
    limit: Option<u32>,
}

/// Type alias for timestamp range parsing result: (start, end) where each can be None for open ranges
type TimestampRange = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

/// Parse `"start,end"` (RFC3339) into UTC datetimes.
/// Supports open ends (`"start,"`, `",end"`). Returns `None` on parse error or if `start > end`.
fn parse_timestamp_range(s: &str) -> Option<TimestampRange> {
    // ---
    // Expected timestamp syntax (RFC3339):
    //   2025-03-21T00:00:00Z
    //   2025-03-21T00:00:00+00:00
    //   2025-03-21T00:00:00.123Z
    //   2025-03-21T00:00:00-07:00
    // Range forms (whitespace OK):
    //   "start,end" | "start," | ",end"

    let s = s.trim();
    let (a, b) = s.split_once(',')?;
    let parse = |t: &str| {
        let t = t.trim();
        if t.is_empty() {
            tracing::trace!("Got empty range:{s}");
            None
        } else {
            chrono::DateTime::parse_from_rfc3339(t)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        }
    };
    let start = parse(a);
    let end = parse(b);
    if let (Some(st), Some(en)) = (start, end) {
        if st > en {
            tracing::trace!("Start > End:{s}");
            return None;
        }
    }
    Some((start, end))
}

#[derive(Serialize)]
struct ApiError {
    error: &'static str,
    hint: &'static str,
}

/// Ensure data exists: if `sensor_data` is empty, fetch from the API,
/// transform, persist, and update summaries; otherwise no-op. Used to avoid
/// re-ingesting on every GET.
async fn ensure_data_loaded(
    pool: &PgPool,
    api_url: &str,
    api_max_pages: u32,
) -> Result<(), String> {
    // ---

    // Quick query of posgres then skip ingest if we already have data
    let has_data: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sensor_data)")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    if has_data {
        tracing::debug!("Data present; skipping ingest");
        return Ok(());
    }

    tracing::info!("No data present; performing initial ingest");

    // Expensive call to ingest data and store in DB
    let raw = fetch_sensor_data(api_url, api_max_pages)
        .await
        .map_err(|e| e.to_string())?;

    for r in raw {
        let t = r.to_transformed();
        if let Err(e) = store_sensor_reading(pool, &t).await {
            tracing::error!("store failed: {e}");
        }
    }
    update_mesh_summaries(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Load filtered readings from `sensor_data` using database-level filtering.
///
/// Builds dynamic SQL queries with proper parameter binding. PostgreSQL automatically
/// selects the optimal index based on query filters:
///   - Single filters use corresponding single-column indexes
///   - Combined filters prefer composite indexes when available
///   - Results ordered by `timestamp_utc DESC` for deterministic output
///   - `LIMIT` applied at database level for memory efficiency
///
/// Available indexes: `device_id`, `mesh_id`, `timestamp_utc`, and composites
/// `(device_id, timestamp_utc)`, `(mesh_id, timestamp_utc)` for optimal performance.
async fn load_filtered_readings(
    pool: &PgPool,
    params: &ReadingsQuery,
) -> Result<Vec<SensorReading>, sqlx::Error> {
    use sqlx::QueryBuilder;

    let mut query = QueryBuilder::new(
        r#"
        SELECT mesh_id, device_id, timestamp_utc,
               temperature_c, humidity, status,
               temperature_alert, humidity_alert
        FROM sensor_data
        WHERE 1=1
        "#,
    );

    // Add device_id filter (uses index)
    if let Some(device_id) = &params.device_id {
        query.push(" AND device_id = ");
        query.push_bind(device_id);
    }

    // Add mesh_id filter (uses index)
    if let Some(mesh_id) = &params.mesh_id {
        query.push(" AND mesh_id = ");
        query.push_bind(mesh_id);
    }

    // Add timestamp range filter
    if let Some(ts_range) = &params.timestamp_range {
        if let Some((start, end)) = parse_timestamp_range(ts_range) {
            if let Some(start_time) = start {
                query.push(" AND timestamp_utc >= ");
                query.push_bind(start_time);
            }
            if let Some(end_time) = end {
                query.push(" AND timestamp_utc <= ");
                query.push_bind(end_time);
            }
        }
    }

    // Add ORDER BY for deterministic results
    query.push(" ORDER BY timestamp_utc DESC");

    // Add LIMIT
    let limit = params.limit.unwrap_or(1000);
    query.push(" LIMIT ");
    query.push_bind(limit as i64);

    // Execute query and map results
    let rows = query.build().fetch_all(pool).await?;

    let readings = rows
        .into_iter()
        .map(|row| SensorReading {
            mesh_id: row.get("mesh_id"),
            device_id: row.get("device_id"),
            timestamp_utc: row.get::<DateTime<Utc>, _>("timestamp_utc"),
            temperature_c: row.get("temperature_c"),
            humidity: row.get("humidity"),
            status: row.get("status"),
            temperature_alert: row.get("temperature_alert"),
            humidity_alert: row.get("humidity_alert"),
        })
        .collect();

    Ok(readings)
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn parses_full_range_and_trims() {
        // ---
        let got = parse_timestamp_range(" 2025-03-21T00:00:00Z , 2025-03-21T01:00:00Z ");
        let (s, e) = got.expect("should parse");
        assert_eq!(s, Some(Utc.with_ymd_and_hms(2025, 3, 21, 0, 0, 0).unwrap()));
        assert_eq!(e, Some(Utc.with_ymd_and_hms(2025, 3, 21, 1, 0, 0).unwrap()));
    }

    #[test]
    fn parses_open_start() {
        // ---
        let got = parse_timestamp_range(",2025-03-22T00:00:00Z").expect("should parse");
        assert!(got.0.is_none());
        assert_eq!(
            got.1,
            Some(Utc.with_ymd_and_hms(2025, 3, 22, 0, 0, 0).unwrap())
        );
    }

    #[test]
    fn rejects_reversed_range() {
        assert!(parse_timestamp_range("2025-03-22T00:00:00Z,2025-03-21T00:00:00Z").is_none());
    }

    #[test]
    fn rejects_missing_comma() {
        assert!(parse_timestamp_range("2025-03-21T00:00:00Z").is_none());
    }
}
