use axum::{
    extract::Query, extract::State, http::StatusCode, response::IntoResponse, routing::get, Json,
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{debug, error, info};

use crate::{Config, RawSensorReading, SensorReading};

// ---

pub fn router() -> Router<(PgPool, Config)> {
    // ---
    Router::new().route("/sql/readings", get(handler))
}

async fn handler(
    Query(params): Query<ReadingsQuery>,
    State((pool, config)): State<(PgPool, Config)>,
) -> impl IntoResponse {
    // ---
    info!("GET /sql/readings - Starting pipeline");

    // Step 1: Fetch data from API
    debug!("GET /sql/readings - Step 1");

    let api_url = &config.api_url;
    let api_max_pages = config.api_max_pages;

    let raw_data = match fetch_sensor_data(api_url, api_max_pages).await {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to fetch sensor data: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to fetch data"),
            )
                .into_response();
        }
    };

    // Step 2: Transform and store
    debug!("GET /sql/readings - Step 2");

    let mut transformed_readings = Vec::new();
    for raw in raw_data {
        let transformed = raw.to_transformed();

        // Store in sensor_data table
        if let Err(e) = store_sensor_reading(&pool, &transformed).await {
            error!("Failed to store reading: {}", e);
            continue;
        }

        transformed_readings.push(transformed);
    }

    // Step 3: Update mesh summaries
    debug!("GET /sql/readings - Step 3");

    if let Err(e) = update_mesh_summaries(&pool).await {
        error!("Failed to update summaries: {}", e);
    }

    // Step 4: Apply filters and return data
    let filtered_readings = apply_filters(transformed_readings, &params);
    info!(
        "Pipeline complete, returning {} readings",
        filtered_readings.len()
    );
    debug!("GET /sql/readings - Returning OK");
    (StatusCode::OK, Json(filtered_readings)).into_response()
}

// ---

/// Fetch paginated sensor data from external API
async fn fetch_sensor_data(
    base_url: &str,
    max_pages: u32,
) -> Result<Vec<RawSensorReading>, Box<dyn std::error::Error>> {
    // ---
    let client = reqwest::Client::new();
    let mut all_data = Vec::new();
    let mut cursor: Option<String> = None;
    let mut page_count = 0;

    loop {
        if page_count >= max_pages {
            tracing::debug!(
                "Hit page limit of {}, stopping pagination. Fetched {} records so far.",
                max_pages,
                all_data.len()
            );
            break;
        }
        page_count += 1;

        let url = if let Some(ref cursor) = cursor {
            format!("{}?cursor={}", base_url, cursor)
        } else {
            base_url.to_string()
        };

        tracing::debug!("Fetching page {} from: {}", page_count, url);

        let response: serde_json::Value = client.get(&url).send().await?.json().await?;

        tracing::debug!("Page {} raw response: {}", page_count, response);

        if let Some(data) = response.get("results").and_then(|d| d.as_array()) {
            tracing::debug!(
                "Page {} found data array with {} items",
                page_count,
                data.len()
            );
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

async fn store_sensor_reading(pool: &PgPool, reading: &SensorReading) -> Result<(), sqlx::Error> {
    // ---
    sqlx::query(
        r#"
        INSERT INTO sensor_data (
            mesh_id, device_id, timestamp_utc,
            temperature_c, temperature_f, humidity, status,
            temperature_alert, humidity_alert
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(&reading.mesh_id)
    .bind(&reading.device_id)
    .bind(&reading.timestamp_utc)
    .bind(reading.temperature_c)
    .bind(reading.temperature_f)
    .bind(reading.humidity)
    .bind(&reading.status)
    .bind(reading.temperature_alert)
    .bind(reading.humidity_alert)
    .execute(pool)
    .await?;

    Ok(())
}

/// Store a transformed sensor reading in the database
async fn update_mesh_summaries(pool: &PgPool) -> Result<(), sqlx::Error> {
    // ---
    sqlx::query(
        r#"
        INSERT INTO mesh_summary (mesh_id, avg_temperature_c, avg_temperature_f, avg_humidity, reading_count)
        SELECT 
            mesh_id,
            AVG(temperature_c) as avg_temperature_c,
            AVG(temperature_f) as avg_temperature_f,
            AVG(humidity) as avg_humidity,
            COUNT(*) as reading_count
        FROM sensor_data 
        GROUP BY mesh_id
        ON CONFLICT (mesh_id) DO UPDATE SET
            avg_temperature_c = EXCLUDED.avg_temperature_c,
            avg_temperature_f = EXCLUDED.avg_temperature_f,
            avg_humidity = EXCLUDED.avg_humidity,
            reading_count = EXCLUDED.reading_count
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Query parameters for filtering sensor readings
#[derive(Debug, Deserialize)]
pub struct ReadingsQuery {
    device_id: Option<String>,
    mesh_id: Option<String>,
    /// Timestamp range filter (e.g., "2025-03-21T00:00:00Z,2025-03-22T00:00:00Z")
    timestamp_range: Option<String>,
    limit: Option<u32>,
}

/// Apply query filters to sensor readings
fn apply_filters(readings: Vec<SensorReading>, params: &ReadingsQuery) -> Vec<SensorReading> {
    // ---
    info!("Apply filter: {:?}", params);
    readings
        .into_iter()
        .filter(|r| {
            params
                .device_id
                .as_ref()
                .map_or(true, |id| &r.device_id == id)
        })
        .filter(|r| params.mesh_id.as_ref().map_or(true, |id| &r.mesh_id == id))
        .filter(|_r| {
            if let Some(_range) = &params.timestamp_range {
                // Parse "start,end" format and filter by timestamp_utc
                // ... implementation
                todo!()
            } else {
                true
            }
        })
        .take(params.limit.unwrap_or(1000) as usize)
        .collect()
}
