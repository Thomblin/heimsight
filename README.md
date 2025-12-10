# Heimsight

A self-hosted, full-stack observability platform built in Rust.

**Heimsight** is a self-hosted, full-stack observability platform built in Rust. It provides unified logs, traces, metrics, and alerting with a focus on simplicity, performance, and horizontal scalability. The name combines "Heim" (from Heimdall, the Norse guardian) with "sight," representing clear visibility into your systems.

## Current Status

**🚧 Active Development - Core Features Complete**

Heimsight is currently in active development with core observability features fully implemented and production-ready. The platform includes complete OTLP support, persistent storage, and dynamic retention management.

### ✅ Implemented (Production-Ready)

- Full OpenTelemetry Protocol (OTLP) support (gRPC + HTTP)
- Logs ingestion, storage, and querying with full-text search
- Metrics ingestion, storage, and querying (counter, gauge, histogram)
- Distributed traces ingestion, storage, and querying
- SQL-like query language for exploring data
- ClickHouse persistent storage with automatic TTL
- Dynamic retention policy management with automatic database updates
- Multi-tier data aggregation for long-term storage efficiency
- Data age monitoring and metrics
- REST API with comprehensive filtering and pagination
- Basic CLI for health checks

### 🚧 In Development

- Web UI dashboard
- Alerting engine and notification channels
- Grafana datasource plugin
- Data aggregation for long-term storage
- Advanced CLI commands
- Anomaly detection

## Features

- **OTLP-Native** - Full OpenTelemetry Protocol compliance for logs, metrics, and traces (gRPC + HTTP)
- **SQL-Like Queries** - Familiar query syntax for exploring your data
- **ClickHouse Storage** - High-performance columnar database for time-series data
- **Dynamic Retention** - Update TTL policies via API with automatic database updates
- **Persistent Storage** - All data persists in ClickHouse with automatic TTL enforcement
- **REST API** - Comprehensive API for all operations with filtering and pagination

## Project Structure

```
heimsight/
├── api/                 # Axum API server
│   ├── src/
│   └── Cargo.toml
├── cli/                 # CLI tools (heimsight binary)
│   ├── src/
│   └── Cargo.toml
├── shared/              # Shared libraries (models, utilities)
│   ├── src/
│   └── Cargo.toml
├── Cargo.toml           # Workspace definition
├── README.md
├── CHANGELOG.md
├── PROJECT.md           # Detailed project specification
└── CLAUDE_INSTRUCTIONS.md
```

## Getting Started

### Prerequisites

- Rust (stable, latest version recommended)
- Docker (for database, optional for development)

### Building

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run with clippy linting
cargo clippy -- -D warnings

# Format code
cargo fmt
```

### Setting Up the Database

Heimsight uses ClickHouse for persistent storage of logs, metrics, and traces.

```bash
# Start ClickHouse in Docker
docker compose up -d clickhouse

# Verify it's running
docker ps | grep clickhouse

# Apply database schema (includes message normalization function and aggregation tables)
make db-schema

# Check logs
docker logs heimsight-clickhouse

# Stop the database
docker compose down
```

The database schema is automatically initialized on first startup. Tables include:
- `logs` - Log entries with full-text search
- `metrics` - Metrics with multiple types (counter, gauge, histogram)
- `spans` - Distributed trace spans
- Materialized views for automatic aggregation

### Running the API Server

```bash
# Start the database first
docker compose up -d clickhouse

# Run API server in development mode
cargo run -p api

# With custom log level
RUST_LOG=debug cargo run -p api
```

### Using the CLI

```bash
# Build and run CLI
cargo run -p heimsight -- --help

# Check server health
cargo run -p heimsight -- health
```

## Configuration

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| **API Server** | | |
| `HEIMSIGHT_HOST` | HTTP server bind address | `0.0.0.0` |
| `HEIMSIGHT_PORT` | HTTP server port | `8080` |
| `HEIMSIGHT_GRPC_PORT` | gRPC server port | `4317` |
| `RUST_LOG` | Log level filter | `info` |
| **Database** | | |
| `HEIMSIGHT_DB_URL` | ClickHouse URL | `http://localhost:8123` |
| `HEIMSIGHT_DB_NAME` | Database name | `heimsight` |
| `HEIMSIGHT_DB_USER` | Database user | `heimsight` |
| `HEIMSIGHT_DB_PASSWORD` | Database password | `heimsight_dev` |
| **CLI** | | |
| `HEIMSIGHT_API_URL` | API server URL | `http://localhost:8080` |

## API Endpoints

### Core Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check |

