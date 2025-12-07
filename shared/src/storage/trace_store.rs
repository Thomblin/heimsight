//! Trace storage trait and implementations.
//!
//! Provides the `TraceStore` trait for abstracting trace storage operations
//! and an `InMemoryTraceStore` implementation for development and testing.

use crate::models::{Span, SpanStatus, Trace};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors that can occur during trace store operations.
#[derive(Debug, Error)]
pub enum TraceStoreError {
    /// Failed to acquire lock on the store.
    #[error("Failed to acquire lock on trace store")]
    LockError,

    /// Trace not found.
    #[error("Trace not found: {0}")]
    NotFound(String),

    /// Generic storage error.
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Query parameters for retrieving traces.
#[derive(Debug, Clone, Default)]
pub struct TraceQuery {
    /// Filter by service name.
    pub service: Option<String>,

    /// Filter traces starting from this time (inclusive).
    pub start_time: Option<DateTime<Utc>>,

    /// Filter traces up to this time (exclusive).
    pub end_time: Option<DateTime<Utc>>,

    /// Minimum duration in milliseconds.
    pub min_duration_ms: Option<i64>,

    /// Maximum duration in milliseconds.
    pub max_duration_ms: Option<i64>,

    /// Filter by span status.
    pub status: Option<SpanStatus>,

    /// Maximum number of traces to return.
    pub limit: Option<usize>,

    /// Number of traces to skip (for pagination).
    pub offset: Option<usize>,
}

impl TraceQuery {
    /// Creates a new empty query (returns all traces).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the service filter.
    #[must_use]
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Sets the start time filter.
    #[must_use]
    pub fn with_start_time(mut self, start: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self
    }

    /// Sets the end time filter.
    #[must_use]
    pub fn with_end_time(mut self, end: DateTime<Utc>) -> Self {
        self.end_time = Some(end);
        self
    }

    /// Sets the minimum duration filter.
    #[must_use]
    pub fn with_min_duration_ms(mut self, ms: i64) -> Self {
        self.min_duration_ms = Some(ms);
        self
    }

    /// Sets the maximum duration filter.
    #[must_use]
    pub fn with_max_duration_ms(mut self, ms: i64) -> Self {
        self.max_duration_ms = Some(ms);
        self
    }

