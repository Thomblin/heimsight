-- Metrics table for Heimsight
-- Optimized for time-series metric storage and aggregation

USE heimsight;

CREATE TABLE IF NOT EXISTS metrics (
    -- Core timestamp for time-series
    timestamp DateTime64(9) NOT NULL,

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
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (service, name, timestamp)
TTL toDateTime(timestamp) + INTERVAL 90 DAY
SETTINGS index_granularity = 8192;

-- Materialized view for metric aggregations (5-minute intervals)
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_5min_agg
ENGINE = AggregatingMergeTree()
PARTITION BY toYYYYMMDD(time_bucket)
ORDER BY (service, name, time_bucket)
TTL toDateTime(time_bucket) + INTERVAL 180 DAY
AS SELECT
    toStartOfFiveMinutes(timestamp) AS time_bucket,
    service,
    name,
    metric_type,
    labels,
    avgState(value) AS avg_value,
    minState(value) AS min_value,
    maxState(value) AS max_value,
    sumState(value) AS sum_value,
    countState() AS sample_count
FROM metrics
GROUP BY time_bucket, service, name, metric_type, labels;

-- Materialized view for hourly metric aggregations
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_hourly_agg
ENGINE = AggregatingMergeTree()
PARTITION BY toYYYYMMDD(hour)
ORDER BY (service, name, hour)
TTL toDateTime(hour) + INTERVAL 365 DAY
AS SELECT
    toStartOfHour(timestamp) AS hour,
    service,
    name,
    metric_type,
    labels,
    avgState(value) AS avg_value,
    minState(value) AS min_value,
    maxState(value) AS max_value,
    sumState(value) AS sum_value,
    countState() AS sample_count
FROM metrics
GROUP BY hour, service, name, metric_type, labels;
