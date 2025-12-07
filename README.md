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

### Running the API Server

```bash
# Development mode
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
| `HEIMSIGHT_HOST` | API server bind address | `0.0.0.0` |
| `HEIMSIGHT_PORT` | API server port | `8080` |
| `RUST_LOG` | Log level filter | `info` |
| `HEIMSIGHT_API_URL` | CLI: API server URL | `http://localhost:8080` |

## API Endpoints

(Coming in future steps)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/api/v1/logs` | Ingest logs |
| `GET` | `/api/v1/logs` | Query logs |
| `POST` | `/api/v1/query` | SQL-like query |

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
