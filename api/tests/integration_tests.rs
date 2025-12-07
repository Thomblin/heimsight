//! Integration tests for Heimsight API.
//!
//! These tests verify the complete flow of ingesting and querying
//! logs, metrics, and traces through the HTTP API.

use api::{create_router, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};

/// Creates a test router with fresh in-memory stores.
fn test_app() -> (Router, AppState) {
    let state = AppState::with_in_memory_store();
    let router = create_router(state.clone());
    (router, state)
}

/// Helper to make a POST request with JSON body.
async fn post_json(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
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
async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
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

// ============================================================================
// LOG TESTS
// ============================================================================

mod logs {
    use super::*;

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

    // ============================================================================
    // SQL-LIKE QUERY TESTS
    // ============================================================================

    mod sql_query {
        use super::*;

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
            let query = json!({"query": "SELECT * FROM logs WHERE level = 'error' AND service = 'payment'"});
            let (status, response) = post_json(app.clone(), "/api/v1/query", query).await;

            assert_eq!(status, StatusCode::OK);
            assert_eq!(response["total_count"], 1);
            assert_eq!(response["logs"][0]["message"], "Critical failure");

            // Query with OR
            let query =
                json!({"query": "SELECT * FROM logs WHERE level = 'error' OR level = 'warn'"});
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
    }

    // ============================================================================
    // METRICS TESTS
    // ============================================================================

    mod metrics {
        use super::*;

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
    }

    // ============================================================================
    // TRACES TESTS
    // ============================================================================

    mod traces {
        use super::*;

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
    }

    // ============================================================================
    // CROSS-FEATURE TESTS
    // ============================================================================

    mod cross_feature {
        use super::*;

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
    }
}
