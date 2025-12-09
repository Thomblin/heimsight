//! Retention configuration API routes.
//!
//! Provides endpoints for managing data retention (TTL) policies.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use shared::config::{DataType, RetentionConfig, RetentionPolicy};

use crate::state::AppState;

/// Request body for updating a single retention policy.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRetentionPolicyRequest {
    /// The data type to update.
    pub data_type: DataType,
    /// New TTL in days.
    pub ttl_days: u32,
}

/// Response body for retention operations.
#[derive(Debug, Serialize, Deserialize)]
pub struct RetentionResponse {
    /// Success indicator.
    pub success: bool,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The current retention configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<RetentionConfig>,
}

impl RetentionResponse {
    fn success(config: RetentionConfig) -> Self {
        Self {
            success: true,
            message: None,
            config: Some(config),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            config: None,
        }
    }
}

/// Creates retention configuration routes.
///
/// # Routes
///
/// - `GET /api/v1/config/retention` - Get current retention configuration
/// - `PUT /api/v1/config/retention` - Update complete retention configuration
/// - `PUT /api/v1/config/retention/policy` - Update a single retention policy
/// - `GET /api/v1/config/retention/metrics` - Get data age metrics
pub fn retention_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/config/retention", get(get_retention_config))
        .route("/api/v1/config/retention", put(update_retention_config))
        .route(
            "/api/v1/config/retention/policy",
            put(update_retention_policy),
        )
        .route(
            "/api/v1/config/retention/metrics",
            get(get_data_age_metrics),
        )
        .with_state(state)
}

/// Handler for GET /api/v1/config/retention.
///
/// Returns the current retention configuration for all data types.
async fn get_retention_config(State(state): State<AppState>) -> Response {
    let config = state.get_retention_config();
    Json(RetentionResponse::success(config)).into_response()
}

/// Handler for PUT /api/v1/config/retention.
///
/// Updates the complete retention configuration.
async fn update_retention_config(
    State(state): State<AppState>,
    Json(config): Json<RetentionConfig>,
) -> Response {
    // Validate the configuration
    if let Err(e) = config.validate() {
        return (StatusCode::BAD_REQUEST, Json(RetentionResponse::error(e))).into_response();
    }

    // Update ClickHouse TTL if available
    if let Err(e) = state.update_clickhouse_ttl(&config).await {
        // Only fail if we have a ClickHouse client but the operation failed
        if state.clickhouse_client().is_some() {
            tracing::error!(error = %e, "Failed to update ClickHouse TTL policies");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RetentionResponse::error(format!(
                    "Failed to update database TTL: {e}"
                ))),
            )
                .into_response();
        }
        // Otherwise just log a debug message (in-memory mode)
        tracing::debug!("ClickHouse not available, skipping TTL update");
    }

    // Update the configuration
    state.set_retention_config(config.clone());

    Json(RetentionResponse::success(config)).into_response()
}

/// Handler for PUT /api/v1/config/retention/policy.
///
/// Updates a single retention policy.
async fn update_retention_policy(
    State(state): State<AppState>,
    Json(req): Json<UpdateRetentionPolicyRequest>,
) -> Response {
    // Validate the new policy
    let policy = RetentionPolicy::new(req.data_type, req.ttl_days);
    if let Err(e) = policy.validate() {
        return (StatusCode::BAD_REQUEST, Json(RetentionResponse::error(e))).into_response();
    }

    // Update the policy
    let mut config = state.get_retention_config();
    config.update_policy(req.data_type, req.ttl_days);

    // Update ClickHouse TTL if available
    if let Err(e) = state.update_clickhouse_ttl(&config).await {
        // Only fail if we have a ClickHouse client but the operation failed
        if state.clickhouse_client().is_some() {
            tracing::error!(error = %e, "Failed to update ClickHouse TTL policies");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RetentionResponse::error(format!(
                    "Failed to update database TTL: {e}"
                ))),
            )
                .into_response();
        }
        // Otherwise just log a debug message (in-memory mode)
        tracing::debug!("ClickHouse not available, skipping TTL update");
    }

    state.set_retention_config(config.clone());

    Json(RetentionResponse::success(config)).into_response()
}

