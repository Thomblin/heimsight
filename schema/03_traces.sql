-- Traces (spans) table for Heimsight
-- Optimized for distributed trace storage and querying

USE heimsight;

-- Note: Timestamps stored as Int64 (nanoseconds) for Rust client compatibility
-- The clickhouse Rust crate has issues with DateTime64(9) deserialization
CREATE TABLE IF NOT EXISTS spans (
    -- Trace identification
    trace_id String NOT NULL,
    span_id String NOT NULL,
    parent_span_id String DEFAULT '',

    -- Timestamps (Int64 nanoseconds for Rust client compatibility)
    start_time Int64 NOT NULL,
    end_time Int64 NOT NULL,
    duration_ns UInt64 NOT NULL,

    -- Span metadata
    name String NOT NULL,
    span_kind LowCardinality(String) DEFAULT 'INTERNAL',
    service LowCardinality(String) NOT NULL,
    operation String NOT NULL,

    -- Status
    status_code LowCardinality(String) DEFAULT 'OK',
    status_message String DEFAULT '',

    -- Attributes as Map for flexible data
    attributes Map(String, String) DEFAULT map(),

    -- Resource attributes (deployment environment, host, etc.)
    resource_attributes Map(String, String) DEFAULT map(),

    -- Events (span events like exceptions, logs)
    events Array(Tuple(
        Int64,  -- timestamp (nanoseconds)
        String, -- name
        Map(String, String) -- attributes
    )) DEFAULT [],

    -- Links to other spans
    links Array(Tuple(
        String, -- trace_id
        String, -- span_id
        Map(String, String) -- attributes
    )) DEFAULT [],

    -- Indexes for efficient querying
    INDEX idx_trace_id trace_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_span_id span_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_parent_span_id parent_span_id TYPE bloom_filter GRANULARITY 1,
    INDEX idx_name name TYPE tokenbf_v1(32768, 3, 0) GRANULARITY 1
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(toDateTime(start_time / 1000000000))
ORDER BY (service, trace_id, start_time, span_id)
TTL toDateTime(start_time / 1000000000) + INTERVAL 30 DAY
SETTINGS index_granularity = 8192;
