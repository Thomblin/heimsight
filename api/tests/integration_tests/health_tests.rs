//! Integration tests for health check and general API functionality.
//!
//! Tests cover:
//! - Health check endpoint
//! - Empty store behavior

use axum::http::StatusCode;

use super::common::{get, test_app};

#[tokio::test]
async fn test_health_check() {
    let (app, _state) = test_app();

    let (status, response) = get(app, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["status"], "healthy");
    assert_eq!(response["service"], "heimsight-api");
}

#[tokio::test]
async fn test_empty_stores_return_empty_results() {
    let (app, _state) = test_app();

    // Query empty logs
    let (status, response) = get(app.clone(), "/api/v1/logs").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 0);
    assert!(response["logs"].as_array().unwrap().is_empty());

    // Query empty metrics
    let (status, response) = get(app.clone(), "/api/v1/metrics").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 0);

    // Query empty traces
    let (status, response) = get(app, "/api/v1/traces").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 0);
}
