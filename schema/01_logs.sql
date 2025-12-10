-- Logs table for Heimsight
-- Optimized for time-series log storage and querying

USE heimsight;

-- Note: Timestamp stored as Int64 (nanoseconds) for Rust client compatibility
CREATE TABLE IF NOT EXISTS logs (
    -- Core timestamp field (Int64 nanoseconds for Rust client compatibility)
    timestamp Int64 NOT NULL,

    -- Tracing correlation fields
    trace_id String DEFAULT '',
    span_id String DEFAULT '',

    -- Log metadata
    level LowCardinality(String) NOT NULL,
    message String NOT NULL,
    normalized_message String MATERIALIZED normalizeLogMessage(message),
    service LowCardinality(String) NOT NULL,

    -- Flexible attributes stored as Map
    attributes Map(String, String) DEFAULT map(),

    -- Indexing hints
    INDEX idx_trace_id trace_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_message message TYPE tokenbf_v1(32768, 3, 0) GRANULARITY 1,
    INDEX idx_normalized normalized_message TYPE tokenbf_v1(32768, 3, 0) GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(toDateTime(timestamp / 1000000000))
ORDER BY (service, level, timestamp)
TTL toDateTime(timestamp / 1000000000) + INTERVAL 30 DAY
SETTINGS index_granularity = 8192;
