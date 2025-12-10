# Heimsight Database Schema

This directory contains the ClickHouse database schema for Heimsight.

## Schema Files

- `00_functions.sql` - User-defined functions (message normalization, etc.)
- `01_logs.sql` - Logs table schema
- `02_metrics.sql` - Metrics table schema  
- `03_traces.sql` - Traces (spans) table schema
- `04_aggregations.sql` - Aggregation tables and materialized views

## Important Notes

### Timestamp Storage

All timestamp columns are stored as `Int64` (nanoseconds since Unix epoch) rather than `DateTime64(9)` to ensure compatibility with the Rust `clickhouse` crate (v0.14).

The `clickhouse` Rust client has issues deserializing `DateTime64` types when using the `#[derive(clickhouse::Row)]` macro. Using `Int64` provides:
- Full nanosecond precision
- Seamless Rust client compatibility
- Efficient storage and querying

Partitioning and TTL expressions convert Int64 nanoseconds to DateTime using `toDateTime(timestamp / 1000000000)`.

### Data Retention (TTL)

Each table has a TTL (Time-To-Live) policy configured at the schema level:
- **Logs**: 30 days (configurable in schema)
- **Metrics**: 90 days (configurable in schema)
- **Traces**: 30 days (configurable in schema)

ClickHouse automatically deletes data older than the configured TTL. The TTL is enforced at the database level for efficiency and reliability.

#### Runtime Retention Configuration

Heimsight provides runtime retention configuration via the API (`/api/v1/config/retention`), which allows operators to:
- View current retention policies
- Update retention policies dynamically
- Track data age metrics via `/api/v1/config/retention/metrics`

**Automatic TTL Updates**: When using ClickHouse-backed storage, updating retention policies via the API automatically updates the database TTL using `ALTER TABLE` statements. The API will:

1. Validate the new retention policy
2. Execute `ALTER TABLE` commands to update ClickHouse TTL
3. Update the runtime configuration
4. Return success or error response

Example API calls:

```bash
# Update all retention policies
PUT /api/v1/config/retention
{
  "logs": { "data_type": "logs", "ttl_days": 60 },
  "metrics": { "data_type": "metrics", "ttl_days": 180 },
  "traces": { "data_type": "traces", "ttl_days": 45 }
}

# Update a single policy
PUT /api/v1/config/retention/policy
{
  "data_type": "metrics",
  "ttl_days": 180
}
```

**Manual TTL Updates**: If needed, you can also manually update TTL directly in ClickHouse:

```sql
-- Example: Update logs table TTL to 60 days
ALTER TABLE logs MODIFY TTL toDateTime(timestamp / 1000000000) + INTERVAL 60 DAY;

-- Example: Update metrics table TTL to 180 days
ALTER TABLE metrics MODIFY TTL toDateTime(timestamp / 1000000000) + INTERVAL 180 DAY;

-- Example: Update spans table TTL to 45 days
ALTER TABLE spans MODIFY TTL toDateTime(start_time / 1000000000) + INTERVAL 45 DAY;
```

**Monitoring**: The data age monitoring background job tracks the oldest and newest data timestamps and warns if data age exceeds the configured retention policy.

### Data Aggregation

Heimsight uses ClickHouse materialized views for automatic data aggregation, providing multi-tier storage for long-term retention:

#### Metric Aggregations

| Table | Interval | Retention | Purpose |
|-------|----------|-----------|---------|
| `metrics` | Raw | 90 days | Recent detailed metrics |
| `metrics_1min` | 1 minute | 30 days | Short-term trends |
| `metrics_5min` | 5 minutes | 90 days | Medium-term analysis |
| `metrics_1hour` | 1 hour | 365 days | Long-term trends |
| `metrics_1day` | 1 day | 730 days | Historical analysis |

Each aggregation includes: count, sum, min, max, avg grouped by service, name, and labels.

#### Log Aggregations

| Table | Interval | Retention | Purpose |
|-------|----------|-----------|---------|
| `logs` | Raw | 30 days | Recent detailed logs |
| `logs_1hour_counts` | 1 hour | 365 days | Hourly volume by level/service/message pattern |
| `logs_1day_counts` | 1 day | 730 days | Daily volume by level/service/message pattern |

Log aggregations provide count data for volume analysis and trend visualization.

#### Trace/Span Aggregations

