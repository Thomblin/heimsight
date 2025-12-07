//! SQL-like query endpoint.
//!
//! Provides an endpoint for executing SQL-like queries against the log store.

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use shared::models::LogEntry;
use shared::query::{execute_query, parse_query, ExecutionError, ParseError, Query};

/// Request body for SQL-like queries.
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    /// The SQL-like query string.
    pub query: String,
}

/// Response for successful query execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    /// The logs matching the query.
    pub logs: Vec<LogEntry>,

    /// Total count of matching logs (before limit/offset applied).
    pub total_count: usize,

    /// Number of logs returned in this response.
    pub returned_count: usize,

    /// The parsed query (for debugging/transparency).
    pub parsed_query: Query,
}

/// Error response for query operations.
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryError {
    /// Error type.
    pub error: String,
    /// Detailed error message.
    pub message: String,
}

impl From<ParseError> for QueryError {
    fn from(e: ParseError) -> Self {
        Self {
            error: "parse_error".to_string(),
            message: e.to_string(),
        }
    }
}

impl From<ExecutionError> for QueryError {
    fn from(e: ExecutionError) -> Self {
        Self {
            error: "execution_error".to_string(),
            message: e.to_string(),
        }
    }
}

/// Creates the query routes with application state.
pub fn query_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/query", post(execute_sql_query))
        .with_state(state)
}

/// Handler for SQL-like query execution.
///
/// Parses and executes a SQL-like query against the log store.
async fn execute_sql_query(
    State(state): State<AppState>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, (StatusCode, Json<QueryError>)> {
    // Parse the query
    let query = parse_query(&request.query).map_err(|e| {
        tracing::debug!(query = %request.query, error = %e, "Failed to parse query");
        (StatusCode::BAD_REQUEST, Json(QueryError::from(e)))
    })?;

    // Execute the query
    let result = execute_query(&query, state.log_store()).map_err(|e| {
        tracing::error!(error = %e, "Failed to execute query");
        let status = match &e {
            ExecutionError::StorageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ExecutionError::UnsupportedSource(_)
            | ExecutionError::UnknownField(_)
            | ExecutionError::TypeMismatch { .. } => StatusCode::BAD_REQUEST,
        };
        (status, Json(QueryError::from(e)))
    })?;

    tracing::debug!(
        total = result.total_count,
        returned = result.logs.len(),
        "Query executed successfully"
    );

    Ok(Json(QueryResponse {
        returned_count: result.logs.len(),
        logs: result.logs,
        total_count: result.total_count,
        parsed_query: query,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use http_body_util::BodyExt;
    use shared::models::LogLevel;
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        query_routes(AppState::with_in_memory_store())
    }

    fn create_test_router_with_state() -> (Router, AppState) {
        let state = AppState::with_in_memory_store();
        let router = query_routes(state.clone());
        (router, state)
    }

    #[tokio::test]
    async fn test_query_simple_select() {
        let (app, state) = create_test_router_with_state();

        // Insert some logs
        state
            .log_store()
            .insert(LogEntry::new(LogLevel::Info, "Test message", "api"))
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"query": "SELECT * FROM logs"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: QueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs.len(), 1);
    }

    #[tokio::test]
    async fn test_query_with_where() {
        let (app, state) = create_test_router_with_state();

        state
            .log_store()
            .insert(LogEntry::new(LogLevel::Info, "Info message", "api"))
            .unwrap();
        state
            .log_store()
            .insert(LogEntry::new(LogLevel::Error, "Error message", "api"))
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"query": "SELECT * FROM logs WHERE level = 'error'"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: QueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.total_count, 1);
        assert!(result.logs.iter().all(|l| l.level == LogLevel::Error));
    }

    #[tokio::test]
    async fn test_query_with_complex_where() {
        let (app, state) = create_test_router_with_state();

        state
            .log_store()
            .insert(LogEntry::new(LogLevel::Error, "API error", "api"))
            .unwrap();
        state
            .log_store()
            .insert(LogEntry::new(LogLevel::Error, "DB error", "db"))
            .unwrap();
        state
            .log_store()
            .insert(LogEntry::new(LogLevel::Info, "Info", "api"))
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"query": "SELECT * FROM logs WHERE level = 'error' AND service = 'api'"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: QueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].message, "API error");
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let (app, state) = create_test_router_with_state();

        for i in 0..10 {
            state
                .log_store()
                .insert(LogEntry::new(LogLevel::Info, format!("Log {i}"), "api"))
                .unwrap();
        }

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"query": "SELECT * FROM logs LIMIT 5"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: QueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.logs.len(), 5);
        assert_eq!(result.total_count, 10);
    }

    #[tokio::test]
    async fn test_query_invalid_syntax() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"query": "SELECT FROM logs"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: QueryError = serde_json::from_slice(&body).unwrap();

        assert_eq!(error.error, "parse_error");
    }

    #[tokio::test]
    async fn test_query_empty_query() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"query": ""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_query_unsupported_source() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"query": "SELECT * FROM metrics"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: QueryError = serde_json::from_slice(&body).unwrap();

        assert_eq!(error.error, "execution_error");
    }

    #[tokio::test]
    async fn test_query_returns_parsed_query() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"query": "SELECT * FROM logs WHERE level = 'error' LIMIT 10"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: QueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.parsed_query.limit, Some(10));
    }

    #[tokio::test]
    async fn test_query_message_contains() {
        let (app, state) = create_test_router_with_state();

        state
            .log_store()
            .insert(LogEntry::new(
                LogLevel::Error,
                "Connection failed to database",
                "db",
            ))
            .unwrap();
        state
            .log_store()
            .insert(LogEntry::new(LogLevel::Info, "Everything is fine", "api"))
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"query": "SELECT * FROM logs WHERE message CONTAINS 'failed'"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: QueryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.total_count, 1);
    }
}
