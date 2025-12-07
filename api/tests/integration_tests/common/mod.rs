//! Common test utilities and helpers for integration tests.
//!
//! This module provides shared functionality used across all integration tests,
//! including test app setup and HTTP request helpers.

use api::{create_router, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;

/// Creates a test router with fresh in-memory stores.
///
/// # Returns
///
/// A tuple containing the configured router and the app state.
pub fn test_app() -> (Router, AppState) {
    let state = AppState::with_in_memory_store();
    let router = create_router(state.clone());
    (router, state)
}

/// Helper to make a POST request with JSON body.
///
/// # Arguments
///
/// * `app` - The Axum router to send the request to
/// * `uri` - The URI path to POST to
/// * `body` - The JSON body to send
///
/// # Returns
///
/// A tuple containing the response status code and parsed JSON response body.
pub async fn post_json(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap();

    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);

    (status, json)
}

/// Helper to make a GET request.
///
/// # Arguments
///
/// * `app` - The Axum router to send the request to
/// * `uri` - The URI path to GET from
///
/// # Returns
///
/// A tuple containing the response status code and parsed JSON response body.
pub async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
    let response = tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);

    (status, json)
}
