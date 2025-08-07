# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Complete sensor data pipeline implementation
- Data models for raw and transformed sensor readings
- Database schema with `sensor_data` and `mesh_summary` tables
- Paginated API data ingestion with cursor handling
- Real-time transformations: UTC→EST timezone conversion, Celsius→Fahrenheit
- Anomaly detection for temperature and humidity alerts
- Mesh-level aggregation with averages and reading counts
- REST API endpoint `/sql/readings` serving transformed data from PostgreSQL
- Comprehensive documentation and code formatting guidelines
- Docker Compose setup for full development environment

### Changed
- Refactored application structure using Explicit Module Boundary Pattern (EMBP)
- Enhanced configuration loading with detailed error handling
- Improved logging setup with configurable tracing levels
- Removed sqlx macros to eliminate compile-time database dependency

### Infrastructure
- PostgreSQL schema management with automatic table creation
- Database indexing for optimal query performance
- Structured error handling throughout the pipeline
- Production-ready Docker containerization

## [0.1.0] - Initial Setup

### Added
- Basic Rust project structure
- Core dependencies (Axum, SQLx, Tokio, Serde)
- Initial configuration management
- Docker Compose environment setup
- Basic database connectivity

