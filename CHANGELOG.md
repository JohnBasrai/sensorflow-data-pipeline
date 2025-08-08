# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- (placeholder)

### Changed
- (placeholder)

### Fixed
- (placeholder)

---

## [0.2.0] - 2025-08-08

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

[Unreleased]: https://github.com/JohnBasrai/codemetal-sensorflow/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/JohnBasrai/codemetal-sensorflow/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/JohnBasrai/codemetal-sensorflow/releases/tag/v0.1.0
