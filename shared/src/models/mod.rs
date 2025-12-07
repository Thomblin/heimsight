//! Data models for the Heimsight observability platform.
//!
//! This module contains the core data structures for logs, metrics, and traces.

pub mod log;
pub mod metric;
pub mod trace;

pub use log::{LogEntry, LogLevel, LogValidationError};
pub use metric::{
    HistogramBucket, HistogramData, Metric, MetricType, MetricValidationError, MetricValue,
};
pub use trace::{Span, SpanEvent, SpanKind, SpanStatus, SpanValidationError, Trace};
