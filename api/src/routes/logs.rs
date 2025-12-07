//! Log ingestion endpoints.
//!
//! Provides HTTP endpoints for ingesting log data into Heimsight.

use axum::{extract::rejection::JsonRejection, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use shared::models::{LogEntry, LogLevel};
use std::collections::HashMap;

/// Request body for log ingestion - can be a single log or a batch.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LogIngestRequest {
    /// A single log entry.
    Single(LogEntryRequest),
    /// A batch of log entries.
    Batch(Vec<LogEntryRequest>),
}

/// A log entry as received from the API.
///
/// This is similar to `LogEntry` but with optional timestamp (defaults to now).
#[derive(Debug, Deserialize)]
pub struct LogEntryRequest {
    /// Timestamp (optional, defaults to current time).
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Log level (optional, defaults to info).
    #[serde(default)]
    pub level: LogLevel,

    /// Log message (required).
    pub message: String,

    /// Service name (required).
    pub service: String,

    /// Additional attributes (optional).
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,

    /// Trace ID for correlation (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Span ID for correlation (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

impl From<LogEntryRequest> for LogEntry {
    fn from(req: LogEntryRequest) -> Self {
        Self {
            timestamp: req.timestamp,
            level: req.level,
            message: req.message,
            service: req.service,
            attributes: req.attributes,
            trace_id: req.trace_id,
            span_id: req.span_id,
        }
    }
}

/// Response for successful log ingestion.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogIngestResponse {
    /// Number of logs accepted.
    pub accepted: usize,
    /// Message describing the result.
    pub message: String,
}

/// Error response for failed log ingestion.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogIngestError {
    /// Error type.
    pub error: String,
    /// Detailed error message.
    pub message: String,
    /// Validation errors by index (for batch requests).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<ValidationErrorDetail>>,
}

/// Validation error detail for a specific log entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationErrorDetail {
    /// Index in the batch (0 for single requests).
    pub index: usize,
    /// Field that failed validation.
    pub field: String,
    /// Error message.
    pub message: String,
}

/// Creates the log ingestion routes.
pub fn logs_routes() -> Router {
    Router::new().route("/api/v1/logs", post(ingest_logs))
}

/// Handler for log ingestion.
///
/// Accepts either a single log entry or a batch of log entries.
/// Returns 201 Created on success, 400 Bad Request on validation failure.
async fn ingest_logs(
    payload: Result<Json<LogIngestRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<LogIngestResponse>), (StatusCode, Json<LogIngestError>)> {
    // Handle JSON parsing errors
    let Json(request) = payload.map_err(|rejection| {
        (
            StatusCode::BAD_REQUEST,
            Json(LogIngestError {
                error: "invalid_json".to_string(),
                message: rejection.body_text(),
                details: None,
            }),
        )
    })?;

    // Convert to entries and validate
    let entries: Vec<LogEntryRequest> = match request {
        LogIngestRequest::Single(entry) => vec![entry],
        LogIngestRequest::Batch(entries) => entries,
    };

    // Check for empty batch
    if entries.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(LogIngestError {
                error: "empty_batch".to_string(),
                message: "At least one log entry is required".to_string(),
                details: None,
            }),
        ));
    }

    // Validate all entries
    let mut validation_errors = Vec::new();
    let mut valid_entries = Vec::new();

    for (index, entry) in entries.into_iter().enumerate() {
        let log_entry: LogEntry = entry.into();

        if let Err(e) = log_entry.validate_entry() {
            validation_errors.push(ValidationErrorDetail {
                index,
                field: match e {
                    shared::models::log::LogValidationError::EmptyMessage => "message".to_string(),
                    shared::models::log::LogValidationError::EmptyService => "service".to_string(),
                    shared::models::log::LogValidationError::ValidationError(_) => {
                        "unknown".to_string()
                    }
                },
                message: e.to_string(),
            });
        } else {
            valid_entries.push(log_entry);
        }
    }

    // If there are validation errors, return 400
    if !validation_errors.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(LogIngestError {
                error: "validation_failed".to_string(),
                message: format!(
                    "{} log entry/entries failed validation",
                    validation_errors.len()
                ),
                details: Some(validation_errors),
            }),
        ));
    }

    // TODO: Actually store the logs (Step 1.5)
    // For now, we just accept them and log
    let count = valid_entries.len();
    tracing::debug!(count = count, "Accepted log entries");

    Ok((
        StatusCode::CREATED,
        Json(LogIngestResponse {
            accepted: count,
            message: format!(
                "Successfully ingested {} log {}",
                count,
                if count == 1 { "entry" } else { "entries" }
            ),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        logs_routes()
    }

    #[tokio::test]
    async fn test_ingest_single_log() {
        let app = create_test_router();

        let body = r#"{
            "message": "Test log message",
            "service": "test-service",
            "level": "info"
        }"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogIngestResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.accepted, 1);
    }

    #[tokio::test]
    async fn test_ingest_batch_logs() {
        let app = create_test_router();

        let body = r#"[
            {"message": "Log 1", "service": "svc1", "level": "info"},
            {"message": "Log 2", "service": "svc2", "level": "warn"},
            {"message": "Log 3", "service": "svc3", "level": "error"}
        ]"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogIngestResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.accepted, 3);
    }

    #[tokio::test]
    async fn test_ingest_log_with_attributes() {
        let app = create_test_router();

        let body = r#"{
            "message": "Request processed",
            "service": "api",
            "level": "info",
            "attributes": {
                "request_id": "abc-123",
                "user_id": 42,
                "success": true
            },
            "trace_id": "trace-xyz",
            "span_id": "span-123"
        }"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_ingest_log_defaults() {
        let app = create_test_router();

        // Minimal log - only required fields
        let body = r#"{
            "message": "Minimal log",
            "service": "test"
        }"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_ingest_invalid_empty_message() {
        let app = create_test_router();

        let body = r#"{
            "message": "",
            "service": "test-service"
        }"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: LogIngestError = serde_json::from_slice(&body).unwrap();

        assert_eq!(error.error, "validation_failed");
        assert!(error.details.is_some());
    }

    #[tokio::test]
    async fn test_ingest_invalid_empty_service() {
        let app = create_test_router();

        let body = r#"{
            "message": "Test message",
            "service": ""
        }"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_ingest_invalid_json() {
        let app = create_test_router();

        let body = r#"{ invalid json }"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: LogIngestError = serde_json::from_slice(&body).unwrap();

        assert_eq!(error.error, "invalid_json");
    }

    #[tokio::test]
    async fn test_ingest_empty_batch() {
        let app = create_test_router();

        let body = r#"[]"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: LogIngestError = serde_json::from_slice(&body).unwrap();

        assert_eq!(error.error, "empty_batch");
    }

    #[tokio::test]
    async fn test_ingest_batch_partial_validation_failure() {
        let app = create_test_router();

        // Mix of valid and invalid entries
        let body = r#"[
            {"message": "Valid log", "service": "svc1"},
            {"message": "", "service": "svc2"},
            {"message": "Another valid", "service": ""}
        ]"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: LogIngestError = serde_json::from_slice(&body).unwrap();

        assert_eq!(error.error, "validation_failed");
        let details = error.details.unwrap();
        assert_eq!(details.len(), 2); // Two entries failed validation
        assert_eq!(details[0].index, 1);
        assert_eq!(details[1].index, 2);
    }
}