### Logs

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/logs` | Ingest logs (single or batch) |
| `GET` | `/api/v1/logs` | Query logs with filters |

### Metrics

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/metrics` | Ingest metrics (single or batch) |
| `GET` | `/api/v1/metrics` | Query metrics with filters |

### Traces

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/traces` | Ingest spans (single or batch) |
| `GET` | `/api/v1/traces` | Query traces with filters |
| `GET` | `/api/v1/traces/{trace_id}` | Get a single trace by ID |

### Query

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/query` | Execute SQL-like queries |

### Retention Configuration

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/config/retention` | Get current retention configuration |
| `PUT` | `/api/v1/config/retention` | Update complete retention configuration |
| `PUT` | `/api/v1/config/retention/policy` | Update a single retention policy |
| `GET` | `/api/v1/config/retention/metrics` | Get data age metrics |

### OTLP (OpenTelemetry Protocol)

#### HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/logs` | OTLP HTTP logs ingestion |
| `POST` | `/v1/metrics` | OTLP HTTP metrics ingestion |
| `POST` | `/v1/traces` | OTLP HTTP traces ingestion |

#### gRPC Endpoints (Port 4317)

| Service | Method | Description |
|---------|--------|-------------|
| `LogsService` | `Export` | OTLP gRPC logs ingestion |
| `MetricsService` | `Export` | OTLP gRPC metrics ingestion |
| `TraceService` | `Export` | OTLP gRPC traces ingestion |

## Example Requests

HTTP request examples are provided in the `examples/` directory for manual testing with REST Client extensions:

- `examples/health.http` - Health check endpoint
- `examples/logs.http` - Log ingestion and querying
- `examples/metrics.http` - Metric ingestion and querying
- `examples/traces.http` - Trace ingestion and querying
- `examples/query.http` - SQL-like queries
- `examples/config_retention.http` - Retention configuration management
- `examples/aggregation.http` - Aggregation configuration and queries
- `examples/otlp_logs.http` - OTLP log ingestion
- `examples/otlp_metrics.http` - OTLP metric ingestion
- `examples/otlp_traces.http` - OTLP trace ingestion

These files work with the [REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client) extension for VS Code or IntelliJ's HTTP Client.

## Data Retention & TTL Management

Heimsight provides dynamic retention policy management with automatic TTL updates in ClickHouse:

### Features

- **Dynamic Configuration**: Update retention policies via API without restarting the server
- **Automatic TTL Updates**: ClickHouse table TTLs are automatically updated when policies change
- **Data Age Monitoring**: Track oldest/newest data timestamps for each data type
- **Validation**: Policies are validated (1-3650 days) before applying

### API Usage

```bash
# Get current retention configuration
GET /api/v1/config/retention

# Update all retention policies (automatically updates ClickHouse TTL)
PUT /api/v1/config/retention
{
  "logs": { "data_type": "logs", "ttl_days": 60 },
  "metrics": { "data_type": "metrics", "ttl_days": 180 },
  "traces": { "data_type": "traces", "ttl_days": 45 }
}

# Update a single retention policy (automatically updates ClickHouse TTL)
PUT /api/v1/config/retention/policy
{
  "data_type": "metrics",
  "ttl_days": 180
}

# Get data age metrics (oldest/newest timestamps)
GET /api/v1/config/retention/metrics
```

### Default Retention Periods

- **Logs**: 30 days
- **Metrics**: 90 days
- **Traces**: 30 days

### How It Works

1. API validates the new retention policy (1-3650 days)
2. Executes `ALTER TABLE` in ClickHouse to update TTL
3. Updates runtime configuration
4. Background monitor tracks data age and warns if TTL is exceeded

See `examples/config_retention.http` for more examples.

## Data Aggregation for Long-Term Storage

Heimsight automatically aggregates data using ClickHouse materialized views, providing efficient long-term storage:

### Features

- **Automatic Downsampling**: Metrics are automatically aggregated at multiple time intervals
- **Multi-Tier Storage**: Raw data → 1-minute → 5-minute → 1-hour → 1-day aggregates
- **Storage Efficiency**: Reduces storage by 90%+ for historical data
- **No Background Jobs**: ClickHouse materialized views handle aggregation in real-time
- **Flexible Querying**: Query any aggregation level using SQL-like queries

### Aggregation Tiers

#### Metrics
- **Raw data**: 90 days retention (full resolution)
- **1-minute aggregates**: 30 days (count, sum, min, max, avg)
- **5-minute aggregates**: 90 days
- **1-hour aggregates**: 365 days
- **1-day aggregates**: 730 days (2 years)

#### Logs
- **Raw logs**: 30 days retention (full text search)
- **Hourly counts**: 365 days (by level, service, and message pattern)
- **Daily counts**: 730 days (by level, service, and message pattern)

