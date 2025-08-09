//! Simple data models for the sensor pipeline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---

/// Raw reading as returned by the upstream API (wire format).
///
/// - Mirrors the JSON payload 1:1; no normalization or computed fields.
/// - Use `to_transformed()` to produce a `SensorReading` suitable for storage:
///   - normalizes `timestamp` to UTC
///   - computes `temperature_f` from `temperature_c`
///   - flags anomalies: `temperature_alert` (< -10°C or > 60°C),
///     `humidity_alert` (< 10% or > 90%)
/// - `status` is preserved verbatim from upstream; consumers may treat non-"ok" as an alert.
#[derive(Debug, Deserialize)]
pub struct RawSensorReading {
    // ---
    /// Mesh the device belongs to (natural key from upstream)
    pub mesh_id: String,

    /// Device identifier within the mesh (natural key from upstream)
    pub device_id: String,

    /// Source timestamp (may include offset); not forced to UTC here.
    pub timestamp: DateTime<Utc>,

    /// Temperature reported by upstream, in °C
    pub temperature_c: f32,

    /// Relative humidity (%) reported by upstream
    pub humidity: f32,

    /// Upstream status string (e.g., "ok"); passed through unchanged
    pub status: String,
}

/// Normalized sensor reading used for storage and API responses.
///
/// Produced by `RawSensorReading::to_transformed()`. Invariants:
/// - `timestamp_utc`     is normalized to UTC (`timestamptz` when stored).
/// - `temperature_alert` is true if `temperature_c` < -10.0 **or** > 60.0 (strict).
/// - `humidity_alert`    is true if `humidity` < 10.0 **or** > 90.0 (strict).
/// - `status` is copied from upstream; not interpreted here.
/// Maps 1:1 to the `sensor_data` table and is safe to insert via `store_sensor_reading`.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SensorReading {
    // ---
    /// Natural key of the mesh (from upstream).
    pub mesh_id: String,

    /// Device identifier within the mesh (from upstream).
    pub device_id: String,

    /// Timestamp when the sensor captured this reading, normalized to UTC (not
    /// ingest time).
    pub timestamp_utc: chrono::DateTime<chrono::Utc>,

    /// Temperature in °C as reported/normalized.
    pub temperature_c: f32,

    /// Relative humidity in percent.
    pub humidity: f32,

    /// Upstream status string (e.g., "ok"); preserved verbatim.
    pub status: String,

    /// Temp anomaly flag: true if < -10°C or > 60°C.
    pub temperature_alert: bool,

    /// Humidity anomaly flag: true if < 10% or > 90%.
    pub humidity_alert: bool,
}

/// Simple transformation helpers
impl RawSensorReading {
    // ---
    pub fn to_transformed(&self) -> SensorReading {
        // ---

        SensorReading {
            mesh_id: self.mesh_id.clone(),
            device_id: self.device_id.clone(),
            timestamp_utc: self.timestamp, // Keep original UTC, UI will map it to local time
            temperature_c: self.temperature_c,
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
    use chrono::{TimeZone, Utc};

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
    fn utc_timestamp_preserved() {
        // ---
        let original_utc = Utc.with_ymd_and_hms(2025, 3, 26, 18, 45, 0).unwrap();
        let raw = RawSensorReading {
            mesh_id: "mesh-001".to_string(),
            device_id: "device-A".to_string(),
            timestamp: original_utc,
            temperature_c: 20.0,
            humidity: 50.0,
            status: "ok".to_string(),
        };

        let transformed = raw.to_transformed();

        // UTC timestamp should be preserved exactly
        assert_eq!(transformed.timestamp_utc, original_utc);
    }

    #[test]
    fn temperature_alerts() {
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
    fn humidity_alerts() {
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
    fn data_preservation() {
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
