# ClickHouse Schema for Heimsight

This directory contains SQL initialization scripts for ClickHouse. These scripts are automatically executed when the ClickHouse container starts for the first time.

## Files

### 01_logs.sql
Creates the `logs` table for storing log entries with:
- Time-series partitioning by day
- TTL of 30 days for raw logs
- Indexes for trace_id and message search
- Materialized view `logs_hourly_stats` for aggregated log counts (90 day retention)

### 02_metrics.sql
Creates the `metrics` table for storing metrics (counters, gauges, histograms) with:
- Time-series partitioning by day
- TTL of 90 days for raw metrics
- Support for all metric types including histograms and summaries
- Materialized views for 5-minute and hourly aggregations (180 and 365 day retention)

### 03_traces.sql
Creates the `spans` table for storing distributed trace spans with:
- Time-series partitioning by day
- TTL of 30 days for raw spans
- Support for span events and links
- Indexes for trace_id, span_id, parent_span_id, and name
- Materialized views for trace-level stats and hourly performance metrics

## Design Decisions

### MergeTree Engine
All tables use the MergeTree engine family, which is optimized for:
- High-volume inserts
- Time-series data
- Efficient compression
- Background merging of data parts

### Partitioning Strategy
All tables are partitioned by day (`toYYYYMMDD(timestamp)`) which:
- Enables efficient TTL enforcement
- Improves query performance for time-range queries
- Allows for easy data management

### TTL Configuration
- **Logs**: 30 days (high volume, less long-term value)
- **Metrics**: 90 days raw, longer for aggregations
- **Traces**: 30 days (high volume, debugging-focused)
- **Aggregations**: Longer retention (90-365 days) for historical analysis

### Materialized Views
Materialized views provide automatic downsampling and aggregation:
- **logs_hourly_stats**: Log counts by service and level
- **metrics_5min_agg**: 5-minute metric aggregations (avg, min, max, sum)
- **metrics_hourly_agg**: Hourly metric aggregations
- **trace_stats**: Per-trace summary statistics
- **span_performance_hourly**: Hourly service performance metrics

### Data Types
- **DateTime64(9)**: Nanosecond precision timestamps (OpenTelemetry standard)
- **LowCardinality**: Optimizes storage for low-cardinality strings (service, level, etc.)
- **Map(String, String)**: Flexible key-value storage for attributes/labels
- **Array**: Support for histogram buckets, events, links

## Schema Evolution

To modify the schema:
1. Create a new SQL file with a higher number prefix (e.g., `04_add_field.sql`)
2. Use `ALTER TABLE` commands for schema changes
3. Test locally before deploying to production

Example:
```sql
-- 04_add_environment_field.sql
ALTER TABLE logs ADD COLUMN environment LowCardinality(String) DEFAULT 'production';
```

## Local Development

The schema is automatically applied when running:
```bash
docker-compose up -d clickhouse
```

To manually execute SQL files:
```bash
docker exec -it heimsight-clickhouse clickhouse-client --query "$(cat schema/01_logs.sql)"
```

## Querying Examples

### Logs
```sql
-- Get error logs from last hour
SELECT timestamp, service, level, message
FROM logs
WHERE level = 'ERROR'
  AND timestamp > now() - INTERVAL 1 HOUR
ORDER BY timestamp DESC
LIMIT 100;

-- Hourly error count by service
SELECT hour, service, sum(log_count) as errors
FROM logs_hourly_stats
WHERE level = 'ERROR'
  AND hour > now() - INTERVAL 24 HOUR
GROUP BY hour, service
ORDER BY hour DESC, errors DESC;
```

### Metrics
```sql
-- Get metric values for last 5 minutes
SELECT timestamp, name, value, labels
FROM metrics
WHERE name = 'http_requests_total'
  AND timestamp > now() - INTERVAL 5 MINUTE
ORDER BY timestamp DESC;

-- Hourly average metrics
SELECT
    hour,
    name,
    avgMerge(avg_value) as avg,
    minMerge(min_value) as min,
    maxMerge(max_value) as max
FROM metrics_hourly_agg
WHERE name = 'cpu_usage'
  AND hour > now() - INTERVAL 24 HOUR
GROUP BY hour, name
ORDER BY hour DESC;
```

### Traces
```sql
-- Get slow traces (>1 second)
SELECT trace_id, service, total_duration_ns / 1000000 as duration_ms, span_count
FROM trace_stats
WHERE total_duration_ns > 1000000000
  AND trace_start_time > now() - INTERVAL 1 HOUR
ORDER BY total_duration_ns DESC
LIMIT 20;

-- Get all spans for a trace
SELECT span_id, parent_span_id, name, duration_ns, status_code
FROM spans
WHERE trace_id = 'your-trace-id-here'
ORDER BY start_time ASC;
```

## Monitoring

Check table sizes:
```sql
SELECT
    table,
    formatReadableSize(sum(bytes)) AS size,
    sum(rows) AS rows
FROM system.parts
WHERE database = 'heimsight'
  AND active
GROUP BY table
ORDER BY sum(bytes) DESC;
```

## Performance Tuning

For high-volume workloads, consider:
- Increasing `index_granularity` for less frequent queries
- Adjusting TTL periods based on storage capacity
- Adding more specific indexes for common query patterns
- Tuning ClickHouse server settings in `docker-compose.yml`

---

**Last Updated**: 2025-12-08
