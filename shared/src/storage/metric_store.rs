//! Metric storage trait and implementations.
//!
//! Provides the `MetricStore` trait for abstracting metric storage operations
//! and an `InMemoryMetricStore` implementation for development and testing.

use crate::models::{Metric, MetricType};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors that can occur during metric store operations.
#[derive(Debug, Error)]
pub enum MetricStoreError {
    /// Failed to acquire lock on the store.
    #[error("Failed to acquire lock on metric store")]
    LockError,

    /// Generic storage error.
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Query parameters for retrieving metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricQuery {
    /// Filter by metric name.
    pub name: Option<String>,

    /// Filter by metric type.
    pub metric_type: Option<MetricType>,

    /// Filter metrics starting from this time (inclusive).
    pub start_time: Option<DateTime<Utc>>,

    /// Filter metrics up to this time (exclusive).
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by labels (all must match).
    pub labels: HashMap<String, String>,

    /// Maximum number of metrics to return.
    pub limit: Option<usize>,

    /// Number of metrics to skip (for pagination).
    pub offset: Option<usize>,
}

impl MetricQuery {
    /// Creates a new empty query (returns all metrics).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the metric name filter.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the metric type filter.
    #[must_use]
    pub fn with_type(mut self, metric_type: MetricType) -> Self {
        self.metric_type = Some(metric_type);
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

    /// Adds a label filter.
    #[must_use]
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
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

/// Result of a metric query operation.
#[derive(Debug, Clone)]
pub struct MetricQueryResult {
    /// The metrics matching the query.
    pub metrics: Vec<Metric>,

    /// Total count of matching metrics (before limit/offset applied).
    pub total_count: usize,
}

/// Aggregation function for metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationFunction {
    /// Sum of values.
    Sum,
    /// Average of values.
    Avg,
    /// Minimum value.
    Min,
    /// Maximum value.
    Max,
    /// Count of values.
    Count,
}

/// Result of an aggregation operation.
#[derive(Debug, Clone)]
pub struct AggregationResult {
    /// The aggregated value.
    pub value: f64,
    /// Number of data points in the aggregation.
    pub count: usize,
}

/// Trait for metric storage implementations.
///
/// This trait defines the interface for storing and querying metrics.
/// Implementations must be thread-safe (Send + Sync).
pub trait MetricStore: Send + Sync {
    /// Inserts a single metric into the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert(&self, metric: Metric) -> Result<(), MetricStoreError>;

    /// Inserts multiple metrics into the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert_batch(&self, metrics: Vec<Metric>) -> Result<(), MetricStoreError>;

    /// Queries metrics based on the provided parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the query operation fails.
    fn query(&self, query: MetricQuery) -> Result<MetricQueryResult, MetricStoreError>;

    /// Returns the total number of metrics in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the count operation fails.
    fn count(&self) -> Result<usize, MetricStoreError>;

    /// Clears all metrics from the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the clear operation fails.
    fn clear(&self) -> Result<(), MetricStoreError>;

