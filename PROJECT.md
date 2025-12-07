# Heimsight - Observability Platform

## Project Vision

**Heimsight** is a self-hosted, full-stack observability platform built in Rust. It provides unified logs, traces, metrics, and alerting with a focus on simplicity, performance, and horizontal scalability.

The name combines "Heim" (from Heimdall, the Norse guardian) with "sight," representing clear visibility into your systems.

---

## Core Principles

1. **Incremental Development** - Start with a working MVP, iterate toward production-ready
2. **API-First** - Everything accessible via REST API before building UI
3. **OTLP-Native** - Full OpenTelemetry Protocol compliance for interoperability
4. **SQL-Like Queries** - Familiar query syntax for logs and metrics
5. **Horizontal Scale** - Design for single-node but scale to distributed deployment

---

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (stable) |
| Async Runtime | Tokio |
| Web Framework | Axum |
| Frontend | Rust templates + HTMX |
| Database | TBD - evaluating MongoDB, ScyllaDB, or TimescaleDB |
| Serialization | serde, protobuf (for OTLP) |
| Deployment | Docker (local), AWS ECS (production) |

---

## Feature Overview

### Data Ingestion
- **OTLP gRPC/HTTP** - Full OpenTelemetry Protocol support
- **Application logs** - Structured JSON log ingestion via HTTP
- **System metrics** - Host-level metrics from agents
- **Custom events** - Business events and telemetry

### Storage & Retention
- **Configurable TTL** - Per-data-type retention policies
- **Aggregation** - Downsample old data for long-term storage (InfluxDB-style continuous queries)
- **Hot/aggregated tiers** - Full resolution recent data, aggregated historical data

### Query & Analysis
- **SQL-like query language** - `SELECT * FROM logs WHERE level = 'error' AND service = 'api'`
- **Full-text search** - Search within log messages
- **Trace correlation** - Link logs to traces via trace_id

### Alerting
- **Threshold-based** - Alert when metrics exceed limits
- **Anomaly detection** - ML-based unusual pattern detection
- **Log pattern matching** - Alert on specific log patterns/errors

### Notifications
- Webhooks (generic HTTP callbacks)
- Slack/Discord integrations
- Email (SMTP)
- PagerDuty/Opsgenie

### Interfaces
- **REST API** - Full-featured API for all operations
- **Web UI** - Rust + HTMX dashboard
- **CLI** - Command-line tools for querying and management
- **Grafana datasource** - Compatible with Grafana visualization

### Authentication
- None initially (focus on core features)
- API keys planned for future phases

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Data Sources                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐ │
│  │ OTLP     │  │ HTTP     │  │ Agent    │  │ Custom Events    │ │
│  │ (gRPC)   │  │ Logs     │  │ Metrics  │  │ (webhooks)       │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘ │
└───────┼─────────────┼─────────────┼─────────────────┼───────────┘
        │             │             │                 │
        └─────────────┴──────┬──────┴─────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Ingestion Layer                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Axum HTTP/gRPC Server                  │   │
│  │  - OTLP receiver (traces, metrics, logs)                 │   │
│  │  - HTTP log ingestion                                    │   │
│  │  - Validation & normalization                            │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Processing Pipeline                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                 │
│  │ Parser     │  │ Enricher   │  │ Router     │                 │
│  │            │──▶│            │──▶│            │                 │
│  └────────────┘  └────────────┘  └────────────┘                 │
└─────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Storage Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Logs Store   │  │ Metrics Store│  │ Traces Store │          │
│  │              │  │ (time-series)│  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Aggregation Engine (background)              │   │
│  │  - Downsample old data                                   │   │
│  │  - TTL enforcement                                       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Query Layer                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                 SQL-like Query Engine                     │   │
│  │  - Parse SQL-like syntax                                 │   │
│  │  - Execute against storage                               │   │
│  │  - Aggregate results                                     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   REST API   │    │   Web UI     │    │     CLI      │
│              │    │  (HTMX)      │    │              │
└──────────────┘    └──────────────┘    └──────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Alerting Engine                             │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                 │
│  │ Threshold  │  │ Anomaly    │  │ Pattern    │                 │
│  │ Rules      │  │ Detection  │  │ Matching   │                 │
│  └────────────┘  └────────────┘  └────────────┘                 │
│                         │                                        │
│                         ▼                                        │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Notification Dispatcher                      │   │
│  │  Webhooks │ Slack │ Email │ PagerDuty │ Discord          │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Development Phases

