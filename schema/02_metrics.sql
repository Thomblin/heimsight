-- Metrics table for Heimsight
-- Optimized for time-series metric storage and aggregation

USE heimsight;

-- Note: Timestamp stored as Int64 (nanoseconds) for Rust client compatibility
CREATE TABLE IF NOT EXISTS metrics (
    -- Core timestamp (Int64 nanoseconds for Rust client compatibility)
    timestamp Int64 NOT NULL,

    -- Metric identification
    name LowCardinality(String) NOT NULL,
    metric_type LowCardinality(String) NOT NULL, -- counter, gauge, histogram, summary

    -- Metric value (different fields for different types)
    value Float64 NOT NULL,

    -- Labels as Map for flexible dimensionality
    labels Map(String, String) DEFAULT map(),

    -- Service/source identification
    service LowCardinality(String) NOT NULL,

    -- Histogram-specific fields (optional)
    bucket_counts Array(UInt64) DEFAULT [],
    bucket_bounds Array(Float64) DEFAULT [],

    -- Summary-specific fields (optional)
    quantile_values Array(Float64) DEFAULT [],
    quantiles Array(Float64) DEFAULT [],

    -- Index for metric name lookups
    INDEX idx_name name TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(toDateTime(timestamp / 1000000000))
ORDER BY (service, name, timestamp)
TTL toDateTime(timestamp / 1000000000) + INTERVAL 90 DAY
SETTINGS index_granularity = 8192;
