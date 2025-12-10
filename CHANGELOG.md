# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Data Aggregation System**: Multi-tier automatic data aggregation using ClickHouse materialized views
  - **Metrics**: 1-minute, 5-minute, 1-hour, and 1-day aggregates with count, sum, min, max, avg
  - **Logs**: Hourly and daily count aggregations by level, service, and normalized message pattern
  - **Traces/Spans**: Hourly and daily performance statistics including:
    - Latency percentiles (P50, P95, P99) for performance analysis
    - Span counts and throughput trends by service/operation
    - Error rate tracking via status_code grouping
    - Trace characteristics (unique traces, spans per trace)
  - Automatic population via materialized views (no background jobs needed)
  - 90%+ storage reduction for historical data
  - Each aggregation tier has configurable retention (30 days to 2 years)
- **Log Message Normalization**: Intelligent pattern extraction for log aggregation
  - `normalizeLogMessage()` function strips variable parts (timestamps, UUIDs, IPs, numbers, URLs, emails, paths)
  - Groups similar log messages together (e.g., "Error at 10:15" and "Error at 11:30" → same pattern)
  - Enables trend analysis across message patterns rather than unique messages
  - Automatically applied via `MATERIALIZED` column in logs table
  - Aggregation tables include `sample_message` for reference to original format
- **Aggregation Configuration**: Added `AggregationConfig` in `shared/config` module
- **Aggregation API**: `GET /api/v1/config/aggregation` endpoint for viewing aggregation policies
- **Automatic ClickHouse TTL Updates**: Retention policy changes via API now automatically execute `ALTER TABLE` statements to update ClickHouse TTL
- **Data Age Timestamps**: Added `get_oldest_timestamp()` and `get_newest_timestamp()` methods to all store traits (LogStore, MetricStore, TraceStore)
- **Enhanced Data Age Metrics**: `/api/v1/config/retention/metrics` endpoint now returns actual oldest/newest timestamps (returns null when count is zero)
- **ClickHouse Client Tracking**: `AppState` now tracks ClickHouse client availability for automatic TTL updates
- Initial project scaffolding with Rust workspace structure
- `api` crate: Axum-based API server with health check and log ingestion
  - `GET /health` endpoint returning service status, name, and version
  - `POST /api/v1/logs` endpoint for log ingestion (single or batch)
  - `GET /api/v1/logs` endpoint for querying logs with comprehensive filtering:
    - Time range filtering (`start_time`, `end_time`)
    - Log level filtering (`level`)
    - Service name filtering (`service`, exact match)
    - Message search (`contains`, case-insensitive substring match)
    - Pagination support (`limit`, `offset`)
  - Request validation with detailed error responses
  - Request body size limit (10 MB)
  - Configuration via environment variables (`HEIMSIGHT_HOST`, `HEIMSIGHT_PORT`)
  - Graceful shutdown handling (SIGTERM/SIGINT)
  - Request tracing with `tower-http`
  - Application state management with shared storage
- `shared` crate: Common types and utilities
  - `LogEntry` struct for log data with timestamp, level, message, service, attributes
  - `LogLevel` enum (trace, debug, info, warn, error, fatal)
  - Trace correlation fields (trace_id, span_id) for distributed tracing
  - Validation for required fields (non-empty message and service)
  - Builder pattern with `with_attribute()`, `with_trace_id()`, `with_span_id()`
  - `LogStore` trait for abstracting log storage operations
  - `InMemoryLogStore` implementation with thread-safe Vec + RwLock
  - `LogQuery` builder for filtering logs with multiple criteria:
    - Time range (`with_start_time`, `with_end_time`)
    - Level (`with_level`)
    - Service (`with_service`)
    - Message content (`with_message_contains`)
    - Pagination (`with_limit`, `with_offset`)
  - `LogQueryResult` containing logs and total count for pagination support
- `cli` crate: Command-line interface with basic structure
- Project documentation: README.md, CHANGELOG.md, PROJECT.md
- Development instructions: CLAUDE_INSTRUCTIONS.md
- `examples/health.http` and `examples/logs.http` for manual API testing
- `Makefile` with common development commands

### Phase 2: Core Features

- **SQL-like Query Parser and Executor**
  - `POST /api/v1/query` endpoint for SQL-like queries
  - Supports: `SELECT * FROM logs WHERE level = 'error' AND service = 'api'`
  - WHERE clauses with AND, OR, parentheses for grouping
  - Comparison operators: =, !=, <, >, <=, >=, CONTAINS, STARTS WITH, ENDS WITH
  - ORDER BY with ASC/DESC
  - LIMIT and OFFSET for pagination
  - Query AST returned in response for transparency

