-- Aggregation tables and materialized views for Heimsight
-- These provide automatic downsampling for long-term storage efficiency

USE heimsight;

-- ============================================================================
-- 1-MINUTE METRIC AGGREGATIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS metrics_1min (
    timestamp DateTime,  -- Rounded to 1-minute intervals
    name LowCardinality(String) NOT NULL,
    metric_type LowCardinality(String) NOT NULL,
    service LowCardinality(String) NOT NULL,
    
    -- Aggregated values
    count UInt64,
    sum Float64,
    min Float64,
    max Float64,
    avg Float64,
    
    -- Label hash for grouping
    labels_hash UInt64,
    labels Map(String, String) DEFAULT map()
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, name, labels_hash, timestamp)
TTL timestamp + INTERVAL 30 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_1min_mv TO metrics_1min AS
SELECT
    toStartOfMinute(toDateTime(timestamp / 1000000000)) AS timestamp,
    name,
    metric_type,
    service,
    count() AS count,
    sum(value) AS sum,
    min(value) AS min,
    max(value) AS max,
    avg(value) AS avg,
    cityHash64(toString(labels)) AS labels_hash,
    labels
FROM metrics
GROUP BY timestamp, name, metric_type, service, labels_hash, labels;

-- ============================================================================
-- 5-MINUTE METRIC AGGREGATIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS metrics_5min (
    timestamp DateTime,
    name LowCardinality(String) NOT NULL,
    metric_type LowCardinality(String) NOT NULL,
    service LowCardinality(String) NOT NULL,
    
    count UInt64,
    sum Float64,
    min Float64,
    max Float64,
    avg Float64,
    
    labels_hash UInt64,
    labels Map(String, String) DEFAULT map()
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, name, labels_hash, timestamp)
TTL timestamp + INTERVAL 90 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_5min_mv TO metrics_5min AS
SELECT
    toStartOfFiveMinutes(toDateTime(timestamp / 1000000000)) AS timestamp,
    name,
    metric_type,
    service,
    count() AS count,
    sum(value) AS sum,
    min(value) AS min,
    max(value) AS max,
    avg(value) AS avg,
    cityHash64(toString(labels)) AS labels_hash,
    labels
FROM metrics
GROUP BY timestamp, name, metric_type, service, labels_hash, labels;

-- ============================================================================
-- 1-HOUR METRIC AGGREGATIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS metrics_1hour (
    timestamp DateTime,
    name LowCardinality(String) NOT NULL,
    metric_type LowCardinality(String) NOT NULL,
    service LowCardinality(String) NOT NULL,
    
    count UInt64,
    sum Float64,
    min Float64,
    max Float64,
    avg Float64,
    
    labels_hash UInt64,
    labels Map(String, String) DEFAULT map()
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, name, labels_hash, timestamp)
TTL timestamp + INTERVAL 365 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_1hour_mv TO metrics_1hour AS
SELECT
    toStartOfHour(toDateTime(timestamp / 1000000000)) AS timestamp,
    name,
    metric_type,
    service,
    count() AS count,
    sum(value) AS sum,
    min(value) AS min,
    max(value) AS max,
    avg(value) AS avg,
    cityHash64(toString(labels)) AS labels_hash,
    labels
FROM metrics
GROUP BY timestamp, name, metric_type, service, labels_hash, labels;

-- ============================================================================
-- 1-DAY METRIC AGGREGATIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS metrics_1day (
    timestamp DateTime,
    name LowCardinality(String) NOT NULL,
    metric_type LowCardinality(String) NOT NULL,
    service LowCardinality(String) NOT NULL,
    
    count UInt64,
    sum Float64,
    min Float64,
    max Float64,
    avg Float64,
    
    labels_hash UInt64,
    labels Map(String, String) DEFAULT map()
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, name, labels_hash, timestamp)
TTL timestamp + INTERVAL 730 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_1day_mv TO metrics_1day AS
SELECT
    toStartOfDay(toDateTime(timestamp / 1000000000)) AS timestamp,
    name,
    metric_type,
    service,
    count() AS count,
    sum(value) AS sum,
    min(value) AS min,
    max(value) AS max,
    avg(value) AS avg,
    cityHash64(toString(labels)) AS labels_hash,
    labels
FROM metrics
GROUP BY timestamp, name, metric_type, service, labels_hash, labels;

-- ============================================================================
-- LOG COUNT AGGREGATIONS (by level, service, and normalized message)
-- ============================================================================

CREATE TABLE IF NOT EXISTS logs_1hour_counts (
    timestamp DateTime,
    level LowCardinality(String) NOT NULL,
    service LowCardinality(String) NOT NULL,
    normalized_message String NOT NULL,
    count UInt64,
    
    -- Store example of original message for reference
    sample_message String DEFAULT ''
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, level, normalized_message, timestamp)
TTL timestamp + INTERVAL 365 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS logs_1hour_counts_mv TO logs_1hour_counts AS
SELECT
    toStartOfHour(toDateTime(timestamp / 1000000000)) AS timestamp,
    level,
    service,
    normalized_message,
    count() AS count,
    any(message) AS sample_message  -- Keep one example of the actual message