    /// Sets the status filter.
    #[must_use]
    pub fn with_status(mut self, status: SpanStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets the maximum number of results.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the offset for pagination.
    #[must_use]
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Result of a trace query operation.
#[derive(Debug, Clone)]
pub struct TraceQueryResult {
    /// The traces matching the query.
    pub traces: Vec<Trace>,

    /// Total count of matching traces (before limit/offset applied).
    pub total_count: usize,
}

/// Trait for trace storage implementations.
///
/// This trait defines the interface for storing and querying traces.
/// Implementations must be thread-safe (Send + Sync).
pub trait TraceStore: Send + Sync {
    /// Inserts a single span into the store.
    ///
    /// Spans are grouped by `trace_id` to form traces.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert_span(&self, span: Span) -> Result<(), TraceStoreError>;

    /// Inserts multiple spans into the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert_spans(&self, spans: Vec<Span>) -> Result<(), TraceStoreError>;

    /// Gets a trace by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the trace is not found or the operation fails.
    fn get_trace(&self, trace_id: &str) -> Result<Trace, TraceStoreError>;

    /// Queries traces based on the provided parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the query operation fails.
    fn query(&self, query: TraceQuery) -> Result<TraceQueryResult, TraceStoreError>;

    /// Returns the total number of spans in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the count operation fails.
    fn span_count(&self) -> Result<usize, TraceStoreError>;

    /// Returns the total number of unique traces in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the count operation fails.
    fn trace_count(&self) -> Result<usize, TraceStoreError>;

    /// Clears all traces from the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the clear operation fails.
    fn clear(&self) -> Result<(), TraceStoreError>;
}

/// In-memory trace store implementation.
#[derive(Debug, Default)]
pub struct InMemoryTraceStore {
    /// Spans grouped by `trace_id`.
    spans: Arc<RwLock<HashMap<String, Vec<Span>>>>,
}

impl InMemoryTraceStore {
    /// Creates a new empty in-memory trace store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new in-memory trace store wrapped in an Arc.
    #[must_use]
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl TraceStore for InMemoryTraceStore {
    fn insert_span(&self, span: Span) -> Result<(), TraceStoreError> {
        let mut spans = self.spans.write().map_err(|_| TraceStoreError::LockError)?;
        spans.entry(span.trace_id.clone()).or_default().push(span);
        Ok(())
    }

    fn insert_spans(&self, new_spans: Vec<Span>) -> Result<(), TraceStoreError> {
        let mut spans = self.spans.write().map_err(|_| TraceStoreError::LockError)?;
        for span in new_spans {
            spans.entry(span.trace_id.clone()).or_default().push(span);
        }
        Ok(())
    }

    fn get_trace(&self, trace_id: &str) -> Result<Trace, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;

        spans
            .get(trace_id)
            .and_then(|s| Trace::from_spans(s.clone()))
            .ok_or_else(|| TraceStoreError::NotFound(trace_id.to_string()))
    }

    fn query(&self, query: TraceQuery) -> Result<TraceQueryResult, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;

        let mut traces: Vec<Trace> = spans
            .values()
            .filter_map(|s| Trace::from_spans(s.clone()))
            .filter(|trace| {
                // Service filter
                if let Some(ref service) = query.service {
                    if !trace.spans.iter().any(|s| &s.service == service) {
                        return false;
                    }
                }

                // Time range filter (based on any span in the trace)
                if let Some(start) = query.start_time {
                    if !trace.spans.iter().any(|s| s.start_time >= start) {
                        return false;
                    }
                }
                if let Some(end) = query.end_time {
                    if !trace.spans.iter().any(|s| s.start_time < end) {
                        return false;
                    }
                }

                // Duration filter
                if let Some(duration) = trace.duration() {
                    let duration_ms = duration.num_milliseconds();

                    if let Some(min) = query.min_duration_ms {
                        if duration_ms < min {
                            return false;
                        }
                    }
                    if let Some(max) = query.max_duration_ms {
                        if duration_ms > max {
                            return false;
                        }
                    }
                }

                // Status filter
                if let Some(ref status) = query.status {
                    if !trace.spans.iter().any(|s| &s.status == status) {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort by start time (most recent first)
        traces.sort_by(|a, b| {
            let a_start = a.spans.iter().map(|s| s.start_time).min();
            let b_start = b.spans.iter().map(|s| s.start_time).min();
            b_start.cmp(&a_start)
        });

        let total_count = traces.len();

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let result: Vec<Trace> = traces
            .into_iter()
            .skip(offset)
            .take(query.limit.unwrap_or(usize::MAX))
            .collect();

        Ok(TraceQueryResult {
            traces: result,
            total_count,
        })
    }

    fn span_count(&self) -> Result<usize, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;
        Ok(spans.values().map(std::vec::Vec::len).sum())
    }

    fn trace_count(&self) -> Result<usize, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;
        Ok(spans.len())
    }

    fn clear(&self) -> Result<(), TraceStoreError> {
        let mut spans = self.spans.write().map_err(|_| TraceStoreError::LockError)?;
        spans.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_span(trace_id: &str, span_id: &str, service: &str) -> Span {
        Span::new(trace_id, span_id, "test operation", service)
    }

    #[test]
    fn test_new_store_is_empty() {
        let store = InMemoryTraceStore::new();
        assert_eq!(store.span_count().unwrap(), 0);
        assert_eq!(store.trace_count().unwrap(), 0);
    }

    #[test]
    fn test_insert_span() {
        let store = InMemoryTraceStore::new();
        let span = create_test_span("trace-1", "span-1", "api");

        store.insert_span(span).unwrap();

        assert_eq!(store.span_count().unwrap(), 1);
        assert_eq!(store.trace_count().unwrap(), 1);
    }

    #[test]
    fn test_insert_spans_same_trace() {
        let store = InMemoryTraceStore::new();
        let spans = vec![
            create_test_span("trace-1", "span-1", "api"),
            create_test_span("trace-1", "span-2", "db"),
            create_test_span("trace-1", "span-3", "cache"),
        ];

        store.insert_spans(spans).unwrap();

        assert_eq!(store.span_count().unwrap(), 3);
        assert_eq!(store.trace_count().unwrap(), 1);
    }

    #[test]
    fn test_insert_spans_different_traces() {
        let store = InMemoryTraceStore::new();
        let spans = vec![
            create_test_span("trace-1", "span-1", "api"),
            create_test_span("trace-2", "span-2", "api"),
        ];

        store.insert_spans(spans).unwrap();

        assert_eq!(store.span_count().unwrap(), 2);
        assert_eq!(store.trace_count().unwrap(), 2);
    }

    #[test]
    fn test_get_trace() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-1", "span-2", "db"))
            .unwrap();

        let trace = store.get_trace("trace-1").unwrap();

        assert_eq!(trace.trace_id, "trace-1");
        assert_eq!(trace.span_count(), 2);
    }

    #[test]
    fn test_get_trace_not_found() {
        let store = InMemoryTraceStore::new();

        let result = store.get_trace("nonexistent");

        assert!(matches!(result, Err(TraceStoreError::NotFound(_))));
    }

    #[test]
    fn test_query_all_traces() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-2", "span-2", "db"))
            .unwrap();

        let result = store.query(TraceQuery::new()).unwrap();

        assert_eq!(result.total_count, 2);
        assert_eq!(result.traces.len(), 2);
    }

    #[test]
    fn test_query_by_service() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-2", "span-2", "db"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-3", "span-3", "api"))
            .unwrap();

