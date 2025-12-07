//! Integration tests for trace ingestion and querying.
//!
//! Tests cover:
//! - Ingesting spans and retrieving complete traces
//! - Querying traces by service
//! - Filtering by duration (min/max)
//! - Parent-child span relationships
//! - Error handling for non-existent traces

use axum::http::StatusCode;
use serde_json::json;

use super::common::{get, post_json, test_app};

#[tokio::test]
async fn test_ingest_and_get_trace_by_id() {
    let (app, _state) = test_app();

    // Ingest spans forming a complete trace
    let spans = json!([
        {
            "trace_id": "trace-abc123",
            "span_id": "span-root",
            "name": "HTTP GET /users",
            "service": "api-gateway",
            "duration_ms": 150,
            "status": "ok"
        },
        {
            "trace_id": "trace-abc123",
            "span_id": "span-db",
            "parent_span_id": "span-root",
            "name": "SELECT * FROM users",
            "service": "user-service",
            "duration_ms": 80,
            "status": "ok",
            "attributes": {"db.system": "postgresql"}
        }
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 2);

    // Get trace by ID
    let (status, response) = get(app, "/api/v1/traces/trace-abc123").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["trace_id"], "trace-abc123");

    let spans = response["spans"].as_array().unwrap();
    assert_eq!(spans.len(), 2);

    // Verify span details
    let root_span = spans.iter().find(|s| s["span_id"] == "span-root").unwrap();
    assert_eq!(root_span["name"], "HTTP GET /users");
    assert_eq!(root_span["service"], "api-gateway");
    assert!(root_span["parent_span_id"].is_null());

    let db_span = spans.iter().find(|s| s["span_id"] == "span-db").unwrap();
    assert_eq!(db_span["name"], "SELECT * FROM users");
    assert_eq!(db_span["parent_span_id"], "span-root");
    assert_eq!(db_span["attributes"]["db.system"], "postgresql");
}

#[tokio::test]
async fn test_query_traces_by_service() {
    let (app, _state) = test_app();

    // Ingest traces from different services
    let spans = json!([
        {"trace_id": "trace-1", "span_id": "s1", "name": "Request 1", "service": "api-gateway", "duration_ms": 100},
        {"trace_id": "trace-2", "span_id": "s2", "name": "Request 2", "service": "api-gateway", "duration_ms": 200},
        {"trace_id": "trace-3", "span_id": "s3", "name": "Request 3", "service": "payment-service", "duration_ms": 300}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);

    // Query by service
    let (status, response) = get(app, "/api/v1/traces?service=api-gateway").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 2);

    let traces = response["traces"].as_array().unwrap();
    let trace_ids: Vec<&str> = traces
        .iter()
        .map(|t| t["trace_id"].as_str().unwrap())
        .collect();
    assert!(trace_ids.contains(&"trace-1"));
    assert!(trace_ids.contains(&"trace-2"));
}

#[tokio::test]
async fn test_query_traces_by_duration() {
    let (app, _state) = test_app();

    let spans = json!([
        {"trace_id": "fast", "span_id": "s1", "name": "Fast request", "service": "api", "duration_ms": 10},
        {"trace_id": "medium", "span_id": "s2", "name": "Medium request", "service": "api", "duration_ms": 100},
        {"trace_id": "slow", "span_id": "s3", "name": "Slow request", "service": "api", "duration_ms": 1000}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);

    // Query slow traces (> 500ms)
    let (status, response) = get(app.clone(), "/api/v1/traces?min_duration_ms=500").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);
    assert_eq!(response["traces"][0]["trace_id"], "slow");

    // Query fast traces (< 50ms)
    let (status, response) = get(app, "/api/v1/traces?max_duration_ms=50").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);
    assert_eq!(response["traces"][0]["trace_id"], "fast");
}

#[tokio::test]
async fn test_trace_not_found_returns_404() {
    let (app, _state) = test_app();

    let (status, response) = get(app, "/api/v1/traces/nonexistent-trace").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(response["error"], "not_found");
}

#[tokio::test]
async fn test_trace_with_multiple_spans() {
    let (app, _state) = test_app();

    // Create a realistic trace with parent-child relationships
    let spans = json!([
        {
            "trace_id": "order-trace",
            "span_id": "root",
            "name": "POST /orders",
            "service": "api-gateway",
            "duration_ms": 500
        },
        {
            "trace_id": "order-trace",
            "span_id": "auth",
            "parent_span_id": "root",
            "name": "Authenticate",
            "service": "auth-service",
            "duration_ms": 50
        },
        {
            "trace_id": "order-trace",
            "span_id": "validate",
            "parent_span_id": "root",
            "name": "Validate order",
            "service": "order-service",
            "duration_ms": 30
        },
        {
            "trace_id": "order-trace",
            "span_id": "db-insert",
            "parent_span_id": "validate",
            "name": "INSERT INTO orders",
            "service": "order-service",
            "duration_ms": 100
        },
        {
            "trace_id": "order-trace",
            "span_id": "notify",
            "parent_span_id": "root",
            "name": "Send notification",
            "service": "notification-service",
            "duration_ms": 200
        }
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 5);

    // Get the complete trace
    let (status, response) = get(app, "/api/v1/traces/order-trace").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["trace_id"], "order-trace");

    let spans = response["spans"].as_array().unwrap();
    assert_eq!(spans.len(), 5);

    // Verify services are present
    let services: Vec<&str> = spans
        .iter()
        .map(|s| s["service"].as_str().unwrap())
        .collect();
    assert!(services.contains(&"api-gateway"));
    assert!(services.contains(&"auth-service"));
    assert!(services.contains(&"order-service"));
    assert!(services.contains(&"notification-service"));
}

#[tokio::test]
async fn test_empty_traces_store_returns_empty_results() {
    let (app, _state) = test_app();

    // Query empty traces
    let (status, response) = get(app, "/api/v1/traces").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 0);
}