/// Handler for GET /api/v1/config/retention/metrics.
///
/// Returns current data age metrics for all data types.
async fn get_data_age_metrics(State(state): State<AppState>) -> Response {
    use crate::metrics::DataAgeMonitor;
    use std::time::Duration;

    let monitor = DataAgeMonitor::new(state, Duration::from_secs(60));
    match monitor.collect_metrics() {
        Ok(metrics) => Json(metrics).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to collect data age metrics: {}", e)
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::DataAgeMetrics;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        retention_routes(AppState::with_in_memory_store())
    }

    #[tokio::test]
    async fn test_get_retention_config() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/config/retention")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let retention_response: RetentionResponse = serde_json::from_slice(&body).unwrap();

        assert!(retention_response.success);
        assert!(retention_response.config.is_some());
        let config = retention_response.config.unwrap();
        assert_eq!(config.logs.ttl_days, 30);
        assert_eq!(config.metrics.ttl_days, 90);
        assert_eq!(config.traces.ttl_days, 30);
    }

    #[tokio::test]
    async fn test_update_retention_config_valid() {
        let app = create_test_router();

        let new_config = RetentionConfig::new(60, 180, 45);
        let json_body = serde_json::to_string(&new_config).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/config/retention")
                    .header("content-type", "application/json")
                    .body(Body::from(json_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let retention_response: RetentionResponse = serde_json::from_slice(&body).unwrap();

        assert!(retention_response.success);
        assert!(retention_response.config.is_some());
        let config = retention_response.config.unwrap();
        assert_eq!(config.logs.ttl_days, 60);
        assert_eq!(config.metrics.ttl_days, 180);
        assert_eq!(config.traces.ttl_days, 45);
    }

    #[tokio::test]
    async fn test_update_retention_config_invalid_zero_ttl() {
        let app = create_test_router();

        let invalid_config = RetentionConfig::new(0, 90, 30);
        let json_body = serde_json::to_string(&invalid_config).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/config/retention")
                    .header("content-type", "application/json")
                    .body(Body::from(json_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let retention_response: RetentionResponse = serde_json::from_slice(&body).unwrap();

        assert!(!retention_response.success);
        assert!(retention_response.message.is_some());
        assert!(retention_response
            .message
            .unwrap()
            .contains("TTL must be greater than zero"));
    }

    #[tokio::test]
    async fn test_update_retention_config_invalid_exceeds_max() {
        let app = create_test_router();

        let invalid_config = RetentionConfig::new(30, 3651, 30);
        let json_body = serde_json::to_string(&invalid_config).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/config/retention")
                    .header("content-type", "application/json")
                    .body(Body::from(json_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let retention_response: RetentionResponse = serde_json::from_slice(&body).unwrap();

        assert!(!retention_response.success);
        assert!(retention_response.message.is_some());
        assert!(retention_response.message.unwrap().contains("10 years"));
    }

    #[tokio::test]
    async fn test_update_single_retention_policy() {
        let app = create_test_router();

        let update_request = UpdateRetentionPolicyRequest {
            data_type: DataType::Logs,
            ttl_days: 60,
        };
        let json_body = serde_json::to_string(&update_request).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/config/retention/policy")
                    .header("content-type", "application/json")
                    .body(Body::from(json_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let retention_response: RetentionResponse = serde_json::from_slice(&body).unwrap();

        assert!(retention_response.success);
        assert!(retention_response.config.is_some());
        let config = retention_response.config.unwrap();
        assert_eq!(config.logs.ttl_days, 60); // Updated
        assert_eq!(config.metrics.ttl_days, 90); // Unchanged
        assert_eq!(config.traces.ttl_days, 30); // Unchanged
    }

    #[tokio::test]
    async fn test_update_single_retention_policy_invalid() {
        let app = create_test_router();

        let update_request = UpdateRetentionPolicyRequest {
            data_type: DataType::Metrics,
            ttl_days: 0,
        };
        let json_body = serde_json::to_string(&update_request).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/config/retention/policy")
                    .header("content-type", "application/json")
                    .body(Body::from(json_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let retention_response: RetentionResponse = serde_json::from_slice(&body).unwrap();

        assert!(!retention_response.success);
        assert!(retention_response.message.is_some());
    }

    #[tokio::test]
    async fn test_get_data_age_metrics() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/config/retention/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let metrics: DataAgeMetrics = serde_json::from_slice(&body).unwrap();

        // Should have metrics for all data types (even if empty)
        assert_eq!(metrics.logs.data_type, DataType::Logs);
        assert_eq!(metrics.metrics.data_type, DataType::Metrics);
        assert_eq!(metrics.traces.data_type, DataType::Traces);
    }
}