    /// Aggregates metrics matching the query.
    ///
    /// # Errors
    ///
    /// Returns an error if the aggregation operation fails.
    fn aggregate(
        &self,
        query: MetricQuery,
        function: AggregationFunction,
    ) -> Result<AggregationResult, MetricStoreError>;
}

/// In-memory metric store implementation.
#[derive(Debug, Default)]
pub struct InMemoryMetricStore {
    metrics: Arc<RwLock<Vec<Metric>>>,
}

impl InMemoryMetricStore {
    /// Creates a new empty in-memory metric store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Creates a new in-memory metric store wrapped in an Arc.
    #[must_use]
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl MetricStore for InMemoryMetricStore {
    fn insert(&self, metric: Metric) -> Result<(), MetricStoreError> {
        let mut metrics = self
            .metrics
            .write()
            .map_err(|_| MetricStoreError::LockError)?;
        metrics.push(metric);
        Ok(())
    }

    fn insert_batch(&self, new_metrics: Vec<Metric>) -> Result<(), MetricStoreError> {
        let mut metrics = self
            .metrics
            .write()
            .map_err(|_| MetricStoreError::LockError)?;
        metrics.extend(new_metrics);
        Ok(())
    }

    fn query(&self, query: MetricQuery) -> Result<MetricQueryResult, MetricStoreError> {
        let metrics = self
            .metrics
            .read()
            .map_err(|_| MetricStoreError::LockError)?;

        let filtered: Vec<Metric> = metrics
            .iter()
            .filter(|m| {
                // Name filter
                if let Some(ref name) = query.name {
                    if &m.name != name {
                        return false;
                    }
                }

                // Type filter
                if let Some(ref metric_type) = query.metric_type {
                    if &m.metric_type != metric_type {
                        return false;
                    }
                }

                // Time range filter
                if let Some(start) = query.start_time {
                    if m.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = query.end_time {
                    if m.timestamp >= end {
                        return false;
                    }
                }

                // Label filters (all must match)
                for (key, value) in &query.labels {
                    match m.labels.get(key) {
                        Some(v) if v == value => {}
                        _ => return false,
                    }
                }

                true
            })
            .cloned()
            .collect();

        let total_count = filtered.len();

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let result: Vec<Metric> = filtered
            .into_iter()
            .skip(offset)
            .take(query.limit.unwrap_or(usize::MAX))
            .collect();

        Ok(MetricQueryResult {
            metrics: result,
            total_count,
        })
    }

    fn count(&self) -> Result<usize, MetricStoreError> {
        let metrics = self
            .metrics
            .read()
            .map_err(|_| MetricStoreError::LockError)?;
        Ok(metrics.len())
    }

    fn clear(&self) -> Result<(), MetricStoreError> {
        let mut metrics = self
            .metrics
            .write()
            .map_err(|_| MetricStoreError::LockError)?;
        metrics.clear();
        Ok(())
    }

    fn aggregate(
        &self,
        query: MetricQuery,
        function: AggregationFunction,
    ) -> Result<AggregationResult, MetricStoreError> {
        let result = self.query(query)?;

        let values: Vec<f64> = result
            .metrics
            .iter()
            .filter_map(super::super::models::metric::Metric::simple_value)
            .collect();

        if values.is_empty() {
            return Ok(AggregationResult {
                value: 0.0,
                count: 0,
            });
        }

        #[allow(clippy::cast_precision_loss)]
        let value = match function {
            AggregationFunction::Sum => values.iter().sum(),
            AggregationFunction::Avg => values.iter().sum::<f64>() / values.len() as f64,
            AggregationFunction::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
            AggregationFunction::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            AggregationFunction::Count => values.len() as f64,
        };

        Ok(AggregationResult {
            value,
            count: values.len(),
        })
    }
}

/// `ClickHouse`-backed metric store implementation.
///
/// This implementation stores metrics in `ClickHouse` for production use.
/// It provides persistent storage and efficient time-series queries with aggregation.
#[derive(Clone)]
pub struct ClickHouseMetricStore {
    client: Arc<clickhouse::Client>,
}

impl ClickHouseMetricStore {
    /// Creates a new `ClickHouse` metric store with the given client.
    #[must_use]
    pub fn new(client: Arc<clickhouse::Client>) -> Self {
        Self { client }
    }

    /// Creates a new `ClickHouse` metric store wrapped in an Arc.
    #[must_use]
    pub fn new_shared(client: Arc<clickhouse::Client>) -> Arc<Self> {
        Arc::new(Self::new(client))
    }

    /// Helper to execute async operations synchronously.
    fn block_on<F, T>(future: F) -> Result<T, MetricStoreError>
    where
        F: std::future::Future<Output = Result<T, clickhouse::error::Error>>,
    {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(future)
                .map_err(|e| MetricStoreError::StorageError(e.to_string()))
        })
    }
}

impl MetricStore for ClickHouseMetricStore {
    fn insert(&self, metric: Metric) -> Result<(), MetricStoreError> {
        self.insert_batch(vec![metric])
    }

