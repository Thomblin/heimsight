# Database Evaluation for Heimsight

## Overview
This document evaluates four database options for Heimsight's persistent storage layer. The system needs to handle high-volume time-series data (logs, metrics, traces) with efficient aggregation and querying capabilities.

---

## Requirements Summary

### Must Have
- **Time-series optimized**: Efficient storage and querying of timestamped data
- **High write throughput**: Handle 10k+ events/second ingestion
- **Flexible schema**: Support semi-structured data (logs with arbitrary attributes)
- **Aggregation support**: Time-based rollups, grouping, statistical functions
- **TTL/retention**: Automatic data expiration
- **Rust client**: Well-maintained Rust driver
- **Operational simplicity**: Easy to deploy, monitor, maintain

### Nice to Have
- Horizontal scalability
- Built-in compression
- SQL-like query interface
- Active community and commercial support

---

## Database Options

### 1. ClickHouse

**Overview**: Column-oriented OLAP database designed for analytics workloads.

#### Pros
- **Exceptional time-series performance**: Purpose-built for analytical queries on time-series data
- **Superior compression**: 10-100x compression ratios typical
- **SQL support**: Full SQL with extensions for analytics (great for our SQL-like query feature)
- **Aggregation engine**: Built-in materialized views, continuous aggregation
- **TTL support**: Native table/column-level TTL
- **Mature Rust client**: `clickhouse-rs` is well-maintained
- **Widely adopted**: Used by Cloudflare, Uber, eBay for observability
- **Excellent documentation**: Comprehensive guides and examples
- **Horizontal scaling**: Supports sharding and replication

#### Cons
- **Memory hungry**: Requires significant RAM for good performance
- **Complex operations**: Updates/deletes are expensive (but rare in observability)
- **Steeper learning curve**: More features = more complexity

#### Use Case Fit
**Excellent** - ClickHouse is used by many observability platforms (Grafana Tempo, SigNoz, etc.). Perfect for our requirements.

#### Example Schema
```sql
CREATE TABLE logs (
    timestamp DateTime64(9),
    trace_id String,
    span_id String,
    level LowCardinality(String),
    message String,
    service LowCardinality(String),
    attributes Map(String, String)
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, level, timestamp)
TTL timestamp + INTERVAL 30 DAY;
```

---

### 2. TimescaleDB

**Overview**: PostgreSQL extension for time-series data.

#### Pros
- **PostgreSQL compatibility**: Full SQL support, mature ecosystem
- **Time-series optimization**: Automatic partitioning (hypertables)
- **Continuous aggregations**: Built-in downsampling and rollups
- **Retention policies**: Native TTL support
- **Strong Rust support**: `tokio-postgres` + `sqlx` are excellent
- **ACID compliance**: Full transactional guarantees (overkill for us, but nice)
- **Operational familiarity**: Many teams know PostgreSQL

#### Cons
- **Write performance**: Not as fast as column-stores for analytics workloads
- **Compression**: Less efficient than ClickHouse
- **Resource usage**: Can be memory-intensive under load
- **Scaling complexity**: Sharding requires TimescaleDB commercial (or manual setup)

#### Use Case Fit
**Good** - Solid choice if you want PostgreSQL's reliability and SQL compatibility. Less optimal for pure analytical workloads.

#### Example Schema
```sql
CREATE TABLE logs (
    timestamp TIMESTAMPTZ NOT NULL,
    trace_id TEXT,
    span_id TEXT,
    level TEXT,
    message TEXT,
    service TEXT,
    attributes JSONB
);

SELECT create_hypertable('logs', 'timestamp');
SELECT add_retention_policy('logs', INTERVAL '30 days');
```

---

### 3. MongoDB

**Overview**: Document-oriented NoSQL database.

#### Pros
- **Flexible schema**: JSON documents naturally fit log structure
- **Simple operations**: Easy to get started
- **Time-series collections**: Native support since v5.0
- **TTL indexes**: Built-in expiration
- **Good Rust client**: `mongodb` crate is official and maintained
- **Horizontal scaling**: Sharding is straightforward

#### Cons
- **Aggregation limitations**: Pipeline is powerful but less intuitive than SQL
- **Time-series optimization**: Good but not as specialized as ClickHouse
- **Compression**: Not as efficient for analytical data
- **Query performance**: Generally slower for analytical queries vs column-stores
- **Memory usage**: Can be high for indexes

#### Use Case Fit
**Fair** - Works but not optimized for observability workloads. Better for transactional document storage.

#### Example Schema
```javascript
db.createCollection("logs", {
    timeseries: {
        timeField: "timestamp",
        metaField: "metadata",
        granularity: "seconds"
    },
    expireAfterSeconds: 2592000  // 30 days
});
```

---

### 4. ScyllaDB

