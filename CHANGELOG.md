# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-09-10

### Changed
- Revised README to reflect independent, portfolio-driven purpose
- Updated Cargo.toml metadata (`name`, `description`, `authors` unchanged; version bumped to 0.4.0)

### Documentation
 - Update design and evaluation goals

### Added
- (placeholder)

### Changed
- (placeholder)

### Fixed
- (placeholder)

---

## [v0.3.2] - 2025-08-09

### Documentation
- Add .env setup step to README installation instructions
- Add reference to Testing section in quick start guide

---

## [0.3.1] - 2025-08-09

### Performance
- Move filtering from memory to database queries (~0.11s response time)
- Add timestamp and composite indexes for better query performance

### Added
- Version logging at startup
- Dynamic SQL query building for filtered requests

### Removed
- In-memory filtering after database load

### Changed
- Replace `load_all_readings` with `load_filtered_readings`
- Improved logging config to reduce database noise

---

## [0.3.0] - 2025-01-08

### BREAKING CHANGES
- **API Response Format**: Removed `temperature_f` field from `/sql/readings` endpoint
- Clients should now convert temperature client-side: `const f = c * 9/5 + 32`

### Changed
- Store only Celsius temperatures (UTC-only approach)
- Improved database schema efficiency
- Updated documentation with client-side conversion examples

### Removed
- Removed `temperature_f` column from `sensor_data` table
- Removed temperature conversion logic from backend
- Updated integration tests to match new response format

### Tests
- Updated integration tests to match new response format

---

## [0.2.0] - 2025-08-08

### Performance
- Ingest-once cache + DB fast path for `/sql/readings`; subsequent requests and test runs are sub-second.

### Tests
- Filter integration tests run sequentially to avoid overlapping the initial ingest.
- Integration test asserts **422** for invalid `timestamp_range`.
- Unit tests added for `parse_timestamp_range`.

### BREAKING
- Remove `timestamp_est` from DB and API; **store UTC-only timestamps**. Clients must
  convert to local time on the frontend. (See README “Timezone Handling”.)

### Added
- Query filters for `device_id`, `mesh_id`, and `timestamp_range` on `/sql/readings`.

### Fixed
- Accept common query parameter aliases:
  - `device_id` ← `device`, `deviceId`, `deviceID`
  - `mesh_id`   ← `mesh`, `meshId`, `meshID`
  - `timestamp_range` ← `ts_range`, `timestampRange`

### Changed
- Logging defaults for local dev: `RUST_LOG=warn`, `AXUM_LOG_LEVEL=debug`,
  `AXUM_SPAN_EVENTS=enter_exit`, `FORCE_COLOR=0`.

### Infrastructure
- Idempotent schema creation on startup; mesh summary upsert; indexes for `mesh_id`
  and `device_id`.

## [0.1.0] - 2025-08-07

### Added
- Complete sensor data pipeline implementation
- Data models for raw and transformed sensor readings
- Database schema with `sensor_data` and `mesh_summary` tables
- `/sql/readings` endpoint for query and retrieval
- Comprehensive documentation and code formatting guidelines
- Docker Compose setup for full development environment

### Changed
- Refactored application structure using Explicit Module Boundary Pattern (EMBP)
- Enhanced configuration loading with detailed error handling
- Removed `sqlx` macros to eliminate compile-time database dependency

### Infrastructure
- PostgreSQL schema management with automatic table creation
- Database indexing for optimal query performance
- Structured error handling throughout the pipeline

---

[Unreleased]: https://github.com/JohnBasrai/sensorflow-data-pipeline/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/JohnBasrai/sensorflow-data-pipeline/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/JohnBasrai/sensorflow-data-pipeline/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/JohnBasrai/sensorflow-data-pipeline/releases/tag/v0.1.0
