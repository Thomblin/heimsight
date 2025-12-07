//! Trace and span data models.
//!
//! Defines the core structures for distributed tracing in Heimsight.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use validator::Validate;

/// Status code for a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SpanStatus {
    /// The span completed without error.
    #[default]
    Ok,
    /// The span encountered an error.
    Error,
    /// The span was cancelled.
    Cancelled,
}

impl std::fmt::Display for SpanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::Error => write!(f, "error"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Kind of span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SpanKind {
    /// Default span kind (internal operation).
    #[default]
    Internal,
    /// The span represents a server handling a request.
    Server,
    /// The span represents a client making a request.
    Client,
    /// The span represents a producer sending a message.
    Producer,
    /// The span represents a consumer receiving a message.
    Consumer,
}

impl std::fmt::Display for SpanKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal => write!(f, "internal"),
            Self::Server => write!(f, "server"),
            Self::Client => write!(f, "client"),
            Self::Producer => write!(f, "producer"),
            Self::Consumer => write!(f, "consumer"),
        }
    }
}

/// An event within a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// The name of the event.
    pub name: String,
    /// Timestamp when the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Additional attributes for the event.
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
}

/// A span representing a unit of work in a distributed trace.
///
/// # Example
///
/// ```
/// use shared::models::{Span, SpanKind, SpanStatus};
///
/// let span = Span::new("trace-123", "span-456", "HTTP GET /api/users", "api-service")
///     .with_kind(SpanKind::Server)
///     .with_attribute("http.method", "GET")
///     .with_attribute("http.status_code", 200);
///
/// assert!(span.validate_span().is_ok());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Span {
    /// Unique identifier for the trace this span belongs to.
    #[validate(length(min = 1, message = "Trace ID cannot be empty"))]
    pub trace_id: String,

    /// Unique identifier for this span.
    #[validate(length(min = 1, message = "Span ID cannot be empty"))]
    pub span_id: String,

    /// The parent span ID (None for root spans).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,

    /// The name/operation of this span.
    #[validate(length(min = 1, message = "Span name cannot be empty"))]
    pub name: String,

    /// The service that generated this span.
    #[validate(length(min = 1, message = "Service name cannot be empty"))]
    pub service: String,

    /// The kind of span.
    #[serde(default)]
    pub kind: SpanKind,

    /// The status of the span.
    #[serde(default)]
    pub status: SpanStatus,

    /// Timestamp when the span started.
    pub start_time: DateTime<Utc>,

    /// Timestamp when the span ended.
    pub end_time: DateTime<Utc>,

    /// Additional attributes for the span.
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,

    /// Events that occurred during the span.
    #[serde(default)]
    pub events: Vec<SpanEvent>,
}

/// Errors that can occur during span validation.
#[derive(Debug, Error)]
pub enum SpanValidationError {
    /// The trace ID is empty.
    #[error("Trace ID cannot be empty")]
    EmptyTraceId,

    /// The span ID is empty.
    #[error("Span ID cannot be empty")]
    EmptySpanId,

    /// The span name is empty.
    #[error("Span name cannot be empty")]
    EmptyName,

    /// The service name is empty.
    #[error("Service name cannot be empty")]
    EmptyService,

    /// The end time is before the start time.
    #[error("End time cannot be before start time")]
    InvalidTimeRange,

    /// Validation failed with details.
    #[error("Validation failed: {0}")]
    ValidationError(#[from] validator::ValidationErrors),
}

impl Span {
    /// Creates a new span with the current time as both start and end.
    ///
    /// Call `finish()` to set the actual end time.
    #[must_use]
    pub fn new(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        name: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
            name: name.into(),
            service: service.into(),
            kind: SpanKind::default(),
            status: SpanStatus::default(),
            start_time: now,
            end_time: now,
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Sets the parent span ID.
    #[must_use]
    pub fn with_parent(mut self, parent_span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    /// Sets the span kind.
    #[must_use]
    pub fn with_kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets the span status.
    #[must_use]
    pub fn with_status(mut self, status: SpanStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets the start time.
    #[must_use]
    pub fn with_start_time(mut self, start_time: DateTime<Utc>) -> Self {
        self.start_time = start_time;
        self
    }

    /// Sets the end time.
    #[must_use]
    pub fn with_end_time(mut self, end_time: DateTime<Utc>) -> Self {
        self.end_time = end_time;
        self
    }

    /// Adds an attribute to the span.
    #[must_use]
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.attributes.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    /// Adds an event to the span.
    #[must_use]
    pub fn with_event(mut self, name: impl Into<String>) -> Self {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        });
        self
    }

    /// Returns the duration of the span.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.end_time - self.start_time
    }

    /// Returns the duration in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> i64 {
        self.duration().num_milliseconds()
    }

    /// Returns true if this is a root span (no parent).
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent_span_id.is_none()
    }

    /// Validates the span.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The trace ID is empty
    /// - The span ID is empty
    /// - The name is empty
    /// - The service is empty
    /// - The end time is before the start time
    pub fn validate_span(&self) -> Result<(), SpanValidationError> {
        if self.trace_id.is_empty() {
            return Err(SpanValidationError::EmptyTraceId);
        }
        if self.span_id.is_empty() {
            return Err(SpanValidationError::EmptySpanId);
        }
        if self.name.is_empty() {
            return Err(SpanValidationError::EmptyName);
        }
        if self.service.is_empty() {
            return Err(SpanValidationError::EmptyService);
        }
        if self.end_time < self.start_time {
            return Err(SpanValidationError::InvalidTimeRange);
        }
        self.validate()?;
        Ok(())
    }
}

