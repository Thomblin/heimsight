//! Log data model.
//!
//! Defines the core `LogEntry` structure for storing and transmitting log data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use validator::Validate;

/// Log severity level.
///
/// Follows standard syslog-style severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Detailed debug information.
    Trace,
    /// Debug information.
    Debug,
    /// Informational messages.
    Info,
    /// Warning conditions.
    Warn,
    /// Error conditions.
    Error,
    /// Critical/fatal conditions.
    Fatal,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "trace"),
            Self::Debug => write!(f, "debug"),
            Self::Info => write!(f, "info"),
            Self::Warn => write!(f, "warn"),
            Self::Error => write!(f, "error"),
            Self::Fatal => write!(f, "fatal"),
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// A log entry representing a single log event.
///
/// This is the core data structure for log ingestion and storage in Heimsight.
///
/// # Example
///
/// ```
/// use shared::models::{LogEntry, LogLevel};
/// use chrono::Utc;
/// use std::collections::HashMap;
///
/// let log = LogEntry {
///     timestamp: Utc::now(),
///     level: LogLevel::Info,
///     message: "User logged in".to_string(),
///     service: "auth-service".to_string(),
///     attributes: HashMap::from([
///         ("user_id".to_string(), serde_json::json!("12345")),
///     ]),
///     trace_id: None,
///     span_id: None,
/// };
///
/// assert!(log.validate_entry().is_ok());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LogEntry {
    /// Timestamp when the log event occurred.
    pub timestamp: DateTime<Utc>,

    /// Severity level of the log.
    #[serde(default)]
    pub level: LogLevel,

    /// The log message content.
    #[validate(length(min = 1, message = "Message cannot be empty"))]
    pub message: String,

    /// Name of the service that generated the log.
    #[validate(length(min = 1, message = "Service name cannot be empty"))]
    pub service: String,

    /// Additional key-value attributes.
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,

    /// Optional trace ID for distributed tracing correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Optional span ID for distributed tracing correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

/// Errors that can occur during log entry validation.
#[derive(Debug, Error)]
pub enum LogValidationError {
    /// The log message is empty.
    #[error("Log message cannot be empty")]
    EmptyMessage,

    /// The service name is empty.
    #[error("Service name cannot be empty")]
    EmptyService,

    /// Validation failed with details.
    #[error("Validation failed: {0}")]
    ValidationError(#[from] validator::ValidationErrors),
}

impl LogEntry {
    /// Creates a new log entry with the current timestamp.
    ///
    /// # Arguments
    ///
    /// * `level` - The severity level of the log
    /// * `message` - The log message content
    /// * `service` - The name of the service generating the log
    ///
    /// # Example
    ///
    /// ```
    /// use shared::models::{LogEntry, LogLevel};
    ///
    /// let log = LogEntry::new(LogLevel::Info, "Server started", "api-server");
    /// assert_eq!(log.level, LogLevel::Info);
    /// assert_eq!(log.message, "Server started");
    /// ```
    #[must_use]
    pub fn new(level: LogLevel, message: impl Into<String>, service: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message: message.into(),
            service: service.into(),
            attributes: HashMap::new(),
            trace_id: None,
            span_id: None,
        }
    }

