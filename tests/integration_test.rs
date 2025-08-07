// tests/integration_test.rs
// Place this file in: tests/integration_test.rs (in the root of your main project)

use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SensorReading {
    pub mesh_id: String,
    pub device_id: String,
    pub timestamp_utc: DateTime<Utc>,
    pub timestamp_est: DateTime<Utc>,
    pub temperature_c: f32,
    pub temperature_f: f32,
    pub humidity: f32,
    pub status: String,
    pub temperature_alert: bool,
    pub humidity_alert: bool,
}

#[tokio::test]
async fn test_sensor_pipeline_integration() -> Result<()> {
    println!("🔍 Testing Sensor Data Pipeline Integration");
    println!("==========================================");

    let client = Client::new();
    let response = client.get("http://localhost:8080/sql/readings").send().await?;
    
    assert!(response.status().is_success(), "API request failed: {}", response.status());

    let readings: Vec<SensorReading> = response.json().await?;
    assert!(!readings.is_empty(), "No data returned from PostgreSQL");
    
    println!("✅ API responds with {} records from PostgreSQL", readings.len());
    
    // Test the 4 REQUIRED transformations from the problem description
    // Test only first 5 records for speed
    for (i, reading) in readings.iter().take(5).enumerate() {
        println!("\n📋 Testing record {} (mesh: {}, device: {})", i+1, reading.mesh_id, reading.device_id);
        
        // Required Test 1: Temperature conversion
        let expected_f = (reading.temperature_c * 9.0 / 5.0) + 32.0;
        let temp_diff = (reading.temperature_f - expected_f).abs();
        assert!(temp_diff < 0.01, 
            "Temperature conversion failed: {}°C should be {:.1}°F, got {}°F", 
            reading.temperature_c, expected_f, reading.temperature_f);
        println!("  ✅ Temperature: {}°C → {}°F", reading.temperature_c, reading.temperature_f);
        
        // Required Test 2: Timezone conversion (EST = UTC - 5)
        let expected_est = reading.timestamp_utc - chrono::Duration::hours(5);
        assert_eq!(reading.timestamp_est, expected_est,
            "Timezone conversion failed: UTC {} should become EST {}", 
            reading.timestamp_utc, expected_est);
        println!("  ✅ Timezone: UTC {} → EST {}", 
            reading.timestamp_utc.format("%H:%M"), 
            reading.timestamp_est.format("%H:%M"));
        
        // Required Test 3: Temperature alerts
        let expected_temp_alert = reading.temperature_c < -10.0 || reading.temperature_c > 60.0;
        assert_eq!(reading.temperature_alert, expected_temp_alert,
            "Temperature alert wrong: {}°C should have alert={}, got={}", 
            reading.temperature_c, expected_temp_alert, reading.temperature_alert);
        println!("  ✅ Temp Alert: {} ({}°C)", reading.temperature_alert, reading.temperature_c);
        
        // Required Test 4: Humidity alerts  
        let expected_humidity_alert = reading.humidity < 10.0 || reading.humidity > 90.0;
        assert_eq!(reading.humidity_alert, expected_humidity_alert,
            "Humidity alert wrong: {}% should have alert={}, got={}", 
            reading.humidity, expected_humidity_alert, reading.humidity_alert);
        println!("  ✅ Humidity Alert: {} ({}%)", reading.humidity_alert, reading.humidity);
    }

    println!("\n🎉 All required transformations working correctly!");
    Ok(())
}