/// A trace consisting of multiple spans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    /// The trace ID.
    pub trace_id: String,

    /// All spans in this trace.
    pub spans: Vec<Span>,

    /// The root span (if any).
    #[serde(skip)]
    root_span_index: Option<usize>,
}

impl Trace {
    /// Creates a new trace from a collection of spans.
    #[must_use]
    pub fn from_spans(spans: Vec<Span>) -> Option<Self> {
        if spans.is_empty() {
            return None;
        }

        let trace_id = spans[0].trace_id.clone();

        // Find root span
        let root_span_index = spans.iter().position(Span::is_root);

        Some(Self {
            trace_id,
            spans,
            root_span_index,
        })
    }

    /// Returns the root span if it exists.
    #[must_use]
    pub fn root_span(&self) -> Option<&Span> {
        self.root_span_index.map(|i| &self.spans[i])
    }

    /// Returns the total duration of the trace (from earliest start to latest end).
    #[must_use]
    pub fn duration(&self) -> Option<Duration> {
        if self.spans.is_empty() {
            return None;
        }

        let start = self.spans.iter().map(|s| s.start_time).min()?;
        let end = self.spans.iter().map(|s| s.end_time).max()?;

        Some(end - start)
    }

    /// Returns the number of spans in this trace.
    #[must_use]
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Returns all services involved in this trace.
    #[must_use]
    pub fn services(&self) -> Vec<&str> {
        let mut services: Vec<&str> = self.spans.iter().map(|s| s.service.as_str()).collect();
        services.sort_unstable();
        services.dedup();
        services
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_new() {
        let span = Span::new("trace-123", "span-456", "GET /api", "api-service");

        assert_eq!(span.trace_id, "trace-123");
        assert_eq!(span.span_id, "span-456");
        assert_eq!(span.name, "GET /api");
        assert_eq!(span.service, "api-service");
        assert!(span.is_root());
        assert_eq!(span.status, SpanStatus::Ok);
    }

    #[test]
    fn test_span_with_parent() {
        let span =
            Span::new("trace-123", "span-456", "DB query", "db-service").with_parent("span-123");

        assert!(!span.is_root());
        assert_eq!(span.parent_span_id, Some("span-123".to_string()));
    }

    #[test]
    fn test_span_with_attributes() {
        let span = Span::new("trace-123", "span-456", "HTTP request", "api")
            .with_attribute("http.method", "GET")
            .with_attribute("http.status_code", 200)
            .with_attribute("success", true);

        assert_eq!(span.attributes.len(), 3);
        assert_eq!(
            span.attributes.get("http.method"),
            Some(&serde_json::json!("GET"))
        );
    }

    #[test]
    fn test_span_duration() {
        let start = Utc::now();
        let end = start + Duration::milliseconds(100);

        let span = Span::new("trace-123", "span-456", "operation", "service")
            .with_start_time(start)
            .with_end_time(end);

        assert_eq!(span.duration_ms(), 100);
    }

    #[test]
    fn test_span_validation_success() {
        let span = Span::new("trace-123", "span-456", "operation", "service");
        assert!(span.validate_span().is_ok());
    }

    #[test]
    fn test_span_validation_empty_trace_id() {
        let span = Span::new("", "span-456", "operation", "service");
        assert!(matches!(
            span.validate_span(),
            Err(SpanValidationError::EmptyTraceId)
        ));
    }

    #[test]
    fn test_span_validation_empty_span_id() {
        let span = Span::new("trace-123", "", "operation", "service");
        assert!(matches!(
            span.validate_span(),
            Err(SpanValidationError::EmptySpanId)
        ));
    }

    #[test]
    fn test_span_validation_invalid_time_range() {
        let start = Utc::now();
        let end = start - Duration::seconds(1);

        let span = Span::new("trace-123", "span-456", "operation", "service")
            .with_start_time(start)
            .with_end_time(end);

        assert!(matches!(
            span.validate_span(),
            Err(SpanValidationError::InvalidTimeRange)
        ));
    }

    #[test]
    fn test_span_serialization() {
        let span = Span::new("trace-123", "span-456", "GET /api", "api")
            .with_kind(SpanKind::Server)
            .with_attribute("user_id", "12345");

        let json = serde_json::to_string(&span).unwrap();

        assert!(json.contains("\"trace_id\":\"trace-123\""));
        assert!(json.contains("\"kind\":\"server\""));
    }

    #[test]
    fn test_trace_from_spans() {
        let root = Span::new("trace-123", "span-1", "root", "api");
        let child = Span::new("trace-123", "span-2", "child", "db").with_parent("span-1");

        let trace = Trace::from_spans(vec![root, child]).unwrap();

        assert_eq!(trace.trace_id, "trace-123");
        assert_eq!(trace.span_count(), 2);
        assert!(trace.root_span().is_some());
    }

    #[test]
    fn test_trace_services() {
        let spans = vec![
            Span::new("trace-123", "span-1", "op1", "api"),
            Span::new("trace-123", "span-2", "op2", "db"),
            Span::new("trace-123", "span-3", "op3", "api"),
            Span::new("trace-123", "span-4", "op4", "cache"),
        ];

        let trace = Trace::from_spans(spans).unwrap();
        let services = trace.services();

        assert_eq!(services.len(), 3);
        assert!(services.contains(&"api"));
        assert!(services.contains(&"db"));
        assert!(services.contains(&"cache"));
    }

    #[test]
    fn test_span_status_display() {
        assert_eq!(SpanStatus::Ok.to_string(), "ok");
        assert_eq!(SpanStatus::Error.to_string(), "error");
        assert_eq!(SpanStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_span_kind_display() {
        assert_eq!(SpanKind::Server.to_string(), "server");
        assert_eq!(SpanKind::Client.to_string(), "client");
        assert_eq!(SpanKind::Internal.to_string(), "internal");
    }
}
