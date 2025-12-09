# Heimsight

A self-hosted, full-stack observability platform built in Rust.

Heimsight provides unified logs, traces, metrics, and alerting with a focus on simplicity, performance, and horizontal scalability. The name combines "Heim" (from Heimdall, the Norse guardian) with "sight," representing clear visibility into your systems.

## Features

- **OTLP-Native** - Full OpenTelemetry Protocol compliance for logs, metrics, and traces
- **SQL-Like Queries** - Familiar query syntax for exploring your data
- **Unified Platform** - Logs, metrics, traces, and alerting in one place
- **Horizontal Scaling** - Designed for single-node but scales to distributed deployment
- **Multiple Interfaces** - REST API, Web UI, CLI, and Grafana-compatible datasource

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

## Development

This project follows Test-Driven Development (TDD). See `CLAUDE_INSTRUCTIONS.md` for development workflow and coding standards.

### Running Tests

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific crate
cargo test -p api
```

## License

MIT
