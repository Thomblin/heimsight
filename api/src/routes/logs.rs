//! Log ingestion and query endpoints.
//!
//! Provides HTTP endpoints for ingesting and querying log data in Heimsight.

use crate::state::AppState;
use axum::{
    extract::{rejection::JsonRejection, Query, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::models::{LogEntry, LogLevel};
use shared::storage::LogQuery;
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

/// Query parameters for log retrieval.
#[derive(Debug, Deserialize)]
pub struct LogQueryParams {
    /// Filter logs starting from this time (inclusive).
    pub start_time: Option<DateTime<Utc>>,

    /// Filter logs up to this time (exclusive).
    pub end_time: Option<DateTime<Utc>>,

    /// Maximum number of logs to return (default: 100, max: 1000).
    pub limit: Option<usize>,

    /// Number of logs to skip (for pagination).
    pub offset: Option<usize>,
}

/// Response for log queries.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogQueryResponse {
    /// The logs matching the query.
    pub logs: Vec<LogEntry>,

    /// Total count of matching logs (before limit/offset applied).
    pub total_count: usize,

    /// Number of logs returned in this response.
    pub returned_count: usize,

    /// Limit used for this query.
    pub limit: usize,

    /// Offset used for this query.
    pub offset: usize,
}

/// Generic API error response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    /// Error type.
    pub error: String,
    /// Detailed error message.
    pub message: String,
}

/// Creates the log routes with application state.
pub fn logs_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/logs", post(ingest_logs).get(query_logs))
        .with_state(state)
}

/// Maximum limit for log queries.
const MAX_QUERY_LIMIT: usize = 1000;

/// Default limit for log queries.
const DEFAULT_QUERY_LIMIT: usize = 100;

/// Handler for log ingestion.
///
/// Accepts either a single log entry or a batch of log entries.
/// Returns 201 Created on success, 400 Bad Request on validation failure.
async fn ingest_logs(
    State(state): State<AppState>,
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

    // Store the logs
    let count = valid_entries.len();
    if let Err(e) = state.log_store().insert_batch(valid_entries) {
        tracing::error!(error = %e, "Failed to store logs");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LogIngestError {
                error: "storage_error".to_string(),
                message: "Failed to store logs".to_string(),
                details: None,
            }),
        ));
    }

    tracing::debug!(count = count, "Stored log entries");

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

/// Handler for log queries.
///
/// Returns logs matching the provided query parameters.
async fn query_logs(
    State(state): State<AppState>,
    Query(params): Query<LogQueryParams>,
) -> Result<Json<LogQueryResponse>, (StatusCode, Json<ApiError>)> {
    // Apply defaults and limits
    let limit = params
        .limit
        .unwrap_or(DEFAULT_QUERY_LIMIT)
        .min(MAX_QUERY_LIMIT);
    let offset = params.offset.unwrap_or(0);

    // Build the query
    let mut query = LogQuery::new().with_limit(limit).with_offset(offset);

    if let Some(start) = params.start_time {
        query = query.with_start_time(start);
    }
    if let Some(end) = params.end_time {
        query = query.with_end_time(end);
    }

    // Execute the query
    let result = state.log_store().query(query).map_err(|e| {
        tracing::error!(error = %e, "Failed to query logs");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: "storage_error".to_string(),
                message: "Failed to query logs".to_string(),
            }),
        )
    })?;

    Ok(Json(LogQueryResponse {
        returned_count: result.logs.len(),
        logs: result.logs,
        total_count: result.total_count,
        limit,
        offset,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        logs_routes(AppState::with_in_memory_store())
    }

    fn create_test_router_with_state() -> (Router, AppState) {
        let state = AppState::with_in_memory_store();
        let router = logs_routes(state.clone());
        (router, state)
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

    // ========== Query endpoint tests ==========

    #[tokio::test]
    async fn test_query_empty_store() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/logs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogQueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.logs.len(), 0);
        assert_eq!(result.total_count, 0);
        assert_eq!(result.returned_count, 0);
    }

    #[tokio::test]
    async fn test_query_returns_ingested_logs() {
        let (app, state) = create_test_router_with_state();

        // Insert some logs directly into the store
        let log = LogEntry::new(LogLevel::Info, "Test message", "test-service");
        state.log_store().insert(log).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/logs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogQueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.logs.len(), 1);
        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].message, "Test message");
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let (app, state) = create_test_router_with_state();

        // Insert multiple logs
        for i in 0..10 {
            let log = LogEntry::new(LogLevel::Info, format!("Log {}", i), "test-service");
            state.log_store().insert(log).unwrap();
        }

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/logs?limit=5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogQueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.logs.len(), 5);
        assert_eq!(result.returned_count, 5);
        assert_eq!(result.total_count, 10);
        assert_eq!(result.limit, 5);
    }

    #[tokio::test]
    async fn test_query_with_offset() {
        let (app, state) = create_test_router_with_state();

        // Insert multiple logs
        for i in 0..10 {
            let log = LogEntry::new(LogLevel::Info, format!("Log {}", i), "test-service");
            state.log_store().insert(log).unwrap();
        }

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/logs?offset=5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogQueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.logs.len(), 5);
        assert_eq!(result.offset, 5);
        assert_eq!(result.total_count, 10);
    }

    #[tokio::test]
    async fn test_query_limit_capped_at_max() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/logs?limit=5000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogQueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.limit, MAX_QUERY_LIMIT);
    }

    #[tokio::test]
    async fn test_query_with_time_range() {
        let (app, state) = create_test_router_with_state();

        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);

        // Insert a log with a specific timestamp
        let log = LogEntry {
            timestamp: one_hour_ago,
            level: LogLevel::Info,
            message: "Old log".to_string(),
            service: "test-service".to_string(),
            attributes: HashMap::new(),
            trace_id: None,
            span_id: None,
        };
        state.log_store().insert(log).unwrap();

        // Query with time range that includes the log
        // Use URL encoding for the timestamps
        let start_str = (now - chrono::Duration::hours(2)).to_rfc3339();
        let end_str = now.to_rfc3339();
        let start = urlencoding::encode(&start_str);
        let end = urlencoding::encode(&end_str);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!(
                        "/api/v1/logs?start_time={}&end_time={}",
                        start, end
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: LogQueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.total_count, 1);
    }

    #[tokio::test]
    async fn test_logs_persist_in_memory() {
        let (app, _state) = create_test_router_with_state();

        // First, ingest a log
        let ingest_body = r#"{
            "message": "Persisted log",
            "service": "test-service"
        }"#;

        let ingest_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(ingest_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(ingest_response.status(), StatusCode::CREATED);

        // Then query to verify it's stored
        let query_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/logs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(query_response.status(), StatusCode::OK);

        let body = query_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let result: LogQueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].message, "Persisted log");
    }
}