FROM logs
GROUP BY timestamp, level, service, normalized_message;

CREATE TABLE IF NOT EXISTS logs_1day_counts (
    timestamp DateTime,
    level LowCardinality(String) NOT NULL,
    service LowCardinality(String) NOT NULL,
    normalized_message String NOT NULL,
    count UInt64,
    
    -- Store example of original message for reference
    sample_message String DEFAULT ''
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, level, normalized_message, timestamp)
TTL timestamp + INTERVAL 730 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS logs_1day_counts_mv TO logs_1day_counts AS
SELECT
    toStartOfDay(toDateTime(timestamp / 1000000000)) AS timestamp,
    level,
    service,
    normalized_message,
    count() AS count,
    any(message) AS sample_message  -- Keep one example of the actual message
FROM logs
GROUP BY timestamp, level, service, normalized_message;

-- ============================================================================
-- SPAN PERFORMANCE AGGREGATIONS (latency, throughput, errors)
-- ============================================================================

CREATE TABLE IF NOT EXISTS spans_1hour_stats (
    timestamp DateTime,
    service LowCardinality(String) NOT NULL,
    operation String NOT NULL,
    span_kind LowCardinality(String) NOT NULL,
    status_code LowCardinality(String) NOT NULL,
    
    -- Span counts
    span_count UInt64,
    
    -- Duration statistics (nanoseconds)
    avg_duration_ns Float64,
    min_duration_ns UInt64,
    max_duration_ns UInt64,
    p50_duration_ns Float64,
    p95_duration_ns Float64,
    p99_duration_ns Float64
) ENGINE = AggregatingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, operation, span_kind, status_code, timestamp)
TTL timestamp + INTERVAL 365 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS spans_1hour_stats_mv TO spans_1hour_stats AS
SELECT
    toStartOfHour(toDateTime(start_time / 1000000000)) AS timestamp,
    service,
    operation,
    span_kind,
    status_code,
    count() AS span_count,
    avg(duration_ns) AS avg_duration_ns,
    min(duration_ns) AS min_duration_ns,
    max(duration_ns) AS max_duration_ns,
    quantile(0.5)(duration_ns) AS p50_duration_ns,
    quantile(0.95)(duration_ns) AS p95_duration_ns,
    quantile(0.99)(duration_ns) AS p99_duration_ns
FROM spans
GROUP BY timestamp, service, operation, span_kind, status_code;

CREATE TABLE IF NOT EXISTS spans_1day_stats (
    timestamp DateTime,
    service LowCardinality(String) NOT NULL,
    operation String NOT NULL,
    span_kind LowCardinality(String) NOT NULL,
    status_code LowCardinality(String) NOT NULL,
    
    -- Span counts
    span_count UInt64,
    
    -- Duration statistics (nanoseconds)
    avg_duration_ns Float64,
    min_duration_ns UInt64,
    max_duration_ns UInt64,
    p50_duration_ns Float64,
    p95_duration_ns Float64,
    p99_duration_ns Float64
) ENGINE = AggregatingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, operation, span_kind, status_code, timestamp)
TTL timestamp + INTERVAL 730 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS spans_1day_stats_mv TO spans_1day_stats AS
SELECT
    toStartOfDay(toDateTime(start_time / 1000000000)) AS timestamp,
    service,
    operation,
    span_kind,
    status_code,
    count() AS span_count,
    avg(duration_ns) AS avg_duration_ns,
    min(duration_ns) AS min_duration_ns,
    max(duration_ns) AS max_duration_ns,
    quantile(0.5)(duration_ns) AS p50_duration_ns,
    quantile(0.95)(duration_ns) AS p95_duration_ns,
    quantile(0.99)(duration_ns) AS p99_duration_ns
FROM spans
GROUP BY timestamp, service, operation, span_kind, status_code;

-- ============================================================================
-- TRACE-LEVEL AGGREGATIONS (unique traces, spans per trace)
-- ============================================================================

CREATE TABLE IF NOT EXISTS traces_1hour_stats (
    timestamp DateTime,
    service LowCardinality(String) NOT NULL,
    
    -- Trace counts
    unique_traces UInt64,
    total_spans UInt64
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, timestamp)
TTL timestamp + INTERVAL 365 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS traces_1hour_stats_mv TO traces_1hour_stats AS
SELECT
    toStartOfHour(toDateTime(start_time / 1000000000)) AS timestamp,
    service,
    uniq(trace_id) AS unique_traces,
    count() AS total_spans
FROM spans
GROUP BY timestamp, service;

CREATE TABLE IF NOT EXISTS traces_1day_stats (
    timestamp DateTime,
    service LowCardinality(String) NOT NULL,
    
    -- Trace counts
    unique_traces UInt64,
    total_spans UInt64
) ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, timestamp)
TTL timestamp + INTERVAL 730 DAY
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS traces_1day_stats_mv TO traces_1day_stats AS
SELECT
    toStartOfDay(toDateTime(start_time / 1000000000)) AS timestamp,
    service,
    uniq(trace_id) AS unique_traces,
    count() AS total_spans
FROM spans
GROUP BY timestamp, service;