### Phase 1: Foundation (MVP)
Minimal working system for log ingestion and querying.

### Phase 2: Core Features
Add metrics, traces, basic alerting.

### Phase 3: Production Hardening
Aggregation, retention policies, performance optimization.

### Phase 4: Full Platform
Web UI, CLI, advanced alerting, Grafana integration.

### Phase 5: Scale & Polish
Horizontal scaling, anomaly detection, documentation.

---

## Implementation TODO List

Each task is designed to be completed in one session. Complete tasks in order, marking each as done before proceeding.

---

### Phase 1: Foundation (MVP)

#### Step 1.1: Project Scaffolding
**Status:** `[x]` Complete

**Goal:** Create the Rust workspace structure with all crates.

**Tasks:**
- Create workspace `Cargo.toml` with members
- Create `api/` crate (Axum server)
- Create `shared/` crate (common types, models)
- Create `cli/` crate (placeholder)
- Add basic dependencies to each crate
- Create `README.md` with project overview
- Create `CHANGELOG.md`
- Verify `cargo build` succeeds

**Acceptance Criteria:**
- `cargo build` passes
- `cargo test` passes (no tests yet, but runs)
- Directory structure matches PROJECT.md architecture

---

#### Step 1.2: Basic Axum Server
**Status:** `[x]` Complete

**Goal:** Running HTTP server with health check endpoint.

**Tasks:**
- Set up Axum with Tokio runtime in `api/`
- Add `/health` endpoint returning 200 OK
- Add examples/health.http for manual testing
- Add basic configuration (host, port) via environment
- Add `tracing` for structured logging
- Write integration test for health endpoint
- Add graceful shutdown handling

**Acceptance Criteria:**
- Server starts on configured port
- `GET /health` returns 200
- Logs show request/response with tracing
- `cargo test` passes with health check test

---

#### Step 1.3: Log Data Model
**Status:** `[x]` Complete

**Goal:** Define core data structures for logs.

**Tasks:**
- Create `LogEntry` struct in `shared/`
- Fields: timestamp, level, message, service, attributes (key-value)
- Add trace_id/span_id optional fields for correlation
- Implement serde Serialize/Deserialize
- Add validation (non-empty message, valid timestamp)
- Write unit tests for serialization

**Acceptance Criteria:**
- `LogEntry` can serialize to/from JSON
- Validation catches invalid entries
- Tests pass for all edge cases

---

#### Step 1.4: Log Ingestion Endpoint
**Status:** `[x]` Complete

**Goal:** Accept logs via HTTP POST.

**Tasks:**
- Add `POST /api/v1/logs` endpoint
- Accept single log or batch of logs
- Validate incoming log entries
- Return appropriate status codes (201 created, 400 bad request)
- Add request body size limits
- Write integration tests

**Acceptance Criteria:**
- Can POST a log entry and get 201
- Invalid logs return 400 with error details
- Batch ingestion works (array of logs)

---

#### Step 1.5: In-Memory Storage
**Status:** `[x]` Complete

**Goal:** Store logs in memory for querying (temporary, MVP only).

**Tasks:**
- Create `LogStore` trait in `shared/`
- Implement `InMemoryLogStore` with Vec + RwLock
- Add methods: insert, query (by time range), count
- Wire into API server
- Add basic query endpoint `GET /api/v1/logs`
- Add examples/post_logs.http for manual testing
- Add examples/get_logs.http for manual testing
- Support query params: start_time, end_time, limit