    fn insert_batch(&self, metrics: Vec<Metric>) -> Result<(), MetricStoreError> {
        if metrics.is_empty() {
            return Ok(());
        }

        let client = Arc::clone(&self.client);
        Self::block_on(async move {
            #[derive(clickhouse::Row, serde::Serialize)]
            struct MetricRow {
                timestamp: i64,
                name: String,
                metric_type: String,
                value: f64,
                labels: HashMap<String, String>,
                service: String,
                bucket_counts: Vec<u64>,
                bucket_bounds: Vec<f64>,
                quantile_values: Vec<f64>,
                quantiles: Vec<f64>,
            }

            let mut inserter = client.insert::<MetricRow>("metrics").await?;

            for metric in metrics {
                // Determine which service to use (from labels or default)
                let service = metric
                    .labels
                    .get("service")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                // Extract simple value or histogram data
                let (value, bucket_counts, bucket_bounds) = match &metric.value {
                    crate::models::metric::MetricValue::Simple(v) => {
                        (*v, Vec::<u64>::new(), Vec::<f64>::new())
                    }
                    crate::models::metric::MetricValue::Histogram(hist) => {
                        let counts: Vec<u64> = hist.buckets.iter().map(|b| b.count).collect();
                        let bounds: Vec<f64> = hist.buckets.iter().map(|b| b.upper_bound).collect();
                        (hist.sum, counts, bounds)
                    }
                };

                let row = MetricRow {
                    timestamp: metric.timestamp.timestamp_nanos_opt().unwrap_or(0),
                    name: metric.name,
                    metric_type: metric.metric_type.to_string(),
                    value,
                    labels: metric.labels,
                    service,
                    bucket_counts,
                    bucket_bounds,
                    quantile_values: Vec::new(),
                    quantiles: Vec::new(),
                };

                inserter.write(&row).await?;
            }

            inserter.end().await?;
            Ok(())
        })
    }

    #[allow(clippy::too_many_lines)]
    fn query(&self, query: MetricQuery) -> Result<MetricQueryResult, MetricStoreError> {
        use std::fmt::Write as _;

        // Define row structure for deserialization
        #[derive(clickhouse::Row, serde::Deserialize)]
        struct MetricRow {
            timestamp: i64,
            name: String,
            metric_type: String,
            value: f64,
            labels: HashMap<String, String>,
            #[allow(dead_code)]
            service: String,
            bucket_counts: Vec<u64>,
            bucket_bounds: Vec<f64>,
        }

        // Build SQL query
        let mut sql = String::from("SELECT timestamp, name, metric_type, value, labels, service, bucket_counts, bucket_bounds FROM metrics WHERE 1=1");

        // Add name filter
        if let Some(ref name) = query.name {
            write!(&mut sql, " AND name = '{}'", name.replace('\'', "''")).unwrap();
        }

        // Add type filter
        if let Some(ref metric_type) = query.metric_type {
            write!(&mut sql, " AND metric_type = '{metric_type}'").unwrap();
        }

        // Add time range filters
        if let Some(start) = query.start_time {
            write!(
                &mut sql,
                " AND timestamp >= {}",
                start.timestamp_nanos_opt().unwrap_or(0)
            )
            .unwrap();
        }
        if let Some(end) = query.end_time {
            write!(
                &mut sql,
                " AND timestamp < {}",
                end.timestamp_nanos_opt().unwrap_or(0)
            )
            .unwrap();
        }

        // Add label filters
        for (key, value) in &query.labels {
            write!(
                &mut sql,
                " AND labels['{}'] = '{}'",
                key.replace('\'', "''"),
                value.replace('\'', "''")
            )
            .unwrap();
        }

        // Add ordering
        sql.push_str(" ORDER BY timestamp DESC");

        // Calculate total count query
        let count_sql = sql.replace(
            "SELECT timestamp, name, metric_type, value, labels, service, bucket_counts, bucket_bounds FROM metrics",
            "SELECT count() FROM metrics",
        );

        // Add limit and offset
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(1000);
        write!(&mut sql, " LIMIT {limit} OFFSET {offset}").unwrap();

        let client = Arc::clone(&self.client);
        let count_sql_clone = count_sql.clone();

        Self::block_on(async move {
            // Execute count query
            let total_count: u64 = client.query(&count_sql_clone).fetch_one::<u64>().await?;

            // Execute main query
            let rows: Vec<MetricRow> = client.query(&sql).fetch_all::<MetricRow>().await?;

            // Convert rows to Metric
            let metrics: Vec<Metric> = rows
                .into_iter()
                .map(|row| {
                    let timestamp = DateTime::from_timestamp_nanos(row.timestamp);
                    let metric_type = match row.metric_type.as_str() {
                        "counter" => MetricType::Counter,
                        "histogram" => MetricType::Histogram,
                        _ => MetricType::Gauge,
                    };

                    let value = if row.bucket_counts.is_empty() {
                        crate::models::metric::MetricValue::Simple(row.value)
                    } else {
                        // Reconstruct histogram
                        let buckets: Vec<crate::models::metric::HistogramBucket> = row
                            .bucket_bounds
                            .iter()
                            .zip(row.bucket_counts.iter())
                            .map(|(bound, count)| crate::models::metric::HistogramBucket {
                                upper_bound: *bound,
                                count: *count,
                            })
                            .collect();
                        let sum = row.value;
                        let count = row.bucket_counts.iter().sum();
                        crate::models::metric::MetricValue::Histogram(
                            crate::models::metric::HistogramData {
                                buckets,
                                sum,
                                count,
                            },
                        )
                    };

                    Metric {
                        name: row.name,
                        metric_type,
                        value,
                        timestamp,
                        labels: row.labels,
                        description: None,
                        unit: None,
                    }
                })
                .collect();

            Ok(MetricQueryResult {
                metrics,
                total_count: usize::try_from(total_count).unwrap_or(usize::MAX),
            })
        })
    }

