-- Traces (spans) table for Heimsight
-- Optimized for distributed trace storage and querying

USE heimsight;

CREATE TABLE IF NOT EXISTS spans (
    -- Trace identification
    trace_id String NOT NULL,
    span_id String NOT NULL,
    parent_span_id String DEFAULT '',

    -- Timestamps
    start_time DateTime64(9) NOT NULL,
    end_time DateTime64(9) NOT NULL,
    duration_ns UInt64 NOT NULL, -- Computed: (end_time - start_time) in nanoseconds

    -- Span metadata
    name String NOT NULL,
    span_kind LowCardinality(String) DEFAULT 'INTERNAL', -- INTERNAL, SERVER, CLIENT, PRODUCER, CONSUMER
    service LowCardinality(String) NOT NULL,
    operation String NOT NULL,

    -- Status
    status_code LowCardinality(String) DEFAULT 'OK', -- OK, ERROR, UNSET
    status_message String DEFAULT '',

    -- Attributes as Map for flexible data
    attributes Map(String, String) DEFAULT map(),

    -- Resource attributes (deployment environment, host, etc.)
    resource_attributes Map(String, String) DEFAULT map(),

    -- Events (span events like exceptions, logs)
    events Array(Tuple(
        timestamp DateTime64(9),
        name String,
        attributes Map(String, String)
    )) DEFAULT [],

    -- Links to other spans
    links Array(Tuple(
        trace_id String,
        span_id String,
        attributes Map(String, String)
    )) DEFAULT [],

    -- Indexes for efficient querying
    INDEX idx_trace_id trace_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_span_id span_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_parent_span_id parent_span_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_name name TYPE tokenbf_v1(32768, 3, 0) GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(start_time)
ORDER BY (service, trace_id, start_time, span_id)
TTL toDateTime(start_time) + INTERVAL 30 DAY
SETTINGS index_granularity = 8192;

-- Materialized view for trace-level aggregations
CREATE MATERIALIZED VIEW IF NOT EXISTS trace_stats
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMMDD(trace_start_time)
ORDER BY (service, trace_id)
TTL toDateTime(trace_start_time) + INTERVAL 30 DAY
AS SELECT
    trace_id,
    service,
    min(start_time) AS trace_start_time,
    max(end_time) AS trace_end_time,
    toUnixTimestamp64Nano(max(end_time)) - toUnixTimestamp64Nano(min(start_time)) AS total_duration_ns,
    count() AS span_count,
    countIf(status_code = 'ERROR') AS error_count,
    groupArray(name) AS span_names
FROM spans
GROUP BY trace_id, service;

-- Materialized view for service-level performance stats
CREATE MATERIALIZED VIEW IF NOT EXISTS span_performance_hourly
ENGINE = AggregatingMergeTree()
PARTITION BY toYYYYMMDD(hour)
ORDER BY (service, operation, hour)
TTL toDateTime(hour) + INTERVAL 90 DAY
AS SELECT
    toStartOfHour(start_time) AS hour,
    service,
    operation,
    span_kind,
    status_code,
    countState() AS span_count,
    avgState(duration_ns) AS avg_duration_ns,
    quantilesState(0.5, 0.95, 0.99)(duration_ns) AS duration_quantiles
FROM spans
GROUP BY hour, service, operation, span_kind, status_code;
