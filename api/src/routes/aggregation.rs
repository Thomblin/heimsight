//! Aggregation configuration API routes.
//!
//! Provides endpoints for managing data aggregation policies for long-term storage.

use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use shared::config::AggregationConfig;

use crate::state::AppState;

/// Response body for aggregation operations.
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregationResponse {
    /// Success indicator.
    pub success: bool,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The current aggregation configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<AggregationConfig>,
}

impl AggregationResponse {
    fn success(config: AggregationConfig) -> Self {
        Self {
            success: true,
            message: None,
            config: Some(config),
        }
    }

    #[allow(dead_code)] // Will be used when PUT endpoint is added
    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            config: None,
        }
    }
}

/// Creates aggregation configuration routes.
///
/// # Routes
///
/// - `GET /api/v1/config/aggregation` - Get current aggregation configuration
pub fn aggregation_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/config/aggregation", get(get_aggregation_config))
        .with_state(state)
}

/// Handler for GET /api/v1/config/aggregation.
///
/// Returns the current aggregation configuration.
async fn get_aggregation_config(State(state): State<AppState>) -> Response {
    let config = state.get_aggregation_config();
    Json(AggregationResponse::success(config)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        aggregation_routes(AppState::with_in_memory_store())
    }

    #[tokio::test]
    async fn test_get_aggregation_config() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/config/aggregation")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let agg_response: AggregationResponse = serde_json::from_slice(&body).unwrap();

        assert!(agg_response.success);
        assert!(agg_response.config.is_some());
        let config = agg_response.config.unwrap();
        assert!(!config.enabled); // Disabled by default
        assert_eq!(config.one_minute.retention_days, 30);
        assert_eq!(config.one_hour.retention_days, 365);
    }
}