**Acceptance Criteria:**
- Logs persist in memory across requests
- Query endpoint returns stored logs
- Time-range filtering works
- Memory store is thread-safe

---

#### Step 1.6: Simple Log Query API
**Status:** `[x]` Complete

**Goal:** Query logs with filters.

**Tasks:**
- Extend `GET /api/v1/logs` with filters
- Support: level, service, contains (message search)
- Add pagination (limit, offset)
- Return total count in response
- Write tests for each filter type

**Acceptance Criteria:**
- Can filter logs by level
- Can filter logs by service name
- Can search within message text
- Pagination works correctly

---

### Phase 2: Core Features

#### Step 2.1: SQL-Like Query Parser
**Status:** `[x]` Complete

**Goal:** Parse SQL-like queries for logs.

**Tasks:**
- Create query parser in `shared/`
- Support: `SELECT * FROM logs WHERE level = 'error'`
- Support: AND, OR, comparison operators
- Support: LIMIT, ORDER BY timestamp
- Parse into AST (Abstract Syntax Tree)
- Write comprehensive parser tests

**Acceptance Criteria:**
- Parser handles basic SELECT queries
- WHERE clauses with multiple conditions work
- Error messages are helpful for syntax errors

---

#### Step 2.2: Query Execution Engine
**Status:** `[x]` Complete

**Goal:** Execute parsed queries against log store.

**Tasks:**
- Create query executor that takes AST
- Apply filters from WHERE clause
- Apply LIMIT and ORDER BY
- Add `POST /api/v1/query` endpoint for SQL queries
- Return results in JSON format
- Write integration tests

**Acceptance Criteria:**
- Can execute SQL-like queries via API
- Complex WHERE clauses work correctly
- Results are properly ordered and limited

---

#### Step 2.3: Metrics Data Model
**Status:** `[x]` Complete

**Goal:** Define data structures for metrics.

**Tasks:**
- Create `Metric` struct (name, value, timestamp, labels)
- Support metric types: counter, gauge, histogram
- Create `MetricStore` trait
- Implement `InMemoryMetricStore`
- Add validation for metric data
- Write unit tests

**Acceptance Criteria:**
- Metrics serialize/deserialize correctly
- Different metric types are handled
- Validation catches invalid metrics

---

#### Step 2.4: Metrics Ingestion & Query
**Status:** `[x]` Complete

**Goal:** Ingest and query metrics.

**Tasks:**
- Add `POST /api/v1/metrics` endpoint
- Add `GET /api/v1/metrics` with filters
- Support: metric name, label filters, time range
- Add aggregation: avg, sum, min, max over time windows
- Write integration tests

**Acceptance Criteria:**
- Can ingest counter/gauge metrics
- Can query metrics by name and labels
- Basic aggregations work

---

#### Step 2.5: Trace Data Model
**Status:** `[x]` Complete

**Goal:** Define data structures for distributed traces.

**Tasks:**
- Create `Span` struct (trace_id, span_id, parent_span_id, name, timestamps, attributes)
- Create `Trace` as collection of spans
- Create `TraceStore` trait
- Implement `InMemoryTraceStore`
- Add service name and operation name fields
- Write unit tests

**Acceptance Criteria:**
- Spans can form a trace tree
- Serialization handles nested structures
- Validation ensures required fields present

---

#### Step 2.6: Trace Ingestion & Query
**Status:** `[x]` Complete

**Goal:** Ingest and query traces.

**Tasks:**
- Add `POST /api/v1/traces` endpoint
- Add `GET /api/v1/traces` with filters
- Add `GET /api/v1/traces/{trace_id}` for single trace
- Support: service filter, time range, duration filter
- Build trace tree from spans
- Write integration tests

**Acceptance Criteria:**
- Can ingest spans
- Can retrieve full trace by ID
- Can search traces by service/duration

---

### Phase 3: OTLP & Storage

#### Step 3.1: OTLP Protobuf Definitions
**Status:** `[ ]` Pending

