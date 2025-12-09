//! Application state module.
//!
//! Defines the shared application state that is passed to route handlers.

use shared::config::RetentionConfig;
use shared::storage::{
    ClickHouseLogStore, ClickHouseMetricStore, ClickHouseTraceStore, InMemoryLogStore,
    InMemoryMetricStore, InMemoryTraceStore, LogStore, MetricStore, TraceStore,
};
use std::sync::{Arc, RwLock};

/// Application state shared across all request handlers.
///
/// This struct contains all the shared resources needed by the API,
/// such as storage backends and configuration.
#[derive(Clone)]
#[allow(clippy::struct_field_names)]
pub struct AppState {
    /// The log storage backend.
    log_store: Arc<dyn LogStore>,
    /// The metric storage backend.
    metric_store: Arc<dyn MetricStore>,
    /// The trace storage backend.
    trace_store: Arc<dyn TraceStore>,
    /// Retention configuration (TTL policies).
    retention_config: Arc<RwLock<RetentionConfig>>,
    /// Optional `ClickHouse` client for direct database operations.
    clickhouse_client: Option<Arc<clickhouse::Client>>,
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
            retention_config: Arc::new(RwLock::new(RetentionConfig::default())),
            clickhouse_client: None,
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
            retention_config: Arc::new(RwLock::new(RetentionConfig::default())),
            clickhouse_client: None,
        }
    }

    /// Creates a new application state with ClickHouse-backed stores.
    ///
    /// This is used for production deployments with persistent storage.
    #[must_use]
    pub fn with_clickhouse_store(client: Arc<clickhouse::Client>) -> Self {
        Self {
            log_store: Arc::new(ClickHouseLogStore::new(Arc::clone(&client))),
            metric_store: Arc::new(ClickHouseMetricStore::new(Arc::clone(&client))),
            trace_store: Arc::new(ClickHouseTraceStore::new(Arc::clone(&client))),
            retention_config: Arc::new(RwLock::new(RetentionConfig::default())),
            clickhouse_client: Some(client),
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

    /// Gets the current retention configuration.
    ///
    /// # Panics
    ///
    /// Panics if the retention config lock is poisoned.
    #[must_use]
    pub fn get_retention_config(&self) -> RetentionConfig {
        self.retention_config
            .read()
            .expect("Retention config lock poisoned")
            .clone()
    }

    /// Sets the retention configuration.
    ///
    /// # Panics
    ///
    /// Panics if the retention config lock is poisoned.
    pub fn set_retention_config(&self, config: RetentionConfig) {
        *self
            .retention_config
            .write()
            .expect("Retention config lock poisoned") = config;
    }

    /// Returns a reference to the `ClickHouse` client, if available.
    ///
    /// This is `None` when using in-memory stores.
    #[must_use]
    pub fn clickhouse_client(&self) -> Option<&Arc<clickhouse::Client>> {
        self.clickhouse_client.as_ref()
    }

    /// Updates `ClickHouse` TTL policies to match the retention configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or no `ClickHouse` client is available.
    pub async fn update_clickhouse_ttl(&self, config: &RetentionConfig) -> anyhow::Result<()> {
        let client = self
            .clickhouse_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ClickHouse client not available"))?;

        // Update logs table TTL
        let logs_sql = format!(
            "ALTER TABLE logs MODIFY TTL toDateTime(timestamp / 1000000000) + INTERVAL {} DAY",
            config.logs.ttl_days
        );
        client.query(&logs_sql).execute().await?;

        // Update metrics table TTL
        let metrics_sql = format!(
            "ALTER TABLE metrics MODIFY TTL toDateTime(timestamp / 1000000000) + INTERVAL {} DAY",
            config.metrics.ttl_days
        );
        client.query(&metrics_sql).execute().await?;

        // Update spans table TTL
        let traces_sql = format!(
            "ALTER TABLE spans MODIFY TTL toDateTime(start_time / 1000000000) + INTERVAL {} DAY",
            config.traces.ttl_days
        );
        client.query(&traces_sql).execute().await?;

        tracing::info!(
            logs_ttl_days = config.logs.ttl_days,
            metrics_ttl_days = config.metrics.ttl_days,
            traces_ttl_days = config.traces.ttl_days,
            "Updated ClickHouse TTL policies"
        );

        Ok(())
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