**Overview**: High-performance Cassandra-compatible database (C++ rewrite).

#### Pros
- **Extreme write performance**: Can handle 1M+ writes/second
- **Horizontal scalability**: Distributed by design
- **Low latency**: Consistently fast reads/writes
- **Rust client**: `scylla` crate is official and excellent
- **Operational maturity**: Battle-tested at scale

#### Cons
- **Query limitations**: No joins, limited aggregations (by design)
- **No built-in TTL aggregation**: Must implement manually
- **Complex data modeling**: Requires denormalization
- **Overkill for MVP**: Designed for multi-datacenter scale
- **Higher operational complexity**: More moving parts

#### Use Case Fit
**Overkill** - Excellent if you need multi-datacenter replication and millions of writes/second. Too complex for our MVP.

---

## Comparison Matrix

| Feature                      | ClickHouse  | TimescaleDB | MongoDB    | ScyllaDB    |
|---------                     |------------ |-------------|---------   |----------   |
| **Time-series optimization** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐    | ⭐⭐⭐     | ⭐⭐        |
| **Write throughput**         | ⭐⭐⭐⭐   | ⭐⭐⭐      | ⭐⭐⭐     | ⭐⭐⭐⭐⭐  |
| **Query performance**        | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐    | ⭐⭐⭐     | ⭐⭐        |
| **Aggregation support**      | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐  | ⭐⭐⭐     | ⭐⭐        |
| **SQL compatibility**        | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐  | ⭐         | ⭐          |
| **Operational simplicity**   | ⭐⭐⭐     | ⭐⭐⭐⭐    | ⭐⭐⭐⭐   | ⭐⭐        |
| **Rust client quality**      | ⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐  | ⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐  | 
| **Compression**              | ⭐⭐⭐⭐⭐ | ⭐⭐⭐      | ⭐⭐       | ⭐⭐⭐      |
| **TTL support**              | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐  | ⭐⭐⭐⭐   | ⭐⭐⭐      |
| **Community/docs**           | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐    | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐    |
| **Horizontal scaling**       | ⭐⭐⭐⭐   | ⭐⭐⭐      | ⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐  |

---

## Recommendation

### Primary Recommendation: **ClickHouse**

**Rationale:**
1. **Purpose-built for observability**: Many production observability platforms use ClickHouse (SigNoz, Grafana Tempo, etc.)
2. **Exceptional time-series performance**: Designed exactly for our use case
3. **SQL compatibility**: Aligns perfectly with our SQL-like query feature (Step 2.1-2.2)
4. **Superior compression**: Reduces storage costs significantly
5. **Built-in aggregation**: Materialized views handle downsampling (Step 3.7)
6. **Native TTL**: Simplifies retention policies (Step 3.6)
7. **Proven at scale**: Used by companies with massive observability workloads

**Trade-offs:**
- More memory-intensive than alternatives
- Slightly more complex than MongoDB for initial setup
- Updates/deletes are expensive (but rare in observability)

### Alternative: **TimescaleDB**

If you prefer PostgreSQL's reliability and operational familiarity, TimescaleDB is a solid second choice. It's easier to operate but trades some analytical performance for transactional guarantees we don't need.

---

## Implementation Plan (ClickHouse)

### Docker Compose Setup
```yaml
services:
  clickhouse:
    image: clickhouse/clickhouse-server:latest
    ports:
      - "8123:8123"  # HTTP interface
      - "9000:9000"  # Native protocol
    environment:
      CLICKHOUSE_DB: heimsight
      CLICKHOUSE_USER: heimsight
      CLICKHOUSE_PASSWORD: heimsight_dev
    volumes:
      - clickhouse_data:/var/lib/clickhouse
      - ./schema:/docker-entrypoint-initdb.d
    healthcheck:
      test: ["CMD", "clickhouse-client", "--query", "SELECT 1"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  clickhouse_data:
```

### Rust Dependencies
```toml
# api/Cargo.toml
clickhouse = "0.12"  # Official Rust client
```

### Connection Configuration
```rust
// Environment variables
HEIMSIGHT_DB_URL=http://localhost:8123
HEIMSIGHT_DB_NAME=heimsight
HEIMSIGHT_DB_USER=heimsight
HEIMSIGHT_DB_PASSWORD=heimsight_dev
```

---

## Next Steps After Selection

1. Add ClickHouse to Docker Compose
2. Create initialization SQL scripts
3. Add `clickhouse` crate to dependencies
4. Implement connection pooling
5. Create database schema (logs, metrics, traces tables)
6. Write connection tests
7. Update README with database setup instructions

---

**Recommendation Summary**: Choose **ClickHouse** for production-grade observability performance, or **TimescaleDB** if PostgreSQL operational simplicity is more important.

**Last Updated**: 2025-12-08
