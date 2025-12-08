-- Logs table for Heimsight
-- Optimized for time-series log storage and querying

USE heimsight;

CREATE TABLE IF NOT EXISTS logs (
    -- Core timestamp field for time-series partitioning
    timestamp DateTime64(9) NOT NULL,

    -- Tracing correlation fields
    trace_id String DEFAULT '',
    span_id String DEFAULT '',

    -- Log metadata
    level LowCardinality(String) NOT NULL,
    message String NOT NULL,
    service LowCardinality(String) NOT NULL,

    -- Flexible attributes stored as Map
    attributes Map(String, String) DEFAULT map(),

    -- Indexing hints
    INDEX idx_trace_id trace_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_message message TYPE tokenbf_v1(32768, 3, 0) GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, level, timestamp)
TTL toDateTime(timestamp) + INTERVAL 30 DAY
SETTINGS index_granularity = 8192;

-- Materialized view for log level counts by service (aggregation)
CREATE MATERIALIZED VIEW IF NOT EXISTS logs_hourly_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(hour)
ORDER BY (service, level, hour)
TTL toDateTime(hour) + INTERVAL 90 DAY
AS SELECT
    toStartOfHour(timestamp) AS hour,
    service,
    level,
    count() AS log_count,
    uniqExact(trace_id) AS unique_traces
FROM logs
WHERE trace_id != ''
GROUP BY hour, service, level;
