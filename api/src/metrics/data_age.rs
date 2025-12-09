//! Data age monitoring and metrics.
//!
//! Tracks and reports the age of data in the system for retention monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::config::DataType;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::state::AppState;

/// Statistics about data age for a specific data type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataAgeStats {
    /// The data type these statistics apply to.
    pub data_type: DataType,
    /// Timestamp of the oldest data point (if any).
    pub oldest: Option<DateTime<Utc>>,
    /// Timestamp of the newest data point (if any).
    pub newest: Option<DateTime<Utc>>,
    /// Total count of data points.
    pub count: u64,
    /// Age of oldest data in days (if any).
    pub oldest_age_days: Option<f64>,
}

impl DataAgeStats {
    /// Creates new data age statistics.
    #[must_use]
    pub fn new(
        data_type: DataType,
        oldest: Option<DateTime<Utc>>,
        newest: Option<DateTime<Utc>>,
        count: u64,
    ) -> Self {
        let oldest_age_days = oldest.map(|dt| {
            let duration = Utc::now().signed_duration_since(dt);
            // Cast is acceptable here: precision loss is negligible for day-level granularity
            #[allow(clippy::cast_precision_loss)]
            let age_days = duration.num_seconds() as f64 / 86400.0;
            age_days
        });

        Self {
            data_type,
            oldest,
            newest,
            count,
            oldest_age_days,
        }
    }

    /// Returns true if this data type has any data.
    #[must_use]
    pub fn has_data(&self) -> bool {
        self.count > 0
    }

    /// Returns true if the oldest data exceeds the given TTL.
    #[must_use]
    pub fn exceeds_ttl(&self, ttl_days: u32) -> bool {
        match self.oldest_age_days {
            Some(age) => age > f64::from(ttl_days),
            None => false,
        }
    }
}

/// Complete metrics about data age across all data types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataAgeMetrics {
    /// Statistics for logs.
    pub logs: DataAgeStats,
    /// Statistics for metrics.
    pub metrics: DataAgeStats,
    /// Statistics for traces.
    pub traces: DataAgeStats,
    /// When these metrics were collected.
    pub collected_at: DateTime<Utc>,
}

impl DataAgeMetrics {
    /// Creates new data age metrics.
    #[must_use]
    pub fn new(logs: DataAgeStats, metrics: DataAgeStats, traces: DataAgeStats) -> Self {
        Self {
            logs,
            metrics,
            traces,
            collected_at: Utc::now(),
        }
    }

    /// Gets statistics for a specific data type.
    #[must_use]
    pub fn get_stats(&self, data_type: DataType) -> &DataAgeStats {
        match data_type {
            DataType::Logs => &self.logs,
            DataType::Metrics => &self.metrics,
            DataType::Traces => &self.traces,
        }
    }
}

/// Background monitor for data age metrics.
pub struct DataAgeMonitor {
    state: AppState,
    interval_duration: Duration,
}

impl DataAgeMonitor {
    /// Creates a new data age monitor.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state for accessing stores
    /// * `interval_duration` - How often to collect metrics
    #[must_use]
    pub fn new(state: AppState, interval_duration: Duration) -> Self {
        Self {
            state,
            interval_duration,
        }
    }

    /// Collects current data age metrics by querying all stores.
    ///
    /// # Errors
    ///
    /// Returns an error if any store query fails.
    pub fn collect_metrics(&self) -> anyhow::Result<DataAgeMetrics> {
        // Collect logs statistics
        let log_count = self.state.log_store().count()?;
        let (logs_oldest, logs_newest) = if log_count == 0 {
            (None, None)
        } else {
            (
                self.state.log_store().get_oldest_timestamp()?,
                self.state.log_store().get_newest_timestamp()?,
            )
        };
        let logs_stats =
            DataAgeStats::new(DataType::Logs, logs_oldest, logs_newest, log_count as u64);

        // Collect metrics statistics
        let metric_count = self.state.metric_store().count()?;
        let (metrics_oldest, metrics_newest) = if metric_count == 0 {
            (None, None)
        } else {
            (
                self.state.metric_store().get_oldest_timestamp()?,
                self.state.metric_store().get_newest_timestamp()?,
            )
        };
        let metrics_stats = DataAgeStats::new(
            DataType::Metrics,
            metrics_oldest,
            metrics_newest,
            metric_count as u64,
        );

        // Collect traces statistics
        let trace_count = self.state.trace_store().span_count()?;
        let (traces_oldest, traces_newest) = if trace_count == 0 {
            (None, None)
        } else {
            (
                self.state.trace_store().get_oldest_timestamp()?,
                self.state.trace_store().get_newest_timestamp()?,
            )
        };
        let traces_stats = DataAgeStats::new(
            DataType::Traces,
            traces_oldest,
            traces_newest,
            trace_count as u64,
        );

        Ok(DataAgeMetrics::new(logs_stats, metrics_stats, traces_stats))
    }

