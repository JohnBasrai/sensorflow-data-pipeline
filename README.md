# sensorflow-data-pipeline
Modular sensor data pipeline in Rust: fetch, transform, analyze, store, and expose via API

## Summary

`sensorflow-data-pipeline` is a modular, extensible data pipeline written in Rust for processing time-series sensor data. It fetches paginated data from a secured API, performs real-time transformations and anomaly detection, aggregates results by mesh ID, stores summarized data in PostgreSQL, and exposes a clean API for retrieval. Designed for testability, clarity, and production realism.

> This project is a portfolio-quality demo of an end-to-end data pipeline built in Rust for processing time-series sensor data. It securely fetches data from a paginated API, performs real-time transformation and anomaly detection, aggregates results, and serves them through a RESTful API. Designed for clarity, testability, and production realism.  This version removes all prior references to external companies; the project is now presented as an independent, portfolio-quality demo.

---

## 🔥 Features

- Secure ingestion from a paginated API
- UTC timestamp normalization (client-side timezone conversion)
- Anomaly detection (temperature, humidity, device status)
- Aggregation by `mesh_id`
- PostgreSQL-backed storage for summary data
- Strategic database indexing for sub-second query performance
- REST API to serve transformed sensor data
- Docker Compose for full environment setup

---

## 🚀 Quickstart

Clone the repo and start the system using Docker:

```bash
git clone https://github.com/JohnBasrai/sensorflow-data-pipeline.git
cd sensorflow-data-pipeline
cp .env.example .env
# make changes to localize if needed
docker compose up --build -d
# See Testing for how to run the tests.
```

**This will:**

* Start the sensor data source API
* Start PostgreSQL with optimized indexes for efficient filtering
* Run the Rust backend to ingest, process, store, and expose transformed sensor data

You can then query the transformed data via:

## API

### `GET /sql/readings`
Returns sensor readings from Postgres (ingest-once; subsequent calls are fast).

**Query params**
- `device_id` (aliases: `device`, `deviceId`, `deviceID`)
- `mesh_id`   (aliases: `mesh`, `meshId`, `meshID`)
- `timestamp_range` — RFC3339 `"start,end"`; open ends allowed (`"start,"`, `",end"`).  
  Returns **422** on invalid input.
- `limit` — max rows to return (default: 1000)

**Examples**

```console
export BASE=http://localhost:8080
# by device
$ curl "$BASE/sql/readings?device=device-001&limit=10"

# by mesh
$ curl "$BASE/sql/readings?mesh=mesh-001&limit=10"

# by timestamp range (inclusive)
$ curl "$BASE/sql/readings?timestamp_range=2025-03-21T00:00:00Z,2025-03-21T12:00:00Z"

# Invalid timestamp_range → 422 with JSON error
$ curl -i "$BASE/sql/readings?timestamp_range=not-a-timestamp"
HTTP/1.1 422 Unprocessable Entity
content-type: application/json
content-length: 119
date: Sat, 09 Aug 2025 00:03:53 GMT

{"error":"invalid timestamp_range","hint":"use RFC3339 \"start,end\" (e.g. 2025-03-21T00:00:00Z,2025-03-22T00:00:00Z)"}
```
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

   * Store temperature in `temperature_c` only
   * Store timestamps in UTC (timezone conversion handled in frontend)
   * Flag anomalies:

     * `temperature_alert` if `< -10°C` or `> 60°C`
     * `humidity_alert` if `< 10%` or `> 90%`
     * Optional: flag non-"ok" statuses

3. **Aggregate by `mesh_id`**

   * Compute average temperature, average humidity, and count of readings

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

## ✅ Design and Implementation Goals

* Code correctness and clarity
* Modular and testable architecture
* Docker setup and environment handling
* PostgreSQL schema design and data integrity
* Clean API design and filtering
* Git hygiene and documentation

---

## 🧭 Design decisions

### Timezone handling (UTC-only)
**Original requirement:** Store both UTC and EST.  
**Decision:** Store **UTC only**; convert on the client.

**Why:** avoids redundancy/DST bugs, follows industry practice, supports global users.

**Frontend example**
```javascript
const utc = sensor.timestamp_utc;               // e.g., "2025-03-21T00:00:00Z"
const local = new Date(utc).toLocaleString();   // renders in user's local timezone
```

### Temperature units (Celsius-only)

**Original requirement:** Store `temperature_c` **and** `temperature_f`  
**Decision:** Store/return **Celsius only**; clients convert to °F if needed.

**Why:** storing both is redundant and can drift (same smell as UTC+EST).

**Client conversion example**

```javascript
const c = sensor.temperature_c;
const f = c * 9/5 + 32;
```

> If consumers want server-side conversions without duplicating state, we can add
> presentation params (`?units=imperial`) or expose derived values via views/generated columns.

### Ingest-once fast path

`GET /sql/readings` ingests from upstream **only when the DB is empty**, then serves from Postgres.
Subsequent calls/tests are sub-second `(~0.11s)`.

### Validation & errors

* `timestamp_range` must be RFC3339 `"start,end"` (open ends allowed: `"start,"`, `",end"`).
* Invalid input returns **422** with JSON `{ "error", "hint" }`.

---

## ⚡ Performance

### Database Optimization
- Targeted SQL queries with database-level filtering
- Strategic indexing: single-column (`device_id`, `mesh_id`, `timestamp_utc`) and composite indexes
- PostgreSQL query planner automatically selects optimal indexes
- **Result**: ~0.11s response times for filtered queries

### Caching Strategy  
- Ingest-once pattern: data loaded on first request, cached in PostgreSQL
- Subsequent API calls serve directly from database without re-ingestion
- Memory-efficient: no in-memory filtering of large datasets

---

## Testing

1. **Start the full environment:**
```bash
export BASE=http://localhost:8080
docker compose up --build -d
```

2. **Run all tests:**
```bash
cargo test
    Finished `release` profile [optimized] target(s) in 0.22s
     Running unittests src/main.rs (target/release/deps/sensorflow-data-pipeline-ac928a43cce04ee7)

running 8 tests
test models::tests::data_preservation ... ok
test models::tests::humidity_alerts ... ok
test models::tests::temperature_alerts ... ok
test models::tests::utc_timestamp_preserved ... ok
test routes::get_readings::tests::parses_full_range_and_trims ... ok
test routes::get_readings::tests::parses_open_start ... ok
test routes::get_readings::tests::rejects_missing_comma ... ok
test routes::get_readings::tests::rejects_reversed_range ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/integration_test.rs (target/release/deps/integration_test-135dda86e65697c6)

running 3 tests
test filters_work_end_to_end ... ok
test timestamp_range_bad_returns_422 ... ok
test readings_endpoint_transforms_ok ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

# To run just integration tests (keeps unit output quiet)
cargo test --test integration_test -- --nocapture

# Tail backend logs during tests
docker compose logs -f sensor-api

# For versbose logging
RUST_LOG=info,sqlx::query=warn docker compose up -d --build
```

Integration tests run sequentially to avoid overlapping the initial ingest on `/sql/readings`. 
The first run ingests and caches data in Postgres taking about `10s`; subsequent runs are fast `(~0.11s)`.

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
