//! Health check endpoint.
//!
//! Provides a simple health check endpoint for load balancers and monitoring systems.

use axum::{routing::get, Json, Router};
use serde::Serialize;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status (always "healthy" if reachable).
    pub status: &'static str,
    /// Service name.
    pub service: &'static str,
    /// Service version.
    pub version: &'static str,
}

/// Creates the health check routes.
pub fn health_routes() -> Router {
    Router::new().route("/health", get(health_check))
}

/// Health check handler.
///
/// Returns a simple JSON response indicating the service is healthy.
/// This endpoint is intended for use by load balancers and monitoring systems.
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        service: "heimsight-api",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check_status() {
        let app = health_routes();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_check_body() {
        let app = health_routes();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(health["status"], "healthy");
        assert_eq!(health["service"], "heimsight-api");
        assert!(health["version"].is_string());
    }
}