**Message Normalization**: Log messages are automatically normalized to group similar patterns:
```
"Error at 2024-12-09T10:15:23Z: Connection to 192.168.1.1 failed (id: 12345)"
"Error at 2024-12-09T11:30:45Z: Connection to 192.168.1.2 failed (id: 67890)"
→ Both become: "Error at <TIMESTAMP>: Connection to <IP> failed (id: <NUM>)"
```

This enables pattern-based analysis: "How many times did this type of error occur?" instead of counting each unique message.

#### Traces/Spans
- **Raw spans**: 30 days retention (full trace details)
- **Hourly span statistics**: 365 days (latency percentiles, throughput, error rates)
- **Daily span statistics**: 730 days (p50/p95/p99 latencies, span counts)
- **Hourly trace statistics**: 365 days (unique traces, total spans)
- **Daily trace statistics**: 730 days (unique traces, total spans)

**Performance Insights**: Span aggregations provide:
- **Latency analysis**: P50, P95, P99 duration percentiles by service/operation
- **Error rates**: Track failures over time
- **Throughput trends**: Requests per hour/day by service
- **Trace complexity**: Calculate average spans per trace (`total_spans / unique_traces`)

### Querying Aggregated Data

```bash
# Query aggregated metrics
POST /api/v1/query
{
  "query": "SELECT timestamp, avg FROM metrics_1hour WHERE name = 'cpu_usage' LIMIT 100"
}

# Query log volume trends
POST /api/v1/query
{
  "query": "SELECT * FROM logs_1day_counts WHERE level = 'error' LIMIT 30"
}

# Query span latency percentiles
POST /api/v1/query
{
  "query": "SELECT timestamp, service, operation, p95_duration_ns / 1000000 as p95_ms FROM spans_1hour_stats LIMIT 100"
}

# Find slowest operations
POST /api/v1/query
{
  "query": "SELECT service, operation, avg(p99_duration_ns) / 1000000 as p99_ms FROM spans_1day_stats GROUP BY service, operation ORDER BY p99_ms DESC LIMIT 10"
}
```

### Benefits

1. **Cost Savings**: Dramatically reduced storage for historical data
2. **Fast Queries**: Pre-aggregated data speeds up long-range queries
3. **Automatic**: No manual intervention required
4. **Flexible**: Query raw data for recent analysis, aggregates for trends

See `examples/aggregation.http` for more examples and schema/04_aggregations.sql for implementation details.

## Architecture

Heimsight uses a modular architecture:

- **API Server** (`api/`) - Axum-based REST API and gRPC server
  - HTTP endpoints for logs, metrics, traces, and queries
  - OTLP gRPC services for OpenTelemetry compatibility
  - Retention policy management
  - Data age monitoring
  
- **Shared Library** (`shared/`) - Common types and utilities
  - Data models for logs, metrics, and traces
  - Storage trait abstractions (in-memory and ClickHouse)
  - OTLP protocol conversion
  - SQL-like query parser and executor
  - Retention configuration
  
- **CLI** (`cli/`) - Command-line interface
  - Health check command
  - Future: query, ingestion, and management commands

## Development

This project follows Test-Driven Development (TDD). See `CLAUDE.md` for development workflow and coding standards.

### Running Tests

```bash
# All tests (322 tests)
cargo test

# With output
cargo test -- --nocapture

# Specific crate
cargo test -p api
cargo test -p shared

# Integration tests only
cargo test --test integration_tests
```

### Database Commands

```bash
# Apply database schema
make db-schema

# Test message normalization function
make db-test-normalization

# Connect to ClickHouse CLI
make db-client
```

### Development Workflow

```bash
# Run linting
cargo clippy -- -D warnings

# Format code
cargo fmt

# Run both API and gRPC servers
make run-api-debug

# Watch for changes and rebuild
cargo watch -x "test -p api"
```

## Contributing

Contributions are welcome! Please follow these guidelines:

1. **Tests First** - Write tests before implementing features (TDD)
2. **Linting** - Ensure `cargo clippy -- -D warnings` passes
3. **Formatting** - Run `cargo fmt` before committing
4. **Documentation** - Update relevant docs (README, CHANGELOG, code comments)

See `CLAUDE.md` for detailed coding standards and workflow.

## Roadmap

See `PROJECT.md` for the complete implementation roadmap. Major upcoming features:

- **Phase 4**: Alerting & Notifications (threshold, anomaly, pattern matching)
- **Phase 5**: User Interfaces (Web UI, enhanced CLI, Grafana integration)
- **Phase 6**: Production Readiness (performance optimization, horizontal scaling, security)

## License

MIT