    fn count(&self) -> Result<usize, MetricStoreError> {
        let client = Arc::clone(&self.client);
        let count: u64 = Self::block_on(async move {
            client
                .query("SELECT count() FROM metrics")
                .fetch_one::<u64>()
                .await
        })?;

        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    fn clear(&self) -> Result<(), MetricStoreError> {
        let client = Arc::clone(&self.client);
        Self::block_on(async move { client.query("TRUNCATE TABLE metrics").execute().await })
    }

    fn aggregate(
        &self,
        query: MetricQuery,
        function: AggregationFunction,
    ) -> Result<AggregationResult, MetricStoreError> {
        use std::fmt::Write as _;

        // Define row structure for deserialization
        #[derive(clickhouse::Row, serde::Deserialize)]
        struct AggRow {
            agg_value: f64,
            sample_count: u64,
        }

        // Build SQL query with aggregation
        let agg_func = match function {
            AggregationFunction::Sum => "sum(value)",
            AggregationFunction::Avg => "avg(value)",
            AggregationFunction::Min => "min(value)",
            AggregationFunction::Max => "max(value)",
            AggregationFunction::Count => "count()",
        };

        let mut sql = format!(
            "SELECT {agg_func} as agg_value, count() as sample_count FROM metrics WHERE 1=1"
        );

        // Add name filter
        if let Some(ref name) = query.name {
            write!(&mut sql, " AND name = '{}'", name.replace('\'', "''")).unwrap();
        }

        // Add type filter
        if let Some(ref metric_type) = query.metric_type {
            write!(&mut sql, " AND metric_type = '{metric_type}'").unwrap();
        }

        // Add time range filters
        if let Some(start) = query.start_time {
            write!(
                &mut sql,
                " AND timestamp >= {}",
                start.timestamp_nanos_opt().unwrap_or(0)
            )
            .unwrap();
        }
        if let Some(end) = query.end_time {
            write!(
                &mut sql,
                " AND timestamp < {}",
                end.timestamp_nanos_opt().unwrap_or(0)
            )
            .unwrap();
        }

        // Add label filters
        for (key, value) in &query.labels {
            write!(
                &mut sql,
                " AND labels['{}'] = '{}'",
                key.replace('\'', "''"),
                value.replace('\'', "''")
            )
            .unwrap();
        }

        let client = Arc::clone(&self.client);

        Self::block_on(async move {
            let row: AggRow = client.query(&sql).fetch_one::<AggRow>().await?;

            Ok(AggregationResult {
                value: row.agg_value,
                count: usize::try_from(row.sample_count).unwrap_or(usize::MAX),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metric(name: &str, value: f64) -> Metric {
        Metric::gauge(name, value)
    }

    #[test]
    fn test_new_store_is_empty() {
        let store = InMemoryMetricStore::new();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_insert_single_metric() {
        let store = InMemoryMetricStore::new();
        let metric = create_test_metric("cpu_usage", 75.5);

        store.insert(metric).unwrap();

        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_insert_batch() {
        let store = InMemoryMetricStore::new();
        let metrics = vec![
            create_test_metric("cpu_usage", 75.5),
            create_test_metric("memory_usage", 1024.0),
            create_test_metric("disk_usage", 50.0),
        ];

        store.insert_batch(metrics).unwrap();

        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn test_query_all_metrics() {
        let store = InMemoryMetricStore::new();
        store.insert(create_test_metric("metric1", 1.0)).unwrap();
        store.insert(create_test_metric("metric2", 2.0)).unwrap();

        let result = store.query(MetricQuery::new()).unwrap();

        assert_eq!(result.metrics.len(), 2);
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_query_by_name() {
        let store = InMemoryMetricStore::new();
        store.insert(create_test_metric("cpu_usage", 75.5)).unwrap();
        store
            .insert(create_test_metric("memory_usage", 1024.0))
            .unwrap();
        store.insert(create_test_metric("cpu_usage", 80.0)).unwrap();

        let result = store
            .query(MetricQuery::new().with_name("cpu_usage"))
            .unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result.metrics.iter().all(|m| m.name == "cpu_usage"));
    }

    #[test]
    fn test_query_by_type() {
        let store = InMemoryMetricStore::new();
        store.insert(Metric::counter("requests", 100.0)).unwrap();
        store.insert(Metric::gauge("temperature", 25.0)).unwrap();
        store.insert(Metric::counter("errors", 5.0)).unwrap();

        let result = store
            .query(MetricQuery::new().with_type(MetricType::Counter))
            .unwrap();

        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_query_by_labels() {
        let store = InMemoryMetricStore::new();
        store
            .insert(
                create_test_metric("cpu_usage", 75.5)
                    .with_label("host", "server1")
                    .with_label("env", "prod"),
            )
            .unwrap();
        store
            .insert(
                create_test_metric("cpu_usage", 80.0)
                    .with_label("host", "server2")
                    .with_label("env", "prod"),
            )
            .unwrap();
        store
            .insert(
                create_test_metric("cpu_usage", 50.0)
                    .with_label("host", "server1")
                    .with_label("env", "dev"),
            )
            .unwrap();

        let result = store
            .query(MetricQuery::new().with_label("env", "prod"))
            .unwrap();

        assert_eq!(result.total_count, 2);

        let result = store
            .query(
                MetricQuery::new()
                    .with_label("host", "server1")
                    .with_label("env", "prod"),
            )
            .unwrap();

        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn test_query_with_limit() {
        let store = InMemoryMetricStore::new();
        for i in 0..10 {
            store
                .insert(create_test_metric("metric", f64::from(i)))
                .unwrap();
        }

        let result = store.query(MetricQuery::new().with_limit(5)).unwrap();

        assert_eq!(result.metrics.len(), 5);
        assert_eq!(result.total_count, 10);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_aggregation_sum() {
        let store = InMemoryMetricStore::new();
        store.insert(create_test_metric("value", 10.0)).unwrap();
        store.insert(create_test_metric("value", 20.0)).unwrap();
        store.insert(create_test_metric("value", 30.0)).unwrap();

        let result = store
            .aggregate(MetricQuery::new(), AggregationFunction::Sum)
            .unwrap();

        assert_eq!(result.value, 60.0);
        assert_eq!(result.count, 3);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_aggregation_avg() {
        let store = InMemoryMetricStore::new();
        store.insert(create_test_metric("value", 10.0)).unwrap();
        store.insert(create_test_metric("value", 20.0)).unwrap();
        store.insert(create_test_metric("value", 30.0)).unwrap();

        let result = store
            .aggregate(MetricQuery::new(), AggregationFunction::Avg)
            .unwrap();

        assert_eq!(result.value, 20.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_aggregation_min_max() {
        let store = InMemoryMetricStore::new();
        store.insert(create_test_metric("value", 10.0)).unwrap();
        store.insert(create_test_metric("value", 50.0)).unwrap();
        store.insert(create_test_metric("value", 30.0)).unwrap();

        let min = store
            .aggregate(MetricQuery::new(), AggregationFunction::Min)
            .unwrap();
        assert_eq!(min.value, 10.0);

        let max = store
            .aggregate(MetricQuery::new(), AggregationFunction::Max)
            .unwrap();
        assert_eq!(max.value, 50.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_aggregation_with_filter() {
        let store = InMemoryMetricStore::new();
        store
            .insert(create_test_metric("cpu", 75.0).with_label("host", "server1"))
            .unwrap();
        store
            .insert(create_test_metric("cpu", 80.0).with_label("host", "server1"))
            .unwrap();
        store
            .insert(create_test_metric("cpu", 50.0).with_label("host", "server2"))
            .unwrap();

        let result = store
            .aggregate(
                MetricQuery::new().with_label("host", "server1"),
                AggregationFunction::Avg,
            )
            .unwrap();

        assert_eq!(result.value, 77.5);
        assert_eq!(result.count, 2);
    }

    #[test]
    fn test_clear_store() {
        let store = InMemoryMetricStore::new();
        store.insert(create_test_metric("metric1", 1.0)).unwrap();
        store.insert(create_test_metric("metric2", 2.0)).unwrap();

        assert_eq!(store.count().unwrap(), 2);

        store.clear().unwrap();

        assert_eq!(store.count().unwrap(), 0);
    }
}
