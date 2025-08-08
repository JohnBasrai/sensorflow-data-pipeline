use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use serial_test::serial;

#[derive(Debug, Deserialize)]
struct SensorReading {
    mesh_id: String,
    device_id: String,
    timestamp_utc: DateTime<Utc>,
    temperature_c: f32,
    temperature_f: f32,
    humidity: f32,
    temperature_alert: bool,
    humidity_alert: bool,
}

fn base_url() -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into())
}

#[tokio::test]
#[serial]
async fn readings_endpoint_transforms_ok() -> Result<()> {
    // ---

    let base = base_url();
    let url = format!("{}/sql/readings?limit=50", base);

    let client = Client::new();
    let readings: Vec<SensorReading> = client.get(&url).send().await?.json().await?;

    assert!(!readings.is_empty(), "No readings returned from {}", url);

    // Test the 3 core transformations on sample data
    for r in readings.iter().take(5) {
        // ---

        // 0) Basic field validation (prevents unused field warnings)
        assert!(!r.mesh_id.is_empty(), "mesh_id should not be empty");
        assert!(!r.device_id.is_empty(), "device_id should not be empty");
        assert!(
            r.timestamp_utc > DateTime::from_timestamp(0, 0).unwrap(),
            "timestamp_utc should be valid"
        );

        // 1) Temperature conversion: °C → °F
        let expected_f = (r.temperature_c * 9.0 / 5.0) + 32.0;
        assert!(
            (r.temperature_f - expected_f).abs() < 0.01,
            "Temperature conversion failed: {}°C should be {:.2}°F, got {:.2}°F",
            r.temperature_c,
            expected_f,
            r.temperature_f
        );

        // 2) Temperature alerts: < -10°C or > 60°C
        let expected_temp_alert = r.temperature_c < -10.0 || r.temperature_c > 60.0;
        assert_eq!(
            r.temperature_alert, expected_temp_alert,
            "Temperature alert wrong: {}°C should have alert={}, got={}",
            r.temperature_c, expected_temp_alert, r.temperature_alert
        );

        // 3) Humidity alerts: < 10% or > 90%
        let expected_humidity_alert = r.humidity < 10.0 || r.humidity > 90.0;
        assert_eq!(
            r.humidity_alert, expected_humidity_alert,
            "Humidity alert wrong: {}% should have alert={}, got={}",
            r.humidity, expected_humidity_alert, r.humidity_alert
        );
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn filters_work_end_to_end() -> Result<()> {
    // ---
    let base = base_url();
    let client = Client::new();

    filter_by_device(&client, &base).await?;
    filter_by_mesh(&client, &base).await?;
    filter_by_ts_range(&client, &base).await?;

    Ok(())
}

async fn filter_by_device(client: &Client, base: &str) -> Result<()> {
    // ---
    let resp = client
        .get(&format!("{}/sql/readings", base))
        .query(&[("device", "device-001"), ("limit", "10")])
        .send()
        .await?;
    let readings: Vec<SensorReading> = resp.json().await?;
    assert!(readings.len() <= 10);
    for r in &readings {
        assert_eq!(r.device_id, "device-001");
    }
    Ok(())
}

async fn filter_by_mesh(client: &Client, base: &str) -> Result<()> {
    // ---
    let resp = client
        .get(&format!("{}/sql/readings", base))
        .query(&[("mesh", "mesh-001"), ("limit", "10")])
        .send()
        .await?;
    let readings: Vec<SensorReading> = resp.json().await?;
    for r in &readings {
        assert_eq!(r.mesh_id, "mesh-001");
    }
    Ok(())
}

async fn filter_by_ts_range(client: &Client, base: &str) -> Result<()> {
    // ---
    // Anchor on a real ts
    let one: Vec<SensorReading> = client
        .get(&format!("{}/sql/readings", base))
        .query(&[("limit", "1")])
        .send()
        .await?
        .json()
        .await?;
    assert!(!one.is_empty(), "need at least one reading");
    let ts: DateTime<Utc> = one[0].timestamp_utc;
    let range = format!("{},{}", ts.to_rfc3339(), ts.to_rfc3339());

    // Happy path
    let ok = client
        .get(&format!("{}/sql/readings", base))
        .query(&[("timestamp_range", range.as_str()), ("limit", "100")])
        .send()
        .await?;
    assert!(ok.status().is_success());
    let ranged: Vec<SensorReading> = ok.json().await?;
    assert!(ranged.iter().any(|r| r.timestamp_utc == ts));

    // Bad input -> 422
    let bad = client
        .get(&format!("{}/sql/readings", base))
        .query(&[("timestamp_range", "not-a-timestamp")])
        .send()
        .await?;
    assert_eq!(bad.status(), StatusCode::UNPROCESSABLE_ENTITY);

    Ok(())
}

#[tokio::test]
#[serial]
async fn timestamp_range_bad_returns_422() -> Result<(), Box<dyn std::error::Error>> {
    // ---

    // Use BASE_URL if set; default to local compose port
    let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let client = Client::new();

    // Each of these should be rejected by the handler with 422
    let bad_ranges = [
        "not-a-timestamp",                           // totally invalid
        "2025-03-21T00:00:00Z",                      // missing comma
        "2025-03-22T00:00:00Z,2025-03-21T00:00:00Z", // start > end
    ];

    for r in bad_ranges {
        let resp = client
            .get(&format!("{}/sql/readings", base))
            .query(&[("timestamp_range", r)])
            .send()
            .await?;

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "range={r}");
        let body: Value = resp.json().await?;
        assert_eq!(
            body.get("error"),
            Some(&Value::String("invalid timestamp_range".into()))
        );
        assert!(
            body.get("hint").is_some(),
            "missing hint in 422 body for range={r}"
        );
    }

    Ok(())
}
