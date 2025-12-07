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