        let result = store.query(TraceQuery::new().with_service("api")).unwrap();

        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_query_by_duration() {
        let store = InMemoryTraceStore::new();

        // Fast trace (10ms)
        let fast = Span::new("trace-1", "span-1", "fast", "api")
            .with_start_time(Utc::now())
            .with_end_time(Utc::now() + Duration::milliseconds(10));
        store.insert_span(fast).unwrap();

        // Slow trace (500ms)
        let slow = Span::new("trace-2", "span-2", "slow", "api")
            .with_start_time(Utc::now())
            .with_end_time(Utc::now() + Duration::milliseconds(500));
        store.insert_span(slow).unwrap();

        let result = store
            .query(TraceQuery::new().with_min_duration_ms(100))
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.traces[0].trace_id, "trace-2");
    }

    #[test]
    fn test_query_by_status() {
        let store = InMemoryTraceStore::new();

        store
            .insert_span(
                Span::new("trace-1", "span-1", "success", "api").with_status(SpanStatus::Ok),
            )
            .unwrap();
        store
            .insert_span(
                Span::new("trace-2", "span-2", "failure", "api").with_status(SpanStatus::Error),
            )
            .unwrap();

        let result = store
            .query(TraceQuery::new().with_status(SpanStatus::Error))
            .unwrap();

        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn test_query_with_limit() {
        let store = InMemoryTraceStore::new();

        for i in 0..10 {
            store
                .insert_span(create_test_span(
                    &format!("trace-{i}"),
                    &format!("span-{i}"),
                    "api",
                ))
                .unwrap();
        }

        let result = store.query(TraceQuery::new().with_limit(5)).unwrap();

        assert_eq!(result.traces.len(), 5);
        assert_eq!(result.total_count, 10);
    }

    #[test]
    fn test_clear_store() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();

        store.clear().unwrap();

        assert_eq!(store.span_count().unwrap(), 0);
        assert_eq!(store.trace_count().unwrap(), 0);
    }
}