    /// Starts the monitoring loop.
    ///
    /// This function runs indefinitely, collecting metrics at the configured interval.
    /// It logs the metrics using the tracing infrastructure.
    ///
    /// # Cancellation
    ///
    /// This function runs until cancelled via the task handle.
    pub async fn run(self: Arc<Self>) {
        let mut tick = interval(self.interval_duration);

        loop {
            tick.tick().await;

            match self.collect_metrics() {
                Ok(metrics) => {
                    // Log metrics for observability
                    tracing::info!(
                        logs_count = metrics.logs.count,
                        logs_oldest_age_days = metrics.logs.oldest_age_days,
                        metrics_count = metrics.metrics.count,
                        metrics_oldest_age_days = metrics.metrics.oldest_age_days,
                        traces_count = metrics.traces.count,
                        traces_oldest_age_days = metrics.traces.oldest_age_days,
                        "Data age metrics collected"
                    );

                    // Check against configured retention policies
                    let config = self.state.get_retention_config();

                    if metrics.logs.exceeds_ttl(config.logs.ttl_days) {
                        tracing::warn!(
                            age_days = metrics.logs.oldest_age_days,
                            ttl_days = config.logs.ttl_days,
                            "Logs data exceeds configured TTL (ClickHouse should auto-delete)"
                        );
                    }

                    if metrics.metrics.exceeds_ttl(config.metrics.ttl_days) {
                        tracing::warn!(
                            age_days = metrics.metrics.oldest_age_days,
                            ttl_days = config.metrics.ttl_days,
                            "Metrics data exceeds configured TTL (ClickHouse should auto-delete)"
                        );
                    }

                    if metrics.traces.exceeds_ttl(config.traces.ttl_days) {
                        tracing::warn!(
                            age_days = metrics.traces.oldest_age_days,
                            ttl_days = config.traces.ttl_days,
                            "Traces data exceeds configured TTL (ClickHouse should auto-delete)"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to collect data age metrics");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn test_data_age_stats_new() {
        let oldest = Some(Utc::now() - ChronoDuration::days(30));
        let newest = Some(Utc::now());
        let stats = DataAgeStats::new(DataType::Logs, oldest, newest, 1000);

        assert_eq!(stats.data_type, DataType::Logs);
        assert_eq!(stats.count, 1000);
        assert!(stats.oldest.is_some());
        assert!(stats.newest.is_some());
        assert!(stats.oldest_age_days.is_some());

        // Age should be approximately 30 days
        let age = stats.oldest_age_days.unwrap();
        assert!(age >= 29.9 && age <= 30.1);
    }

    #[test]
    fn test_data_age_stats_no_data() {
        let stats = DataAgeStats::new(DataType::Logs, None, None, 0);

        assert_eq!(stats.data_type, DataType::Logs);
        assert_eq!(stats.count, 0);
        assert!(stats.oldest.is_none());
        assert!(stats.newest.is_none());
        assert!(stats.oldest_age_days.is_none());
        assert!(!stats.has_data());
    }

    #[test]
    fn test_data_age_stats_has_data() {
        let stats = DataAgeStats::new(DataType::Logs, None, None, 100);
        assert!(stats.has_data());

        let stats_empty = DataAgeStats::new(DataType::Logs, None, None, 0);
        assert!(!stats_empty.has_data());
    }

    #[test]
    fn test_data_age_stats_exceeds_ttl() {
        let oldest = Some(Utc::now() - ChronoDuration::days(60));
        let stats = DataAgeStats::new(DataType::Logs, oldest, None, 100);

        assert!(stats.exceeds_ttl(30)); // 60 days > 30 day TTL
        assert!(stats.exceeds_ttl(50)); // 60 days > 50 day TTL
        assert!(!stats.exceeds_ttl(90)); // 60 days < 90 day TTL
    }

    #[test]
    fn test_data_age_stats_exceeds_ttl_no_data() {
        let stats = DataAgeStats::new(DataType::Logs, None, None, 0);
        assert!(!stats.exceeds_ttl(30)); // No data, doesn't exceed TTL
    }

    #[test]
    fn test_data_age_metrics_new() {
        let logs = DataAgeStats::new(DataType::Logs, None, None, 100);
        let metrics = DataAgeStats::new(DataType::Metrics, None, None, 200);
        let traces = DataAgeStats::new(DataType::Traces, None, None, 300);

        let age_metrics = DataAgeMetrics::new(logs.clone(), metrics.clone(), traces.clone());

        assert_eq!(age_metrics.logs, logs);
        assert_eq!(age_metrics.metrics, metrics);
        assert_eq!(age_metrics.traces, traces);
        assert!(age_metrics.collected_at <= Utc::now());
    }

    #[test]
    fn test_data_age_metrics_get_stats() {
        let logs = DataAgeStats::new(DataType::Logs, None, None, 100);
        let metrics = DataAgeStats::new(DataType::Metrics, None, None, 200);
        let traces = DataAgeStats::new(DataType::Traces, None, None, 300);

        let age_metrics = DataAgeMetrics::new(logs.clone(), metrics.clone(), traces.clone());

        assert_eq!(age_metrics.get_stats(DataType::Logs), &logs);
        assert_eq!(age_metrics.get_stats(DataType::Metrics), &metrics);
        assert_eq!(age_metrics.get_stats(DataType::Traces), &traces);
    }

    #[test]
    fn test_data_age_monitor_creation() {
        let state = AppState::with_in_memory_store();
        let monitor = DataAgeMonitor::new(state, Duration::from_secs(60));

        assert_eq!(monitor.interval_duration, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_data_age_monitor_collect_metrics_empty() {
        let state = AppState::with_in_memory_store();
        let monitor = DataAgeMonitor::new(state, Duration::from_secs(60));

        let metrics = monitor.collect_metrics().unwrap();

        // Verify counts are zero
        assert_eq!(metrics.logs.count, 0);
        assert_eq!(metrics.metrics.count, 0);
        assert_eq!(metrics.traces.count, 0);

        // Verify oldest and newest are None when count is zero
        assert!(metrics.logs.oldest.is_none());
        assert!(metrics.logs.newest.is_none());
        assert!(metrics.metrics.oldest.is_none());
        assert!(metrics.metrics.newest.is_none());
        assert!(metrics.traces.oldest.is_none());
        assert!(metrics.traces.newest.is_none());
    }

    #[tokio::test]
    async fn test_data_age_monitor_collect_metrics_with_data() {
        use shared::models::{LogEntry, LogLevel, Metric, Span};

        let state = AppState::with_in_memory_store();

        // Add some data
        let log = LogEntry::new(LogLevel::Info, "Test", "service");
        state.log_store().insert(log).unwrap();

        let metric = Metric::gauge("test", 1.0);
        state.metric_store().insert(metric).unwrap();

        let span = Span::new("trace1", "span1", "test", "service");
        state.trace_store().insert_span(span).unwrap();

        let monitor = DataAgeMonitor::new(state, Duration::from_secs(60));
        let metrics = monitor.collect_metrics().unwrap();

        assert_eq!(metrics.logs.count, 1);
        assert_eq!(metrics.metrics.count, 1);
        assert_eq!(metrics.traces.count, 1);
    }

    #[test]
    fn test_data_age_stats_serialization() {
        let oldest = Some(Utc::now() - ChronoDuration::days(30));
        let newest = Some(Utc::now());
        let stats = DataAgeStats::new(DataType::Logs, oldest, newest, 1000);

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: DataAgeStats = serde_json::from_str(&json).unwrap();

        assert_eq!(stats.data_type, deserialized.data_type);
        assert_eq!(stats.count, deserialized.count);
    }

    #[test]
    fn test_data_age_metrics_serialization() {
        let logs = DataAgeStats::new(DataType::Logs, None, None, 100);
        let metrics = DataAgeStats::new(DataType::Metrics, None, None, 200);
        let traces = DataAgeStats::new(DataType::Traces, None, None, 300);

        let age_metrics = DataAgeMetrics::new(logs, metrics, traces);

        let json = serde_json::to_string(&age_metrics).unwrap();
        let deserialized: DataAgeMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(age_metrics.logs.count, deserialized.logs.count);
        assert_eq!(age_metrics.metrics.count, deserialized.metrics.count);
        assert_eq!(age_metrics.traces.count, deserialized.traces.count);
    }
}
