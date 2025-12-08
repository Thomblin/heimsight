# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
