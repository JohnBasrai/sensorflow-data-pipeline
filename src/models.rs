//! Simple data models for the sensor pipeline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---

/// Raw sensor data from the API
#[derive(Debug, Deserialize)]
pub struct RawSensorReading {
    // ---
    pub mesh_id: String,
    pub device_id: String,
    pub timestamp: DateTime<Utc>,
    pub temperature_c: f32,
    pub humidity: f32,
    pub status: String,
}

/// Transformed sensor reading for the API response
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SensorReading {
    // ---
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

/// Simple transformation helpers
impl RawSensorReading {
    // ---
    pub fn to_transformed(&self) -> SensorReading {
        // ---
        let temperature_f = (self.temperature_c * 9.0 / 5.0) + 32.0;
        let timestamp_est = self.timestamp - chrono::Duration::hours(5); // EST = UTC-5

        SensorReading {
            mesh_id: self.mesh_id.clone(),
            device_id: self.device_id.clone(),
            timestamp_utc: self.timestamp,
            timestamp_est,
            temperature_c: self.temperature_c,
            temperature_f,
            humidity: self.humidity,
            status: self.status.clone(),
            temperature_alert: self.temperature_c < -10.0 || self.temperature_c > 60.0,
            humidity_alert: self.humidity < 10.0 || self.humidity > 90.0,
        }
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;
    use chrono::{TimeZone, Timelike, Utc};

    fn create_test_raw_reading(temp_c: f32, humidity: f32) -> RawSensorReading {
        // ---
        RawSensorReading {
            mesh_id: "mesh-001".to_string(),
            device_id: "device-A".to_string(),
            timestamp: Utc.with_ymd_and_hms(2025, 3, 26, 18, 45, 0).unwrap(),
            temperature_c: temp_c,
            humidity,
            status: "ok".to_string(),
        }
    }

    #[test]
    fn test_temperature_conversion() {
        // ---
        let raw = create_test_raw_reading(22.4, 50.0);
        let transformed = raw.to_transformed();

        // 22.4°C should be 72.32°F
        assert_eq!(transformed.temperature_f, 72.32);
        assert_eq!(transformed.temperature_c, 22.4);
    }

    #[test]
    fn test_timezone_conversion() {
        // ---
        let raw = create_test_raw_reading(20.0, 50.0);
        let transformed = raw.to_transformed();

        // EST is UTC-5, so 18:45 UTC becomes 13:45 EST
        assert_eq!(transformed.timestamp_utc.hour(), 18);
        assert_eq!(transformed.timestamp_est.hour(), 13);
    }

    #[test]
    fn test_temperature_alerts() {
        // ---
        // Normal temperature - no alert
        let normal = create_test_raw_reading(25.0, 50.0);
        assert!(!normal.to_transformed().temperature_alert);

        // Too cold - should alert
        let cold = create_test_raw_reading(-15.0, 50.0);
        assert!(cold.to_transformed().temperature_alert);

        // Too hot - should alert
        let hot = create_test_raw_reading(65.0, 50.0);
        assert!(hot.to_transformed().temperature_alert);

        // Edge cases
        let edge_cold = create_test_raw_reading(-10.0, 50.0);
        assert!(!edge_cold.to_transformed().temperature_alert);

        let edge_hot = create_test_raw_reading(60.0, 50.0);
        assert!(!edge_hot.to_transformed().temperature_alert);
    }

    #[test]
    fn test_humidity_alerts() {
        // ---
        // Normal humidity - no alert
        let normal = create_test_raw_reading(25.0, 50.0);
        assert!(!normal.to_transformed().humidity_alert);

        // Too dry - should alert
        let dry = create_test_raw_reading(25.0, 5.0);
        assert!(dry.to_transformed().humidity_alert);

        // Too humid - should alert
        let humid = create_test_raw_reading(25.0, 95.0);
        assert!(humid.to_transformed().humidity_alert);

        // Edge cases
        let edge_dry = create_test_raw_reading(25.0, 10.0);
        assert!(!edge_dry.to_transformed().humidity_alert);

        let edge_humid = create_test_raw_reading(25.0, 90.0);
        assert!(!edge_humid.to_transformed().humidity_alert);
    }

    #[test]
    fn test_data_preservation() {
        // ---
        let raw = RawSensorReading {
            mesh_id: "mesh-test".to_string(),
            device_id: "device-test".to_string(),
            timestamp: Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap(),
            temperature_c: 20.0,
            humidity: 45.0,
            status: "warning".to_string(),
        };

        let transformed = raw.to_transformed();

        // Original data should be preserved
        assert_eq!(transformed.mesh_id, "mesh-test");
        assert_eq!(transformed.device_id, "device-test");
        assert_eq!(transformed.status, "warning");
        assert_eq!(transformed.temperature_c, 20.0);
        assert_eq!(transformed.humidity, 45.0);
    }
}
