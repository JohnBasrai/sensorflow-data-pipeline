## 🧪 Testing

The project includes a comprehensive integration test that validates all pipeline transformations against live data.

### Running the Integration Test

1. **Start the full environment:**
   ```bash
   docker-compose up --build -d
   ```

2. **Run the integration test:**
   ```bash
   cargo test test_sensor_pipeline_integration -- --nocapture
   ```

### What the Test Validates

The integration test verifies all 4 core requirements from the original problem specification:

- ✅ **API Connectivity**: Confirms `/sql/readings` returns data from PostgreSQL
- ✅ **Temperature Conversion**: Validates `(temp_c * 9/5) + 32 = temp_f` 
- ✅ **Timezone Conversion**: Ensures EST = UTC - 5 hours
- ✅ **Anomaly Detection**: Checks temperature and humidity alert flags

### Expected Test Output

When successful, you should see output similar to:

```
🔍 Testing Sensor Data Pipeline Integration
==========================================
✅ API responds with 500 records from PostgreSQL

📋 Testing record 1 (mesh: mesh-003, device: device-001)
  ✅ Temperature: -13.6°C → 7.5200005°F
  ✅ Timezone: UTC 21:22 → EST 16:22
  ✅ Temp Alert: true (-13.6°C)
  ✅ Humidity Alert: false (43.5%)

📋 Testing record 2 (mesh: mesh-001, device: device-004)
  ✅ Temperature: 36°C → 96.8°F
  ✅ Timezone: UTC 18:33 → EST 13:33
  ✅ Temp Alert: false (36°C)
  ✅ Humidity Alert: false (40.1%)

[... more test records ...]

🎉 All required transformations working correctly!
test test_sensor_pipeline_integration ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Test Design

This minimal integration test:
- Tests only the first 5 sample records for speed and clarity
- Validates the 4 transformations explicitly required in the original problem
- Shows clear pass/fail status for each transformation type
- Exits with proper error codes for CI/CD integration
- Runs against live data from the complete pipeline (API → PostgreSQL → REST API)

### Manual Testing

You can also test the pipeline manually:

```bash
# Check the raw sensor data API
curl http://localhost:8081/sensor-data

# Trigger the pipeline and view transformed data
curl http://localhost:8080/sql/readings | jq length
```

The `/sql/readings` endpoint should return approximately 500 transformed sensor records.