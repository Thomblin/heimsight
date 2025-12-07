//! Application state module.
//!
//! Defines the shared application state that is passed to route handlers.

use shared::storage::{
    InMemoryLogStore, InMemoryMetricStore, InMemoryTraceStore, LogStore, MetricStore, TraceStore,
};
use std::sync::Arc;

/// Application state shared across all request handlers.
///
/// This struct contains all the shared resources needed by the API,
/// such as storage backends and configuration.
#[derive(Clone)]
pub struct AppState {
    /// The log storage backend.
    log_store: Arc<dyn LogStore>,
    /// The metric storage backend.
    metric_store: Arc<dyn MetricStore>,
    /// The trace storage backend.
    trace_store: Arc<dyn TraceStore>,
}

impl AppState {
    /// Creates a new application state with the given stores.
    pub fn new(
        log_store: Arc<dyn LogStore>,
        metric_store: Arc<dyn MetricStore>,
        trace_store: Arc<dyn TraceStore>,
    ) -> Self {
        Self {
            log_store,
            metric_store,
            trace_store,
        }
    }

    /// Creates a new application state with in-memory stores.
    ///
    /// This is useful for development and testing.
    #[must_use]
    pub fn with_in_memory_store() -> Self {
        Self {
            log_store: Arc::new(InMemoryLogStore::new()),
            metric_store: Arc::new(InMemoryMetricStore::new()),
            trace_store: Arc::new(InMemoryTraceStore::new()),
        }
    }

    /// Returns a reference to the log store.
    #[must_use]
    pub fn log_store(&self) -> &dyn LogStore {
        self.log_store.as_ref()
    }

    /// Returns a reference to the metric store.
    #[must_use]
    pub fn metric_store(&self) -> &dyn MetricStore {
        self.metric_store.as_ref()
    }

    /// Returns a reference to the trace store.
    #[must_use]
    pub fn trace_store(&self) -> &dyn TraceStore {
        self.trace_store.as_ref()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::with_in_memory_store()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::{LogEntry, LogLevel, Metric, Span};

    #[test]
    fn test_app_state_with_in_memory_store() {
        let state = AppState::with_in_memory_store();

        // Verify we can use all stores
        let log = LogEntry::new(LogLevel::Info, "Test", "test-service");
        state.log_store().insert(log).unwrap();
        assert_eq!(state.log_store().count().unwrap(), 1);

        let metric = Metric::gauge("test_metric", 42.0);
        state.metric_store().insert(metric).unwrap();
        assert_eq!(state.metric_store().count().unwrap(), 1);

        let span = Span::new("trace-1", "span-1", "test", "service");
        state.trace_store().insert_span(span).unwrap();
        assert_eq!(state.trace_store().span_count().unwrap(), 1);
    }

    #[test]
    fn test_app_state_is_clone() {
        let state = AppState::with_in_memory_store();
        let state2 = state.clone();

        // Both should share the same stores
        let log = LogEntry::new(LogLevel::Info, "Test", "test-service");
        state.log_store().insert(log).unwrap();

        assert_eq!(state2.log_store().count().unwrap(), 1);
    }
}