**Goal:** Add OpenTelemetry Protocol definitions.

**Tasks:**
- Add protobuf dependencies (prost, tonic)
- Include OTLP proto files
- Generate Rust code from protos
- Create conversion: OTLP types → internal types
- Write conversion tests

**Acceptance Criteria:**
- OTLP proto types compile
- Conversions to/from internal types work
- All OTLP log/metric/trace fields mapped

---

#### Step 3.2: OTLP HTTP Receiver
**Status:** `[ ]` Pending

**Goal:** Accept OTLP data over HTTP.

**Tasks:**
- Add `POST /v1/logs` (OTLP HTTP logs endpoint)
- Add `POST /v1/metrics` (OTLP HTTP metrics endpoint)
- Add `POST /v1/traces` (OTLP HTTP traces endpoint)
- Handle protobuf and JSON content types
- Convert OTLP to internal format and store
- Write integration tests with OTLP payloads

**Acceptance Criteria:**
- Standard OTLP exporters can send data
- Both protobuf and JSON formats work
- Data appears in internal stores

---

#### Step 3.3: OTLP gRPC Receiver
**Status:** `[ ]` Pending

**Goal:** Accept OTLP data over gRPC.

**Tasks:**
- Add tonic gRPC server alongside Axum
- Implement LogsService, MetricsService, TracesService
- Handle streaming and unary RPCs
- Wire to same internal stores
- Write integration tests

**Acceptance Criteria:**
- gRPC endpoint accepts OTLP data
- Works with OpenTelemetry SDK exporters
- Performance is acceptable

---

#### Step 3.4: Database Selection & Setup
**Status:** `[ ]` Pending

**Goal:** Choose and set up persistent database.

**Tasks:**
- Evaluate: MongoDB, ScyllaDB, TimescaleDB, ClickHouse
- Document trade-offs for each option
- Choose based on: time-series performance, aggregation support, operational simplicity
- Set up Docker Compose for local development
- Create database schema/collections
- Add connection pooling

**Acceptance Criteria:**
- Database runs in Docker
- Connection from Rust works
- Schema supports logs, metrics, traces

---

#### Step 3.5: Persistent Storage Implementation
**Status:** `[ ]` Pending

**Goal:** Replace in-memory stores with database.

**Tasks:**
- Implement `LogStore` for chosen database
- Implement `MetricStore` for chosen database
- Implement `TraceStore` for chosen database
- Add connection configuration
- Migrate tests to use database
- Ensure existing API tests pass

**Acceptance Criteria:**
- All data persists across restarts
- Query performance is acceptable
- Existing tests pass with DB backend

---

#### Step 3.6: Retention & TTL
**Status:** `[ ]` Pending

**Goal:** Automatic data expiration.

**Tasks:**
- Add retention configuration (per data type)
- Implement TTL enforcement (background job)
- Add API to configure retention policies
- Add metrics for data age distribution
- Write tests for TTL behavior

**Acceptance Criteria:**
- Old data is automatically deleted
- Different TTLs for logs/metrics/traces
- Configuration is runtime-updatable

---

#### Step 3.7: Data Aggregation
**Status:** `[ ]` Pending

**Goal:** Downsample old data for long-term storage.

**Tasks:**
- Define aggregation rules (e.g., 1-minute → 1-hour → 1-day)
- Implement background aggregation job
- Store aggregated data separately
- Query engine uses aggregated data for old time ranges
- Add configuration for aggregation policies

**Acceptance Criteria:**
- Old metrics are aggregated
- Log counts are aggregated by level/service
- Queries seamlessly use aggregated data

---

### Phase 4: Alerting & Notifications

#### Step 4.1: Alert Rule Data Model
**Status:** `[ ]` Pending

**Goal:** Define alert rules structure.

**Tasks:**
- Create `AlertRule` struct (name, condition, severity, notification targets)
- Support rule types: threshold, pattern match
- Create `AlertRuleStore` for persistence
- Add validation for rule definitions
- Write unit tests

