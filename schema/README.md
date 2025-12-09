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
