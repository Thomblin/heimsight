//! Integration tests for metrics ingestion and querying.
//!
//! Tests cover:
//! - Single and batch metric ingestion
//! - Filtering by name and metric type
//! - Aggregations (sum, avg, min, max, count)

use axum::http::StatusCode;
use serde_json::json;

use super::common::{get, post_json, test_app};

#[tokio::test]
async fn test_ingest_and_query_single_metric() {
    let (app, _state) = test_app();

    let metric = json!({
        "name": "http_requests_total",
        "metric_type": "counter",
        "value": 100.0,
        "labels": {
            "method": "GET",
            "path": "/api/users"
        }
    });

    let (status, response) = post_json(app.clone(), "/api/v1/metrics", metric).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 1);

    // Query metrics back
    let (status, response) = get(app, "/api/v1/metrics").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);

    let metrics = response["metrics"].as_array().unwrap();
    assert_eq!(metrics[0]["name"], "http_requests_total");
    assert_eq!(metrics[0]["metric_type"], "counter");
    assert_eq!(metrics[0]["value"], 100.0);
    assert_eq!(metrics[0]["labels"]["method"], "GET");
    assert_eq!(metrics[0]["labels"]["path"], "/api/users");
}

#[tokio::test]
async fn test_ingest_batch_and_filter_by_name() {
    let (app, _state) = test_app();

    let metrics = json!([
        {"name": "cpu_usage", "value": 75.5, "labels": {"host": "server-1"}},
        {"name": "cpu_usage", "value": 82.3, "labels": {"host": "server-2"}},
        {"name": "memory_usage", "value": 60.0, "labels": {"host": "server-1"}},
        {"name": "disk_usage", "value": 45.0, "labels": {"host": "server-1"}}
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 4);

    // Filter by name
    let (status, response) = get(app, "/api/v1/metrics?name=cpu_usage").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 2);

    let metrics = response["metrics"].as_array().unwrap();
    assert!(metrics.iter().all(|m| m["name"] == "cpu_usage"));
    let values: Vec<f64> = metrics
        .iter()
        .map(|m| m["value"].as_f64().unwrap())
        .collect();
    assert!(values.contains(&75.5));
    assert!(values.contains(&82.3));
}

#[tokio::test]
async fn test_metric_aggregations() {
    let (app, _state) = test_app();

    // Ingest metrics for aggregation
    let metrics = json!([
        {"name": "response_time_ms", "value": 100.0, "labels": {"endpoint": "/api"}},
        {"name": "response_time_ms", "value": 200.0, "labels": {"endpoint": "/api"}},
        {"name": "response_time_ms", "value": 300.0, "labels": {"endpoint": "/api"}},
        {"name": "response_time_ms", "value": 400.0, "labels": {"endpoint": "/api"}}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);

    // Test SUM
    let (status, response) = get(
        app.clone(),
        "/api/v1/metrics?name=response_time_ms&aggregate=sum",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["aggregation"]["value"], 1000.0);
    assert_eq!(response["aggregation"]["count"], 4);

    // Test AVG
    let (status, response) = get(
        app.clone(),
        "/api/v1/metrics?name=response_time_ms&aggregate=avg",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["aggregation"]["value"], 250.0);

    // Test MIN
    let (status, response) = get(
        app.clone(),
        "/api/v1/metrics?name=response_time_ms&aggregate=min",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["aggregation"]["value"], 100.0);

    // Test MAX
    let (status, response) = get(
        app.clone(),
        "/api/v1/metrics?name=response_time_ms&aggregate=max",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["aggregation"]["value"], 400.0);

    // Test COUNT
    let (status, response) =
        get(app, "/api/v1/metrics?name=response_time_ms&aggregate=count").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["aggregation"]["value"], 4.0);
}

#[tokio::test]
async fn test_filter_by_metric_type() {
    let (app, _state) = test_app();

    let metrics = json!([
        {"name": "requests_total", "metric_type": "counter", "value": 500.0},
        {"name": "cpu_percent", "metric_type": "gauge", "value": 75.0},
        {"name": "errors_total", "metric_type": "counter", "value": 10.0}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);

    // Filter by counter type
    let (status, response) = get(app.clone(), "/api/v1/metrics?metric_type=counter").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 2);

    // Filter by gauge type
    let (status, response) = get(app, "/api/v1/metrics?metric_type=gauge").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);
    assert_eq!(response["metrics"][0]["name"], "cpu_percent");
}

#[tokio::test]
async fn test_empty_metrics_store_returns_empty_results() {
    let (app, _state) = test_app();

    // Query empty metrics
    let (status, response) = get(app, "/api/v1/metrics").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 0);
}