**Acceptance Criteria:**
- Rules can be serialized/persisted
- Validation catches invalid rules
- Multiple condition types supported

---

#### Step 4.2: Alert Rule API
**Status:** `[ ]` Pending

**Goal:** CRUD API for alert rules.

**Tasks:**
- Add `POST /api/v1/alerts/rules` - create rule
- Add `GET /api/v1/alerts/rules` - list rules
- Add `GET /api/v1/alerts/rules/{id}` - get rule
- Add `PUT /api/v1/alerts/rules/{id}` - update rule
- Add `DELETE /api/v1/alerts/rules/{id}` - delete rule
- Write integration tests

**Acceptance Criteria:**
- Full CRUD works
- Rules persist across restarts
- Validation on create/update

---

#### Step 4.3: Alert Evaluation Engine
**Status:** `[ ]` Pending

**Goal:** Evaluate rules against incoming data.

**Tasks:**
- Create alert evaluation loop (background task)
- Evaluate threshold rules against metrics
- Evaluate pattern rules against logs
- Track alert state (pending, firing, resolved)
- Implement alert deduplication
- Write tests for evaluation logic

**Acceptance Criteria:**
- Alerts fire when conditions met
- Alerts resolve when conditions clear
- No duplicate alerts for same condition

---

#### Step 4.4: Webhook Notifications
**Status:** `[ ]` Pending

**Goal:** Send alerts via webhooks.

**Tasks:**
- Create `NotificationChannel` trait
- Implement `WebhookChannel`
- Add configuration for webhook URLs
- Include alert details in payload
- Add retry logic for failed deliveries
- Write tests with mock webhook server

**Acceptance Criteria:**
- Webhooks fire on alert
- Payload includes all alert details
- Failed webhooks are retried

---

#### Step 4.5: Slack/Discord Notifications
**Status:** `[ ]` Pending

**Goal:** Send alerts to chat platforms.

**Tasks:**
- Implement `SlackChannel` with incoming webhooks
- Implement `DiscordChannel` with webhooks
- Format messages nicely for each platform
- Add configuration for channel webhooks
- Write integration tests

**Acceptance Criteria:**
- Slack messages are well-formatted
- Discord messages are well-formatted
- Configuration is straightforward

---

#### Step 4.6: Email Notifications
**Status:** `[ ]` Pending

**Goal:** Send alerts via email.

**Tasks:**
- Implement `EmailChannel` with SMTP
- Add SMTP configuration (host, port, auth)
- Create HTML email templates
- Add recipient configuration
- Write tests with mock SMTP

**Acceptance Criteria:**
- Emails send via SMTP
- HTML formatting looks good
- Multiple recipients supported

---

#### Step 4.7: PagerDuty/Opsgenie Integration
**Status:** `[ ]` Pending

**Goal:** Integrate with incident management platforms.

**Tasks:**
- Implement `PagerDutyChannel` using Events API v2
- Implement `OpsgenieChannel` using REST API
- Map alert severity to platform priorities
- Handle acknowledge/resolve events
- Write integration tests

**Acceptance Criteria:**
- Incidents created in PagerDuty/Opsgenie
- Severity correctly mapped
- Resolved alerts close incidents

---

### Phase 5: User Interfaces

#### Step 5.1: CLI Project Setup
**Status:** `[ ]` Pending

**Goal:** Set up CLI tool structure.

**Tasks:**
- Set up `cli/` crate with clap
- Add configuration (API URL, auth)
- Add basic commands structure
- Implement `health` command
- Add output formatting (table, JSON)
- Write tests for CLI parsing

**Acceptance Criteria:**
- CLI builds and runs
- `heimsight health` works
- Help text is clear

---

#### Step 5.2: CLI Query Commands
**Status:** `[ ]` Pending

**Goal:** Query data from CLI.

**Tasks:**
- Add `heimsight logs` command with filters
- Add `heimsight metrics` command with filters
- Add `heimsight traces` command with filters
- Add `heimsight query` for SQL-like queries
- Support output formats: table, JSON, CSV
- Add streaming/follow mode for logs