    /// Adds an attribute to the log entry.
    ///
    /// # Arguments
    ///
    /// * `key` - The attribute key
    /// * `value` - The attribute value (must be serializable to JSON)
    ///
    /// # Example
    ///
    /// ```
    /// use shared::models::{LogEntry, LogLevel};
    ///
    /// let log = LogEntry::new(LogLevel::Info, "Request processed", "api")
    ///     .with_attribute("request_id", "abc-123")
    ///     .with_attribute("duration_ms", 150);
    ///
    /// assert!(log.attributes.contains_key("request_id"));
    /// ```
    #[must_use]
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.attributes.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    /// Sets the trace ID for distributed tracing correlation.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Sets the span ID for distributed tracing correlation.
    #[must_use]
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }

    /// Validates the log entry.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The message is empty
    /// - The service name is empty
    pub fn validate_entry(&self) -> Result<(), LogValidationError> {
        if self.message.is_empty() {
            return Err(LogValidationError::EmptyMessage);
        }
        if self.service.is_empty() {
            return Err(LogValidationError::EmptyService);
        }
        self.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_log_entry_new() {
        let log = LogEntry::new(LogLevel::Info, "Test message", "test-service");

        assert_eq!(log.level, LogLevel::Info);
        assert_eq!(log.message, "Test message");
        assert_eq!(log.service, "test-service");
        assert!(log.attributes.is_empty());
        assert!(log.trace_id.is_none());
        assert!(log.span_id.is_none());
    }

    #[test]
    fn test_log_entry_with_attributes() {
        let log = LogEntry::new(LogLevel::Debug, "Debug log", "service")
            .with_attribute("user_id", "123")
            .with_attribute("count", 42)
            .with_attribute("enabled", true);

        assert_eq!(log.attributes.len(), 3);
        assert_eq!(log.attributes.get("user_id"), Some(&json!("123")));
        assert_eq!(log.attributes.get("count"), Some(&json!(42)));
        assert_eq!(log.attributes.get("enabled"), Some(&json!(true)));
    }

    #[test]
    fn test_log_entry_with_trace_correlation() {
        let log = LogEntry::new(LogLevel::Info, "Traced log", "service")
            .with_trace_id("trace-abc-123")
            .with_span_id("span-xyz-789");

        assert_eq!(log.trace_id, Some("trace-abc-123".to_string()));
        assert_eq!(log.span_id, Some("span-xyz-789".to_string()));
    }

    #[test]
    fn test_log_entry_serialization() {
        let log = LogEntry::new(LogLevel::Error, "Something failed", "api")
            .with_attribute("error_code", "E001");

        let json = serde_json::to_string(&log).unwrap();

        assert!(json.contains("\"level\":\"error\""));
        assert!(json.contains("\"message\":\"Something failed\""));
        assert!(json.contains("\"service\":\"api\""));
        assert!(json.contains("\"error_code\":\"E001\""));
    }

    #[test]
    fn test_log_entry_deserialization() {
        let json = r#"{
            "timestamp": "2024-01-15T10:30:00Z",
            "level": "warn",
            "message": "High memory usage",
            "service": "monitor",
            "attributes": {"memory_pct": 85},
            "trace_id": "trace-123"
        }"#;

        let log: LogEntry = serde_json::from_str(json).unwrap();

        assert_eq!(log.level, LogLevel::Warn);
        assert_eq!(log.message, "High memory usage");
        assert_eq!(log.service, "monitor");
        assert_eq!(log.attributes.get("memory_pct"), Some(&json!(85)));
        assert_eq!(log.trace_id, Some("trace-123".to_string()));
        assert!(log.span_id.is_none());
    }

    #[test]
    fn test_log_entry_deserialization_defaults() {
        let json = r#"{
            "timestamp": "2024-01-15T10:30:00Z",
            "message": "Simple log",
            "service": "test"
        }"#;

        let log: LogEntry = serde_json::from_str(json).unwrap();

        assert_eq!(log.level, LogLevel::Info); // default
        assert!(log.attributes.is_empty()); // default
    }

    #[test]
    fn test_log_entry_validation_success() {
        let log = LogEntry::new(LogLevel::Info, "Valid message", "valid-service");
        assert!(log.validate_entry().is_ok());
    }

    #[test]
    fn test_log_entry_validation_empty_message() {
        let log = LogEntry::new(LogLevel::Info, "", "service");
        let result = log.validate_entry();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LogValidationError::EmptyMessage
        ));
    }

    #[test]
    fn test_log_entry_validation_empty_service() {
        let log = LogEntry::new(LogLevel::Info, "message", "");
        let result = log.validate_entry();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LogValidationError::EmptyService
        ));
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "trace");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Error.to_string(), "error");
        assert_eq!(LogLevel::Fatal.to_string(), "fatal");
    }

    #[test]
    fn test_log_level_serialization() {
        assert_eq!(
            serde_json::to_string(&LogLevel::Error).unwrap(),
            "\"error\""
        );
        assert_eq!(serde_json::to_string(&LogLevel::Warn).unwrap(), "\"warn\"");
    }

    #[test]
    fn test_log_level_deserialization() {
        let level: LogLevel = serde_json::from_str("\"debug\"").unwrap();
        assert_eq!(level, LogLevel::Debug);

        let level: LogLevel = serde_json::from_str("\"fatal\"").unwrap();
        assert_eq!(level, LogLevel::Fatal);
    }

    #[test]
    fn test_log_entry_roundtrip() {
        let original = LogEntry::new(LogLevel::Info, "Roundtrip test", "service")
            .with_attribute("key", "value")
            .with_trace_id("trace-id")
            .with_span_id("span-id");

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(original.level, deserialized.level);
        assert_eq!(original.message, deserialized.message);
        assert_eq!(original.service, deserialized.service);
        assert_eq!(original.attributes, deserialized.attributes);
        assert_eq!(original.trace_id, deserialized.trace_id);
        assert_eq!(original.span_id, deserialized.span_id);
    }
}
