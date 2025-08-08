# codemetal-sensorflow
Modular sensor data pipeline in Rust: fetch, transform, analyze, store, and expose via API

## Summary

`codemetal-sensorflow` is a modular, extensible data pipeline written in Rust for processing time-series sensor data. It fetches paginated data from a secured API, performs real-time transformations and anomaly detection, aggregates results by mesh ID, stores both raw and summarized data in PostgreSQL, and exposes a clean API for retrieval. Designed for testability, clarity, and production realism.

> This project is a take-home assignment for CodeMetal.ai. It implements a realistic, end-to-end data pipeline in Rust to fetch, transform, analyze, and serve time-series sensor data from a secure API.

---

## 🔥 Features

- Secure ingestion from a paginated API
- Time zone and temperature unit conversions
- Anomaly detection (temperature, humidity, device status)
- Aggregation by `mesh_id`
- PostgreSQL-backed storage for raw and summary data
- REST API to serve transformed sensor data
- Docker Compose for full environment setup

---

## 🚀 Quickstart

Clone the repo and start the system using Docker:

```bash
git clone https://github.com/JohnBasrai/codemetal-sensorflow.git
cd codemetal-sensorflow
docker-compose up --build -d
````

This will:

* Start the sensor data source API
* Start PostgreSQL
* Run the Rust backend to ingest, process, store, and expose transformed sensor data

You can then query the transformed data via:

```
GET /sql/readings
```

Supports optional filters like:

* `device_id`
* `mesh_id`
* `timestamp_range`

---

## 📡 Input Dataset

Sensor data is served via a paginated API running in Docker:

```
GET /sensor-data
```

Each response includes up to 100 records and a `next_cursor`. Continue paging until `next_cursor` is absent.

Example record:

```json
{
  "mesh_id": "mesh-001",
  "device_id": "device-A",
  "timestamp": "2025-03-26T13:45:00Z",
  "temperature_c": 22.4,
  "humidity": 41.2,
  "status": "ok"
}
```

---

## 🧪 Processing Pipeline

1. **Fetch & Store Raw Data**

   * Ingest paginated API data and store all records in PostgreSQL with a normalized schema

2. **Transform**

   * Add `temperature_f`
   * Store timestamps in UTC (timezone conversion handled in frontend)
   * Flag anomalies:

     * `temperature_alert` if `< -10°C` or `> 60°C`
     * `humidity_alert` if `< 10%` or `> 90%`
     * Optional: flag non-"ok" statuses

3. **Aggregate by `mesh_id`**

   * Compute average temperature (C/F), average humidity, and count of readings

4. **Store Summary**

   * Serve transformed and aggregated records via `GET /sql/readings`

5. **Expose Data API**

   * Serve transformed records via `GET /sql/readings`

📁 Project Structure

- `src/` — Rust backend source code
- `api/` — Mock data API (Python + FastAPI)
- `docker-compose.yml` — Full local test environment
- `.cargo/audit.toml` — Advisory exceptions for secure builds

---

## 🧱 Tech Stack

* 🦀 Rust
* 🐘 PostgreSQL
* 🐳 Docker Compose
* 📊 JSON API ingestion
* 🔐 Secure API access (simulated)
* 🚀 Axum web framework (Rust)

---

## ✅ Evaluation Criteria (from CodeMetal.ai)

* Code correctness and clarity
* Modular and testable architecture
* Docker setup and environment handling
* PostgreSQL schema design and data integrity
* Clean API design and filtering
* Git hygiene and documentation

---

## 🏗️ Architecture Decisions

### Timezone Handling

**Original Requirement**: Convert timestamps from UTC to Eastern Standard Time (EST) and store both.

**Implementation Decision**: Store only UTC timestamps and handle timezone conversion in the frontend.

**Rationale**: 
- **Scalability**: Hardcoding EST limits the system to Eastern timezone users
- **Best Practice**: Industry standard is UTC storage with client-side conversion
- **Maintainability**: Avoids complex DST handling and timezone edge cases in the backend
- **Future-Proof**: Supports global users without backend changes

**Frontend Integration**:
```javascript
// Client-side timezone conversion example
const utcTimestamp = sensorData.timestamp_utc;
const localTime = new Date(utcTimestamp).toLocaleString();
// Automatically shows in user's local timezone
```

---

## Testing

See [README-testing.md](README-testing.md)

## ⚠️ Security Advisory Note

> **`cargo audit` reports a medium-severity advisory (RUSTSEC-2023-0071) affecting the `rsa` crate.**
> * This crate is not used by this project directly. It is pulled in **indirectly via `sqlx-mysql`**, which is a transitive dependency of `sqlx-macros-core` — even though only the Postgres backend is enabled in `Cargo.toml`.
> * At this time, SQLx 0.8.x compiles all database backends unconditionally within its macros crate. This behavior is [tracked upstream](https://github.com/launchbadge/sqlx/issues/2487).
> * ✅ **This application does not link to MySQL, does not use the `rsa` crate at runtime, and is not affected by the vulnerability.**
> * The advisory is explicitly ignored via `.cargo/audit.toml` with rationale documented here.

---


## 📄 License

MIT License. See [LICENSE](./LICENSE) for details.

---

## ✍️ Author

John Basrai
[john@basrai.dev](mailto:john@basrai.dev)