**Acceptance Criteria:**
- All query commands work
- Output is well-formatted
- Follow mode streams new logs

---

#### Step 5.3: CLI Management Commands
**Status:** `[ ]` Pending

**Goal:** Manage alerts and configuration from CLI.

**Tasks:**
- Add `heimsight alerts list` command
- Add `heimsight alerts create` command
- Add `heimsight alerts delete` command
- Add `heimsight config` for server configuration
- Add `heimsight status` for system overview
- Write tests

**Acceptance Criteria:**
- Can manage alerts from CLI
- Can view system status
- Commands have helpful error messages

---

#### Step 5.4: Web UI Foundation
**Status:** `[ ]` Pending

**Goal:** Basic web UI with HTMX.

**Tasks:**
- Add template engine (askama or maud)
- Create base layout template
- Add static file serving (CSS, minimal JS)
- Create home page with navigation
- Add `/ui/` routes for web interface
- Style with simple CSS (no framework)

**Acceptance Criteria:**
- Web UI accessible at /ui/
- Navigation works
- Looks clean and functional

---

#### Step 5.5: Log Explorer UI
**Status:** `[ ]` Pending

**Goal:** Web interface for browsing logs.

**Tasks:**
- Create log list view with filters
- Add real-time log streaming (SSE or WebSocket)
- Add log detail view
- Add search functionality
- Add time range picker
- Implement with HTMX for interactivity

**Acceptance Criteria:**
- Can browse logs in browser
- Filters work without page reload
- Real-time updates work

---

#### Step 5.6: Metrics Dashboard UI
**Status:** `[ ]` Pending

**Goal:** Web interface for viewing metrics.

**Tasks:**
- Create metrics list view
- Add basic charting (chart.js or similar minimal lib)
- Add metric detail view with history
- Add time range selection
- Add label filtering

**Acceptance Criteria:**
- Can view metrics in browser
- Charts display metric history
- Filters work interactively

---

#### Step 5.7: Trace Viewer UI
**Status:** `[ ]` Pending

**Goal:** Web interface for viewing traces.

**Tasks:**
- Create trace list view
- Add trace detail view with span tree
- Add waterfall/timeline visualization
- Show span details on click
- Add trace search

**Acceptance Criteria:**
- Can browse traces
- Span tree visualization works
- Timing visualization is clear

---

#### Step 5.8: Alerts Management UI
**Status:** `[ ]` Pending

**Goal:** Web interface for managing alerts.

**Tasks:**
- Create alert rules list view
- Add create/edit rule form
- Show alert history
- Add alert status indicators
- Add notification channel configuration

**Acceptance Criteria:**
- Can manage alert rules in browser
- Forms validate input
- Alert status is visible

---

#### Step 5.9: Grafana Datasource Plugin
**Status:** `[ ]` Pending

**Goal:** Make Heimsight a Grafana datasource.

**Tasks:**
- Implement Grafana datasource HTTP API
- Support metric queries from Grafana
- Support log queries (Explore)
- Document setup in Grafana
- Write integration tests

**Acceptance Criteria:**
- Grafana can add Heimsight as datasource
- Metric queries work in dashboards
- Log queries work in Explore

---

### Phase 6: Production Readiness

#### Step 6.1: Anomaly Detection
**Status:** `[ ]` Pending

**Goal:** ML-based anomaly detection for metrics.

**Tasks:**
- Implement simple anomaly detection (z-score, moving average)
- Add anomaly-based alert rules
- Store baseline statistics
- Add anomaly scores to metrics
- Write tests for detection accuracy

**Acceptance Criteria:**
- Can detect metric anomalies
- Alerts fire on anomalies
- False positive rate is acceptable

---

#### Step 6.2: Performance Optimization
**Status:** `[ ]` Pending

**Goal:** Optimize for high throughput.