| Table | Interval | Retention | Purpose |
|-------|----------|-----------|---------|
| `spans` | Raw | 30 days | Recent detailed traces |
| `spans_1hour_stats` | 1 hour | 365 days | Hourly latency percentiles, throughput, errors |
| `spans_1day_stats` | 1 day | 730 days | Daily performance statistics |
| `traces_1hour_stats` | 1 hour | 365 days | Hourly trace characteristics |
| `traces_1day_stats` | 1 day | 730 days | Daily trace statistics |

Span aggregations include:
- **Duration statistics**: avg, min, max, p50, p95, p99 (in nanoseconds)
- **Span counts**: Total spans per hour/day
- **Error tracking**: Grouped by status_code
- **Service breakdown**: By service, operation, and span_kind

Trace aggregations include:
- **Unique traces**: Count of distinct trace_ids per hour/day
- **Total spans**: Total span count per hour/day
- **Trace complexity**: Calculate `total_spans / unique_traces` at query time for average spans per trace

**Message Normalization**: Log messages are automatically normalized to group similar messages together. The `normalizeLogMessage()` function strips variable parts:

- **Timestamps**: `2024-12-09T10:15:23.456Z` → `<TIMESTAMP>`
- **UUIDs**: `550e8400-e29b-41d4-a716-446655440000` → `<UUID>`
- **IP Addresses**: `192.168.1.1` → `<IP>`, `2001:db8::1` → `<IPv6>`
- **Numbers with units**: `5.01ms`, `250MB`, `3.5s` → `<NUM>ms`, `<NUM>MB`, `<NUM>s`
- **Numbers**: `12345`, `3.14159` → `<NUM>`
- **URLs**: `https://api.example.com/path` → `<URL>`
- **Email**: `user@example.com` → `<EMAIL>`
- **Paths**: `/var/log/app.log` → `<PATH>`
- **Hex Values**: `0x1a2b3c` → `<HEX>`

**Examples**:
```
Original:  "Error at 2024-12-09T10:15:23Z: Connection to 192.168.1.1:5432 failed (request_id: 12345)"
Normalized: "Error at <TIMESTAMP>: Connection to <IP>:<NUM> failed (request_id: <NUM>)"

Original:  "Query took 5.01ms and returned 250MB"
Normalized: "Query took <NUM>ms and returned <NUM>MB"

Original:  "Download completed: 1024KB in 125.5ms"
Normalized: "Download completed: <NUM>KB in <NUM>ms"
```

All instances of these patterns are grouped together regardless of the specific values, enabling effective trend analysis.

#### How It Works

1. **Materialized Views**: ClickHouse automatically populates aggregation tables as data arrives
2. **No Background Jobs**: Aggregation happens in real-time via ClickHouse MergeTree engine
3. **Automatic TTL**: Each aggregation level has its own retention period
4. **Storage Efficiency**: Older data is downsampled, reducing storage by 90%+

#### Querying Aggregated Data

Use the SQL query API to access aggregated tables:

```bash
# Query 1-hour metric aggregates
POST /api/v1/query
{
  "query": "SELECT timestamp, avg FROM metrics_1hour WHERE name = 'cpu_usage' LIMIT 100"
}

# Query daily log counts
POST /api/v1/query
{
  "query": "SELECT * FROM logs_1day_counts WHERE level = 'error' LIMIT 30"
}

# Query span latency statistics
POST /api/v1/query
{
  "query": "SELECT service, operation, p95_duration_ns / 1000000 as p95_ms FROM spans_1hour_stats LIMIT 100"
}

# Find operations with high error rates
POST /api/v1/query
{
  "query": "SELECT service, operation, countIf(status_code != 'OK') * 100.0 / sum(span_count) as error_rate FROM spans_1day_stats GROUP BY service, operation HAVING error_rate > 5"
}
```

See `examples/aggregation.http` for more examples.

### Applying Schema

To initialize the database:

```bash
# Start ClickHouse
docker compose up -d

# Apply schema files in order
docker compose exec -T clickhouse clickhouse-client --multiquery < schema/00_functions.sql
docker compose exec -T clickhouse clickhouse-client --multiquery < schema/01_logs.sql
docker compose exec -T clickhouse clickhouse-client --multiquery < schema/02_metrics.sql
docker compose exec -T clickhouse clickhouse-client --multiquery < schema/03_traces.sql
docker compose exec -T clickhouse clickhouse-client --multiquery < schema/04_aggregations.sql
```

Or use the Makefile target:

```bash
make db-schema
```

## Schema Updates

When updating schema:
1. Update the relevant `.sql` file
2. Document changes in this README
3. Test with both empty database and existing data
4. Consider migration path for production deployments
