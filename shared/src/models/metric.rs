//! Metric data model.
//!
//! Defines the core `Metric` structure for storing and transmitting metric data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use validator::Validate;

/// Type of metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    /// A counter that only increases (e.g., request count).
    Counter,
    /// A gauge that can go up or down (e.g., temperature, memory usage).
    Gauge,
    /// A histogram for measuring distributions (e.g., request latency).
    Histogram,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Counter => write!(f, "counter"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
        }
    }
}

impl Default for MetricType {
    fn default() -> Self {
        Self::Gauge
    }
}

/// A histogram bucket for distribution metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// The upper bound of this bucket (exclusive).
    pub upper_bound: f64,
    /// The cumulative count of observations in this bucket.
    pub count: u64,
}

/// Histogram data for distribution metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistogramData {
    /// The histogram buckets.
    pub buckets: Vec<HistogramBucket>,
    /// The sum of all observed values.
    pub sum: f64,
    /// The total count of observations.
    pub count: u64,
}

/// The value of a metric, which varies by metric type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    /// A simple numeric value (for counters and gauges).
    Simple(f64),
    /// Histogram data for distribution metrics.
    Histogram(HistogramData),
}

impl MetricValue {
    /// Returns the simple value if this is a simple metric.
    #[must_use]
    pub fn as_simple(&self) -> Option<f64> {
        match self {
            Self::Simple(v) => Some(*v),
            Self::Histogram(_) => None,
        }
    }

    /// Returns the histogram data if this is a histogram metric.
    #[must_use]
    pub fn as_histogram(&self) -> Option<&HistogramData> {
        match self {
            Self::Simple(_) => None,
            Self::Histogram(h) => Some(h),
        }
    }
}

/// A metric data point representing a single measurement.
///
/// # Example
///
/// ```
/// use shared::models::{Metric, MetricType, MetricValue};
/// use std::collections::HashMap;
///
/// let metric = Metric::new(
///     "http_requests_total",
///     MetricType::Counter,
///     MetricValue::Simple(1234.0),
/// )
/// .with_label("method", "GET")
/// .with_label("status", "200");
///
/// assert!(metric.validate_metric().is_ok());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Metric {
    /// The name of the metric (e.g., "`http_requests_total`").
    #[validate(length(min = 1, message = "Metric name cannot be empty"))]
    pub name: String,

    /// The type of metric.
    pub metric_type: MetricType,

    /// The metric value.
    pub value: MetricValue,

    /// Timestamp when the metric was recorded.
    pub timestamp: DateTime<Utc>,

    /// Labels (dimensions) for the metric.
    #[serde(default)]
    pub labels: HashMap<String, String>,

    /// Optional description of the metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional unit of the metric (e.g., "bytes", "seconds").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// Errors that can occur during metric validation.
#[derive(Debug, Error)]
pub enum MetricValidationError {
    /// The metric name is empty.
    #[error("Metric name cannot be empty")]
    EmptyName,

    /// Invalid label name.
    #[error("Invalid label name: '{0}'")]
    InvalidLabelName(String),

    /// Histogram has invalid buckets.
    #[error("Histogram buckets must be sorted in ascending order")]
    InvalidHistogramBuckets,

    /// Validation failed with details.
    #[error("Validation failed: {0}")]
    ValidationError(#[from] validator::ValidationErrors),
}

impl Metric {
    /// Creates a new metric with the current timestamp.
    ///
    /// # Arguments
    ///
    /// * `name` - The metric name
    /// * `metric_type` - The type of metric
    /// * `value` - The metric value
    #[must_use]
    pub fn new(name: impl Into<String>, metric_type: MetricType, value: MetricValue) -> Self {
        Self {
            name: name.into(),
            metric_type,
            value,
            timestamp: Utc::now(),
            labels: HashMap::new(),
            description: None,
            unit: None,
        }
    }

    /// Creates a new counter metric.
    #[must_use]
    pub fn counter(name: impl Into<String>, value: f64) -> Self {
        Self::new(name, MetricType::Counter, MetricValue::Simple(value))
    }

    /// Creates a new gauge metric.
    #[must_use]
    pub fn gauge(name: impl Into<String>, value: f64) -> Self {
        Self::new(name, MetricType::Gauge, MetricValue::Simple(value))
    }

    /// Creates a new histogram metric.
    #[must_use]
    pub fn histogram(name: impl Into<String>, data: HistogramData) -> Self {
        Self::new(name, MetricType::Histogram, MetricValue::Histogram(data))
    }

