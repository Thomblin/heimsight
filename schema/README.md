# Heimsight Database Schema

This directory contains the ClickHouse database schema for Heimsight.

## Schema Files

- `01_logs.sql` - Logs table schema
- `02_metrics.sql` - Metrics table schema  
- `03_traces.sql` - Traces (spans) table schema

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

### Applying Schema

To initialize the database:

```bash
# Start ClickHouse
docker compose up -d

# Apply schema files in order
docker exec heimsight-clickhouse clickhouse-client --multiquery < schema/01_logs.sql
docker exec heimsight-clickhouse clickhouse-client --multiquery < schema/02_metrics.sql
docker exec heimsight-clickhouse clickhouse-client --multiquery < schema/03_traces.sql
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
