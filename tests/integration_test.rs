use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

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

#[tokio::test]
async fn readings_endpoint_transforms_ok() -> Result<()> {
    // ---

    let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
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
async fn filtering_works() -> Result<()> {
    // ---
    let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let client = Client::new();

    // Test device_id filter
    let url = format!("{}/sql/readings?device_id=device-001&limit=10", base);
    let readings: Vec<SensorReading> = client.get(&url).send().await?.json().await?;

    // All returned readings should have the specified device_id
    for reading in &readings {
        assert_eq!(reading.device_id, "device-001", "Device filter failed");
    }

    // Test limit
    assert!(readings.len() <= 10, "Limit filter failed");

    Ok(())
}