    /// Adds a label to the metric.
    #[must_use]
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Sets the description of the metric.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the unit of the metric.
    #[must_use]
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Sets the timestamp of the metric.
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Validates the metric.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The name is empty
    /// - Histogram buckets are not sorted
    pub fn validate_metric(&self) -> Result<(), MetricValidationError> {
        if self.name.is_empty() {
            return Err(MetricValidationError::EmptyName);
        }

        // Validate histogram buckets are sorted
        if let MetricValue::Histogram(ref hist) = self.value {
            for i in 1..hist.buckets.len() {
                if hist.buckets[i].upper_bound <= hist.buckets[i - 1].upper_bound {
                    return Err(MetricValidationError::InvalidHistogramBuckets);
                }
            }
        }

        self.validate()?;
        Ok(())
    }

    /// Returns the simple value if this metric has one.
    #[must_use]
    pub fn simple_value(&self) -> Option<f64> {
        self.value.as_simple()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_counter() {
        let metric = Metric::counter("requests_total", 100.0)
            .with_label("method", "GET")
            .with_label("path", "/api/users");

        assert_eq!(metric.name, "requests_total");
        assert_eq!(metric.metric_type, MetricType::Counter);
        assert_eq!(metric.simple_value(), Some(100.0));
        assert_eq!(metric.labels.get("method"), Some(&"GET".to_string()));
    }

    #[test]
    fn test_metric_gauge() {
        let metric = Metric::gauge("memory_usage_bytes", 1024.0 * 1024.0 * 512.0)
            .with_unit("bytes")
            .with_description("Memory usage in bytes");

        assert_eq!(metric.metric_type, MetricType::Gauge);
        assert_eq!(metric.unit, Some("bytes".to_string()));
        assert!(metric.description.is_some());
    }

    #[test]
    fn test_metric_histogram() {
        let histogram = HistogramData {
            buckets: vec![
                HistogramBucket {
                    upper_bound: 0.1,
                    count: 10,
                },
                HistogramBucket {
                    upper_bound: 0.5,
                    count: 25,
                },
                HistogramBucket {
                    upper_bound: 1.0,
                    count: 30,
                },
            ],
            sum: 15.5,
            count: 30,
        };

        let metric = Metric::histogram("request_duration_seconds", histogram);

        assert_eq!(metric.metric_type, MetricType::Histogram);
        assert!(metric.value.as_histogram().is_some());
    }

    #[test]
    fn test_metric_validation_success() {
        let metric = Metric::counter("valid_metric", 1.0);
        assert!(metric.validate_metric().is_ok());
    }

    #[test]
    fn test_metric_validation_empty_name() {
        let metric = Metric::counter("", 1.0);
        let result = metric.validate_metric();
        assert!(matches!(result, Err(MetricValidationError::EmptyName)));
    }

    #[test]
    fn test_metric_validation_invalid_histogram() {
        let histogram = HistogramData {
            buckets: vec![
                HistogramBucket {
                    upper_bound: 1.0,
                    count: 10,
                },
                HistogramBucket {
                    upper_bound: 0.5, // Wrong order!
                    count: 5,
                },
            ],
            sum: 5.0,
            count: 15,
        };

        let metric = Metric::histogram("bad_histogram", histogram);
        let result = metric.validate_metric();
        assert!(matches!(
            result,
            Err(MetricValidationError::InvalidHistogramBuckets)
        ));
    }

    #[test]
    fn test_metric_serialization() {
        let metric = Metric::counter("test_counter", 42.0).with_label("env", "production");

        let json = serde_json::to_string(&metric).unwrap();

        assert!(json.contains("\"name\":\"test_counter\""));
        assert!(json.contains("\"metric_type\":\"counter\""));
        assert!(json.contains("\"value\":42.0"));
    }

    #[test]
    fn test_metric_deserialization() {
        let json = r#"{
            "name": "cpu_usage",
            "metric_type": "gauge",
            "value": 75.5,
            "timestamp": "2024-01-15T10:30:00Z",
            "labels": {"host": "server1"}
        }"#;

        let metric: Metric = serde_json::from_str(json).unwrap();

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.metric_type, MetricType::Gauge);
        assert_eq!(metric.simple_value(), Some(75.5));
        assert_eq!(metric.labels.get("host"), Some(&"server1".to_string()));
    }

    #[test]
    fn test_metric_type_display() {
        assert_eq!(MetricType::Counter.to_string(), "counter");
        assert_eq!(MetricType::Gauge.to_string(), "gauge");
        assert_eq!(MetricType::Histogram.to_string(), "histogram");
    }
}