**Tasks:**
- Add benchmarks for ingestion
- Profile and optimize hot paths
- Add batching for database writes
- Implement connection pooling tuning
- Add caching where beneficial
- Document performance characteristics

**Acceptance Criteria:**
- Ingestion handles 10k+ events/second
- Query latency is under 100ms (simple queries)
- Memory usage is predictable

---

#### Step 6.3: Horizontal Scaling
**Status:** `[ ]` Pending

**Goal:** Support multiple Heimsight instances.

**Tasks:**
- Make all components stateless (state in DB)
- Add instance coordination for alert evaluation
- Document load balancer setup
- Test with multiple instances
- Add distributed tracing for Heimsight itself

**Acceptance Criteria:**
- Multiple instances can run behind LB
- Alerts don't duplicate across instances
- No data loss on instance restart

---

#### Step 6.4: Docker & Deployment
**Status:** `[ ]` Pending

**Goal:** Production-ready Docker setup.

**Tasks:**
- Create optimized Dockerfile (multi-stage)
- Create docker-compose.yml for full stack
- Add health checks to containers
- Document environment variables
- Add example AWS ECS task definition
- Add example Kubernetes manifests

**Acceptance Criteria:**
- Docker image is small and secure
- docker-compose brings up full system
- Deployment docs are clear

---

#### Step 6.5: Monitoring & Observability (Meta)
**Status:** `[ ]` Pending

**Goal:** Heimsight monitors itself.

**Tasks:**
- Expose Prometheus metrics endpoint
- Add key metrics: ingestion rate, query latency, storage size
- Add structured logging with tracing
- Document operational runbook
- Add alerting for Heimsight health

**Acceptance Criteria:**
- Prometheus can scrape Heimsight
- Key operational metrics available
- Runbook covers common issues

---

#### Step 6.6: Security Hardening
**Status:** `[ ]` Pending

**Goal:** Prepare for API key authentication.

**Tasks:**
- Add API key middleware (optional, off by default)
- Add rate limiting
- Add input size limits
- Security audit of all endpoints
- Add TLS termination documentation
- Run `cargo audit` clean

**Acceptance Criteria:**
- API keys work when enabled
- Rate limiting prevents abuse
- No security vulnerabilities

---

#### Step 6.7: Documentation & Polish
**Status:** `[ ]` Pending

**Goal:** Comprehensive documentation.

**Tasks:**
- Complete README with all features
- Add API documentation (OpenAPI/Swagger)
- Add architecture documentation
- Add operator guide
- Add developer guide for contributors
- Add example dashboards and alerts

**Acceptance Criteria:**
- New users can get started easily
- API is fully documented
- Operators have clear guides

---

## Quick Reference

### Starting Development
```bash
# Clone and build
cargo build

# Run tests
cargo test

# Start server (development)
cargo run -p api

# Run with Docker
docker-compose up
```

### API Endpoints (MVP)
```
GET  /health              - Health check
POST /api/v1/logs         - Ingest logs
GET  /api/v1/logs         - Query logs
POST /api/v1/query        - SQL-like query
POST /v1/logs             - OTLP HTTP logs
POST /v1/metrics          - OTLP HTTP metrics
POST /v1/traces           - OTLP HTTP traces
```

### Environment Variables
```
HEIMSIGHT_HOST=0.0.0.0
HEIMSIGHT_PORT=8080
HEIMSIGHT_LOG_LEVEL=info
HEIMSIGHT_DB_URL=mongodb://localhost:27017/heimsight
```

---

## Notes for AI/Developer

1. **Complete one step at a time** - Each step is designed to be a single session of work
2. **Mark steps complete** - Update the `[ ]` to `[x]` when done
3. **Follow CLAUDE_INSTRUCTIONS.md** - TDD, linting, documentation requirements apply
4. **Tests first** - Write failing tests before implementation
5. **Ask if unclear** - Requirements can be clarified before implementing
6. **Keep it simple** - MVP first, enhance later

---

**Last Updated:** 2025-12-07
