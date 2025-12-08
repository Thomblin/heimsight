//! Storage traits and implementations.
//!
//! This module provides abstractions for storing and querying observability data.
//! The `LogStore` trait defines the interface for log storage, allowing different
//! implementations (in-memory, database-backed, etc.).

pub mod log_store;
pub mod metric_store;
pub mod trace_store;

pub use log_store::{
    ClickHouseLogStore, InMemoryLogStore, LogQuery, LogQueryResult, LogStore, LogStoreError,
};
pub use metric_store::{
    AggregationFunction, AggregationResult, ClickHouseMetricStore, InMemoryMetricStore,
    MetricQuery, MetricQueryResult, MetricStore, MetricStoreError,
};
pub use trace_store::{
    ClickHouseTraceStore, InMemoryTraceStore, TraceQuery, TraceQueryResult, TraceStore,
    TraceStoreError,
};
