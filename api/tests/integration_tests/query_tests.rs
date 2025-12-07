//! Integration tests for SQL-like query functionality.
//!
//! Tests cover:
//! - Basic SELECT queries with WHERE clauses
//! - AND/OR logical operators
//! - CONTAINS operator for text search
//! - LIMIT and OFFSET
//! - Error handling for invalid syntax

use axum::http::StatusCode;
use serde_json::json;

use super::common::{post_json, test_app};

#[tokio::test]
async fn test_sql_query_with_where_clause() {
    let (app, _state) = test_app();

    // Ingest test logs
    let logs = json!([
        {"level": "error", "message": "Database error", "service": "db-service"},
        {"level": "warn", "message": "Slow query", "service": "db-service"},
        {"level": "error", "message": "API timeout", "service": "api-gateway"},
        {"level": "info", "message": "Startup complete", "service": "api-gateway"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    // Query using SQL-like syntax
    let query = json!({"query": "SELECT * FROM logs WHERE level = 'error'"});
    let (status, response) = post_json(app.clone(), "/api/v1/query", query).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 2);

    let logs = response["logs"].as_array().unwrap();
    assert!(logs.iter().all(|l| l["level"] == "error"));
}

#[tokio::test]
async fn test_sql_query_with_and_or() {
    let (app, _state) = test_app();

    // Ingest test logs
    let logs = json!([
        {"level": "error", "message": "Critical failure", "service": "payment"},
        {"level": "warn", "message": "Low balance", "service": "payment"},
        {"level": "error", "message": "Network error", "service": "network"},
        {"level": "info", "message": "Success", "service": "payment"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    // Query with AND
    let query =
        json!({"query": "SELECT * FROM logs WHERE level = 'error' AND service = 'payment'"});
    let (status, response) = post_json(app.clone(), "/api/v1/query", query).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 1);
    assert_eq!(response["logs"][0]["message"], "Critical failure");

    // Query with OR
    let query = json!({"query": "SELECT * FROM logs WHERE level = 'error' OR level = 'warn'"});
    let (status, response) = post_json(app, "/api/v1/query", query).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 3);

    let logs = response["logs"].as_array().unwrap();
    assert!(logs
        .iter()
        .all(|l| l["level"] == "error" || l["level"] == "warn"));
    let messages: Vec<&str> = logs
        .iter()
        .map(|l| l["message"].as_str().unwrap())
        .collect();
    assert!(messages.contains(&"Critical failure"));
    assert!(messages.contains(&"Low balance"));
    assert!(messages.contains(&"Network error"));
}

#[tokio::test]
async fn test_sql_query_with_contains() {
    let (app, _state) = test_app();

    let logs = json!([
        {"level": "info", "message": "User john@example.com logged in", "service": "auth"},
        {"level": "info", "message": "User admin logged in", "service": "auth"},
        {"level": "error", "message": "Login failed for unknown user", "service": "auth"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    let query = json!({"query": "SELECT * FROM logs WHERE message CONTAINS 'logged in'"});
    let (status, response) = post_json(app, "/api/v1/query", query).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 2);
    let logs = response["logs"].as_array().unwrap();
    let messages: Vec<&str> = logs
        .iter()
        .map(|l| l["message"].as_str().unwrap())
        .collect();
    assert!(messages.contains(&"User john@example.com logged in"));
    assert!(messages.contains(&"User admin logged in"));
}

#[tokio::test]
async fn test_sql_query_with_limit_offset() {
    let (app, _state) = test_app();

    let logs = json!([
        {"level": "info", "message": "A", "service": "api"},
        {"level": "info", "message": "B", "service": "api"},
        {"level": "info", "message": "C", "service": "api"},
        {"level": "info", "message": "D", "service": "api"},
        {"level": "info", "message": "E", "service": "api"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    let query = json!({"query": "SELECT * FROM logs LIMIT 2 OFFSET 1"});
    let (status, response) = post_json(app, "/api/v1/query", query).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["total_count"], 5);
    assert_eq!(response["returned_count"], 2);

    let logs = response["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 2);
    let messages: Vec<&str> = logs
        .iter()
        .map(|l| l["message"].as_str().unwrap())
        .collect();
    // With OFFSET 1 and LIMIT 2, we skip the first log and get the next 2
    assert!(messages.contains(&"B"));
    assert!(messages.contains(&"C"));
}

#[tokio::test]
async fn test_sql_query_invalid_syntax_returns_error() {
    let (app, _state) = test_app();

    let query = json!({"query": "SELEKT * FROM logs"});
    let (status, response) = post_json(app, "/api/v1/query", query).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["error"], "parse_error");
}