- **Metrics Data Model and API**
  - `Metric` struct with name, type, value, timestamp, labels
  - `MetricType` enum: Counter, Gauge, Histogram
  - `MetricValue` supporting simple values and histogram data
  - `MetricStore` trait and `InMemoryMetricStore` implementation
  - `POST /api/v1/metrics` endpoint for metric ingestion (single or batch)
  - `GET /api/v1/metrics` endpoint with filters (name, type, labels)
  - Aggregation support: sum, avg, min, max, count

- **Trace Data Model and API**
  - `Span` struct with trace_id, span_id, parent_span_id, name, service, timestamps, attributes
  - `SpanKind` enum: Internal, Server, Client, Producer, Consumer
  - `SpanStatus` enum: Ok, Error, Cancelled
  - `Trace` struct for grouping spans by trace_id
  - `TraceStore` trait and `InMemoryTraceStore` implementation
  - `POST /api/v1/traces` endpoint for span ingestion (single or batch)
  - `GET /api/v1/traces` endpoint with filters (service, duration, status)
  - `GET /api/v1/traces/{trace_id}` endpoint to retrieve a single trace

### Phase 3: OTLP & Storage

- **OTLP Protobuf Definitions**
  - Added OpenTelemetry Protocol (OTLP) protobuf definitions
  - Integrated `prost` and `tonic` for protobuf code generation
  - Generated Rust types for OTLP logs, metrics, traces, and collector services
  - Conversion functions from OTLP types to internal Heimsight types:
    - `otlp_log_to_log_entry`: Converts OTLP LogRecord to LogEntry
    - `otlp_span_to_span`: Converts OTLP Span to Heimsight Span
    - `otlp_metrics_to_metrics`: Converts OTLP Metric to Heimsight Metric (supports Gauge, Counter, Histogram)
  - Comprehensive test coverage for all conversion functions
  - Proper handling of resource attributes, trace context, and metric labels
  - Updated Makefile test target to skip doctests in generated protobuf code

- **OTLP HTTP Receiver**
  - `POST /v1/logs` endpoint for OTLP log ingestion
  - `POST /v1/metrics` endpoint for OTLP metric ingestion
  - `POST /v1/traces` endpoint for OTLP trace ingestion
  - Support for both protobuf (`application/x-protobuf`) and JSON (`application/json`) content types
  - Integrated `pbjson` for proper protobuf-JSON encoding (camelCase fields, string timestamps)
  - Automatic conversion from OTLP format to internal Heimsight types
  - Partial success responses indicating rejected items
  - Added example files: `examples/otlp_logs.http`, `examples/otlp_metrics.http`, `examples/otlp_traces.http`
  - Standard OpenTelemetry SDK exporters can now send data directly to Heimsight

- **OTLP gRPC Receiver**
  - Full gRPC server implementation running alongside HTTP server
  - gRPC server listens on port 4317 (configurable via `HEIMSIGHT_GRPC_PORT`)
  - HTTP server listens on port 8080 (configurable via `HEIMSIGHT_PORT`)
  - Implements three OTLP gRPC collector services:
    - `opentelemetry.proto.collector.logs.v1.LogsService`
    - `opentelemetry.proto.collector.metrics.v1.MetricsService`
    - `opentelemetry.proto.collector.trace.v1.TraceService`
  - Both servers run concurrently using `tokio::try_join!` for efficient resource utilization
  - Supports partial success responses for batches with invalid data
  - Automatic conversion from OTLP protobuf format to internal Heimsight types
  - Thread-safe shared storage between HTTP and gRPC endpoints
  - Graceful shutdown handling for both servers
  - Comprehensive test coverage:
    - 10 unit tests for gRPC service implementations
    - 4 integration tests for end-to-end gRPC flow
  - Compatible with OpenTelemetry SDK gRPC exporters

