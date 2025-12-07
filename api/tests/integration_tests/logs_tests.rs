//! Integration tests for log ingestion and querying.
//!
//! Tests cover:
//! - Single and batch log ingestion
//! - Filtering by level, service, and message content
//! - Pagination
//! - SQL-like query syntax

use axum::http::StatusCode;
use serde_json::json;

use super::common::{get, post_json, test_app};

#[tokio::test]
async fn test_ingest_and_query_single_log() {
    let (app, _state) = test_app();

    // Ingest a single log
    let log = json!({
        "level": "error",
        "message": "Database connection failed",
        "service": "db-service",
        "attributes": {
            "error_code": "CONN_TIMEOUT",
            "retry_count": 3
        }
    });

    let (status, response) = post_json(app.clone(), "/api/v1/logs", log).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 1);

    // Query logs back
    let (status, response) = get(app, "/api/v1/logs").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);

    let logs = response["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["level"], "error");
    assert_eq!(logs[0]["message"], "Database connection failed");
    assert_eq!(logs[0]["service"], "db-service");
    assert_eq!(logs[0]["attributes"]["error_code"], "CONN_TIMEOUT");
    assert_eq!(logs[0]["attributes"]["retry_count"], 3);
}

#[tokio::test]
async fn test_ingest_batch_and_filter_by_level() {
    let (app, _state) = test_app();

    // Ingest batch of logs with different levels
    let logs = json!([
        {"level": "info", "message": "Server started", "service": "api"},
        {"level": "warn", "message": "High memory usage", "service": "api"},
        {"level": "error", "message": "Request failed", "service": "api"},
        {"level": "info", "message": "Request completed", "service": "api"}
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 4);

    // Query only error logs
    let (status, response) = get(app.clone(), "/api/v1/logs?level=error").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);

    let logs = response["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["message"], "Request failed");

    // Query info logs
    let (status, response) = get(app, "/api/v1/logs?level=info").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 2);
}

#[tokio::test]
async fn test_filter_by_service_and_message() {
    let (app, _state) = test_app();

    // Ingest logs from different services
    let logs = json!([
        {"level": "info", "message": "User login successful", "service": "auth-service"},
        {"level": "info", "message": "User logout", "service": "auth-service"},
        {"level": "error", "message": "Payment processing failed", "service": "payment-service"},
        {"level": "info", "message": "Order created", "service": "order-service"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    // Filter by service
    let (status, response) = get(app.clone(), "/api/v1/logs?service=auth-service").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 2);

    // Filter by message content
    let (status, response) = get(app.clone(), "/api/v1/logs?contains=failed").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);

    let logs = response["logs"].as_array().unwrap();
    assert_eq!(logs[0]["service"], "payment-service");

    // Combined filter
    let (status, response) = get(app, "/api/v1/logs?service=auth-service&contains=login").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);
    assert_eq!(response["logs"][0]["message"], "User login successful");
}

#[tokio::test]
async fn test_pagination() {
    let (app, _state) = test_app();

    // Ingest 5 logs
    let logs = json!([
        {"level": "info", "message": "Log 1", "service": "api"},
        {"level": "info", "message": "Log 2", "service": "api"},
        {"level": "info", "message": "Log 3", "service": "api"},
        {"level": "info", "message": "Log 4", "service": "api"},
        {"level": "info", "message": "Log 5", "service": "api"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    // Get first 2 logs
    let (status, response) = get(app.clone(), "/api/v1/logs?limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 5);
    assert_eq!(response["logs"].as_array().unwrap().len(), 2);
    let first_page = response["logs"].as_array().unwrap();

    assert_eq!(first_page[0]["message"], "Log 1");
    assert_eq!(first_page[1]["message"], "Log 2");

    // Get next 2 logs
    let (status, response) = get(app, "/api/v1/logs?limit=2&offset=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 5);

    let second_page = response["logs"].as_array().unwrap();
    assert_eq!(second_page[0]["message"], "Log 3");
    assert_eq!(second_page[1]["message"], "Log 4");
}

#[tokio::test]
async fn test_log_validation_errors_return_400() {
    let (app, _state) = test_app();

    // Empty log message
    let invalid_log = json!({"level": "info", "message": "", "service": "api"});
    let (status, response) = post_json(app.clone(), "/api/v1/logs", invalid_log).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["error"], "validation_failed");

    // Empty service
    let invalid_log = json!({"level": "info", "message": "Hello", "service": ""});
    let (status, response) = post_json(app, "/api/v1/logs", invalid_log).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["error"], "validation_failed");
}