- **Database Selection & Setup (ClickHouse)**
  - Evaluated four database options: ClickHouse, TimescaleDB, MongoDB, ScyllaDB
  - Selected ClickHouse for production-grade observability workloads
    - Purpose-built for analytical queries on time-series data
    - Superior compression (10-100x typical)
    - Native SQL support with analytics extensions
    - Built-in materialized views for continuous aggregation
    - Proven at scale by major observability platforms (SigNoz, Grafana Tempo)
  - Docker Compose setup for local development
    - ClickHouse server 24.11 with health checks
    - Automatic schema initialization on first startup
    - Persistent volume for data storage
  - Database schema with three main tables:
    - `logs`: Time-series log storage with TTL (30 days), full-text indexes, Map for attributes
    - `metrics`: Time-series metrics (counter, gauge, histogram) with TTL (90 days), support for all metric types
    - `spans`: Distributed trace spans with TTL (30 days), support for events and links
  - Materialized views for automatic aggregation:
    - `logs_hourly_stats`: Hourly log counts by service and level (90 day retention)
    - `metrics_5min_agg`: 5-minute metric aggregations (180 day retention)
    - `metrics_hourly_agg`: Hourly metric aggregations (365 day retention)
    - `trace_stats`: Per-trace summary statistics (30 day retention)
    - `span_performance_hourly`: Hourly service performance metrics (90 day retention)
  - Database connection module (`api/src/db.rs`):
    - `DatabaseConfig` for loading configuration from environment variables
    - `Database` wrapper with connection pooling using Arc<Client>
    - `ping()` method for health checking database connectivity
    - Comprehensive test coverage (4 tests)
  - Added `clickhouse` crate (v0.12) as dependency
  - Environment variables for database configuration:
    - `HEIMSIGHT_DB_URL`: ClickHouse URL (default: `http://localhost:8123`)
    - `HEIMSIGHT_DB_NAME`: Database name (default: `heimsight`)
    - `HEIMSIGHT_DB_USER`: Database user (default: `heimsight`)
    - `HEIMSIGHT_DB_PASSWORD`: Database password (default: `heimsight_dev`)
  - Documentation:
    - `DATABASE_EVALUATION.md`: Comprehensive comparison of all evaluated databases
    - `schema/README.md`: Schema documentation with design decisions and query examples
    - Updated README.md with database setup instructions
  - All 262 tests passing (including new database connectivity test)

- **Retention Configuration & TTL Management**
  - Runtime retention configuration system in `shared/config` module:
    - `RetentionConfig` struct for managing TTL policies per data type (logs, metrics, traces)
    - `RetentionPolicy` struct with validation (1-3650 days)
    - `DataType` enum for type-safe policy management
    - Default retention: logs (30 days), metrics (90 days), traces (30 days)
  - API endpoints for retention management:
    - `GET /api/v1/config/retention` - View current retention configuration
    - `PUT /api/v1/config/retention` - Update complete retention configuration
    - `PUT /api/v1/config/retention/policy` - Update a single data type's policy
    - `GET /api/v1/config/retention/metrics` - Get data age metrics
  - Data age monitoring and metrics:
    - `DataAgeMonitor` background job running hourly
    - `DataAgeMetrics` tracking oldest/newest data and counts per data type
    - Automatic warnings when data age exceeds configured TTL
    - Statistics exposed via metrics endpoint for observability
  - Thread-safe retention configuration in `AppState` using `Arc<RwLock<RetentionConfig>>`
  - Comprehensive test coverage:
    - 14 unit tests for retention configuration structures
    - 7 integration tests for retention API endpoints
    - 12 unit tests for data age monitoring
  - Enhanced schema documentation:
    - Documented relationship between schema-level TTL and runtime configuration
    - Added SQL examples for updating ClickHouse table TTL values
    - Explained automatic TTL enforcement by ClickHouse
  - All 293 tests passing (31 new tests added for retention features)

### Changed

- **Dependency Upgrades**
  - Upgraded Rust toolchain from 1.86.0 to 1.91.1
  - Upgraded axum from 0.8 to 0.8.7
  - Upgraded tower from 0.5 to 0.5.2
  - Upgraded tower-http from 0.6 to 0.6.6
  - Upgraded tonic from 0.12 to 0.14 (breaking change - requires `tonic-prost` crate)
  - Upgraded prost from 0.13 to 0.14
  - Upgraded base64 from 0.22 to 0.22.1
  - Upgraded pbjson from 0.7 to 0.8
  - Upgraded pbjson-types from 0.7 to 0.8
  - Upgraded clickhouse from 0.12 to 0.14.1
  - Upgraded tracing-subscriber from 0.3 to 0.3.20
  - Upgraded dotenvy from 0.15 to 0.15.7
  - Upgraded tokio-test from 0.4 to 0.4.4
  - Upgraded http-body-util from 0.1 to 0.1.3
  - Upgraded hex from 0.4 to 0.4.3
  - Upgraded urlencoding from 2.1 to 2.1.3
  - Added tonic-prost 0.14 runtime dependency (required for tonic 0.14)
  - Updated build dependencies to tonic-prost-build 0.14 (replaces tonic-build)
  - Kept nom at 7.1 (nom 8.0 has breaking API changes requiring extensive refactoring)
  - Updated Default trait implementations to use derive macros (clippy::derivable_impls)
