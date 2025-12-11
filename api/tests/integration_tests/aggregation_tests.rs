//! Integration tests for ClickHouse aggregation features.
//!
//! Tests cover materialized views that aggregate:
//! - Metrics: 1-minute, 5-minute, 1-hour, and 1-day aggregates
//! - Logs: Hourly and daily log counts by level, service, and normalized message
//! - Spans: Hourly and daily span statistics (latency percentiles, throughput)
//! - Traces: Hourly and daily trace statistics (unique traces, spans per trace)
//!
//! These tests require a running ClickHouse instance with the schema applied.
//! Run with: `cargo test -- --ignored`

use axum::http::StatusCode;
use serde_json::json;
use std::time::Duration;

use super::common::{create_clickhouse_client, get, post_json, test_app_with_clickhouse};

// ============================================================================
// AGGREGATION CONFIG API TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running ClickHouse instance"]
async fn test_get_aggregation_config_with_clickhouse() {
    let (app, _state) = test_app_with_clickhouse();

    let (status, response) = get(app, "/api/v1/config/aggregation").await;

    assert_eq!(status, StatusCode::OK);
    assert!(response["success"].as_bool().unwrap());
    assert!(response["config"].is_object());

    let config = &response["config"];
    assert!(!config["enabled"].as_bool().unwrap()); // Disabled by default
    assert_eq!(config["one_minute"]["retention_days"], 30);
    assert_eq!(config["five_minutes"]["retention_days"], 90);
    assert_eq!(config["one_hour"]["retention_days"], 365);
    assert_eq!(config["one_day"]["retention_days"], 730);
}

// ============================================================================
// METRIC AGGREGATION TESTS
// ============================================================================

/// Helper to clean up test data before/after tests.
async fn cleanup_test_data(client: &clickhouse::Client) {
    // Clean up raw tables
    let _ = client.query("TRUNCATE TABLE logs").execute().await;
    let _ = client.query("TRUNCATE TABLE metrics").execute().await;
    let _ = client.query("TRUNCATE TABLE spans").execute().await;

    // Clean up aggregation tables
    let _ = client.query("TRUNCATE TABLE metrics_1min").execute().await;
    let _ = client.query("TRUNCATE TABLE metrics_5min").execute().await;
    let _ = client.query("TRUNCATE TABLE metrics_1hour").execute().await;
    let _ = client.query("TRUNCATE TABLE metrics_1day").execute().await;
    let _ = client
        .query("TRUNCATE TABLE logs_1hour_counts")
        .execute()
        .await;
    let _ = client
        .query("TRUNCATE TABLE logs_1day_counts")
        .execute()
        .await;
    let _ = client
        .query("TRUNCATE TABLE spans_1hour_stats")
        .execute()
        .await;
    let _ = client
        .query("TRUNCATE TABLE spans_1day_stats")
        .execute()
        .await;
    let _ = client
        .query("TRUNCATE TABLE traces_1hour_stats")
        .execute()
        .await;
    let _ = client
        .query("TRUNCATE TABLE traces_1day_stats")
        .execute()
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_metrics_1min_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest metrics with same name and service
    let metrics = json!([
        {"name": "cpu_usage", "metric_type": "gauge", "value": 50.0, "labels": {"service": "test-service", "host": "server1"}},
        {"name": "cpu_usage", "metric_type": "gauge", "value": 75.0, "labels": {"service": "test-service", "host": "server1"}},
        {"name": "cpu_usage", "metric_type": "gauge", "value": 100.0, "labels": {"service": "test-service", "host": "server1"}},
        {"name": "cpu_usage", "metric_type": "gauge", "value": 25.0, "labels": {"service": "test-service", "host": "server1"}}
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 4);

    // Wait for materialized view to process (ClickHouse is async)
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query the 1-minute aggregation table
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    #[allow(dead_code)] // Fields used in assertions
    struct MetricAgg {
        name: String,
        service: String,
        count: u64,
        sum: f64,
        min: f64,
        max: f64,
    }

    let result: Vec<MetricAgg> = client
        .query(
            "SELECT name, service, count, sum, min, max FROM metrics_1min WHERE name = 'cpu_usage'",
        )
        .fetch_all()
        .await
        .expect("Failed to query metrics_1min");

    // Verify aggregation results
    assert!(
        !result.is_empty(),
        "Expected aggregated metrics in metrics_1min"
    );

    let total_count: u64 = result.iter().map(|r| r.count).sum();
    let total_sum: f64 = result.iter().map(|r| r.sum).sum();

    assert_eq!(total_count, 4, "Expected count of 4 metrics");
    assert!((total_sum - 250.0).abs() < 0.01, "Expected sum of 250.0");

    // Verify min/max across all rows
    let min_value: f64 = result.iter().map(|r| r.min).fold(f64::INFINITY, f64::min);
    let max_value: f64 = result
        .iter()
        .map(|r| r.max)
        .fold(f64::NEG_INFINITY, f64::max);

    assert!((min_value - 25.0).abs() < 0.01, "Expected min of 25.0");
    assert!((max_value - 100.0).abs() < 0.01, "Expected max of 100.0");

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running ClickHouse instance"]
async fn test_metrics_aggregation_by_service() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest metrics from different services
    let metrics = json!([
        {"name": "request_count", "metric_type": "counter", "value": 100.0, "labels": {"service": "api-service"}},
        {"name": "request_count", "metric_type": "counter", "value": 200.0, "labels": {"service": "api-service"}},
        {"name": "request_count", "metric_type": "counter", "value": 50.0, "labels": {"service": "worker-service"}},
        {"name": "request_count", "metric_type": "counter", "value": 150.0, "labels": {"service": "worker-service"}}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query aggregations grouped by service
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    struct ServiceMetricAgg {
        service: String,
        count: u64,
        sum: f64,
    }

    let result: Vec<ServiceMetricAgg> = client
        .query("SELECT service, sum(count) as count, sum(sum) as sum FROM metrics_1min WHERE name = 'request_count' GROUP BY service ORDER BY service")
        .fetch_all()
        .await
        .expect("Failed to query metrics_1min by service");

    assert_eq!(result.len(), 2, "Expected 2 services");

    let api_service = result.iter().find(|r| r.service == "api-service").unwrap();
    let worker_service = result
        .iter()
        .find(|r| r.service == "worker-service")
        .unwrap();

    assert_eq!(api_service.count, 2);
    assert!((api_service.sum - 300.0).abs() < 0.01);

    assert_eq!(worker_service.count, 2);
    assert!((worker_service.sum - 200.0).abs() < 0.01);

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_metrics_hourly_aggregation_exists() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest metrics
    let metrics = json!([
        {"name": "memory_usage_mb", "metric_type": "gauge", "value": 512.0, "labels": {"service": "test-svc"}},
        {"name": "memory_usage_mb", "metric_type": "gauge", "value": 768.0, "labels": {"service": "test-svc"}}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify data exists in hourly aggregation table
    let count: u64 = client
        .query("SELECT count() FROM metrics_1hour WHERE name = 'memory_usage_mb'")
        .fetch_one()
        .await
        .expect("Failed to query metrics_1hour");

    assert!(count > 0, "Expected data in metrics_1hour table");

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running ClickHouse instance"]
async fn test_metrics_daily_aggregation_exists() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest metrics
    let metrics = json!([
        {"name": "disk_usage_gb", "metric_type": "gauge", "value": 100.0, "labels": {"service": "storage-svc"}},
        {"name": "disk_usage_gb", "metric_type": "gauge", "value": 150.0, "labels": {"service": "storage-svc"}}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify data exists in daily aggregation table
    let count: u64 = client
        .query("SELECT count() FROM metrics_1day WHERE name = 'disk_usage_gb'")
        .fetch_one()
        .await
        .expect("Failed to query metrics_1day");

    assert!(count > 0, "Expected data in metrics_1day table");

    cleanup_test_data(&client).await;
}

// ============================================================================
// LOG AGGREGATION TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_logs_hourly_count_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest logs with different levels
    let logs = json!([
        {"level": "error", "message": "Connection timeout", "service": "db-service"},
        {"level": "error", "message": "Connection refused", "service": "db-service"},
        {"level": "error", "message": "Connection reset", "service": "db-service"},
        {"level": "warn", "message": "High memory usage", "service": "db-service"},
        {"level": "info", "message": "Request completed", "service": "api-service"}
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 5);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query hourly log counts
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    struct LogCountAgg {
        level: String,
        service: String,
        count: u64,
    }

    let result: Vec<LogCountAgg> = client
        .query("SELECT level, service, sum(count) as count FROM logs_1hour_counts GROUP BY level, service ORDER BY count DESC")
        .fetch_all()
        .await
        .expect("Failed to query logs_1hour_counts");

    assert!(!result.is_empty(), "Expected log count aggregations");

    // Find error logs for db-service
    let db_errors = result
        .iter()
        .find(|r| r.level == "error" && r.service == "db-service");
    assert!(
        db_errors.is_some(),
        "Expected error logs for db-service in aggregations"
    );
    assert_eq!(
        db_errors.unwrap().count,
        3,
        "Expected 3 error logs for db-service"
    );

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running ClickHouse instance"]
async fn test_logs_daily_count_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest logs
    let logs = json!([
        {"level": "error", "message": "Payment failed for user 123", "service": "payment-service"},
        {"level": "error", "message": "Payment failed for user 456", "service": "payment-service"},
        {"level": "info", "message": "Payment succeeded", "service": "payment-service"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify data exists in daily aggregation table
    let count: u64 = client
        .query("SELECT count() FROM logs_1day_counts WHERE service = 'payment-service'")
        .fetch_one()
        .await
        .expect("Failed to query logs_1day_counts");

    assert!(count > 0, "Expected data in logs_1day_counts table");

    // Also verify the normalized message grouping in hourly table
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct HourlyCount {
        normalized_message: String,
        count: u64,
    }

    let hourly_result: Vec<HourlyCount> = client
        .query("SELECT normalized_message, sum(count) as count FROM logs_1hour_counts WHERE service = 'payment-service' GROUP BY normalized_message ORDER BY normalized_message")
        .fetch_all()
        .await
        .expect("Failed to query logs_1hour_counts");

    assert!(!hourly_result.is_empty(), "Expected hourly aggregations");

    // Verify error logs are grouped together
    let error_group = hourly_result
        .iter()
        .find(|r| r.normalized_message.contains("Payment failed"));
    assert!(
        error_group.is_some(),
        "Expected normalized group for 'Payment failed' messages"
    );
    assert_eq!(
        error_group.unwrap().count,
        2,
        "Expected 2 'Payment failed' messages grouped together"
    );

    // Verify info log is separate
    let info_group = hourly_result
        .iter()
        .find(|r| r.normalized_message.contains("Payment succeeded"));
    assert!(
        info_group.is_some(),
        "Expected normalized group for 'Payment succeeded' message"
    );
    assert_eq!(
        info_group.unwrap().count,
        1,
        "Expected 1 'Payment succeeded' message"
    );

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_logs_normalized_message_grouping() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest similar log messages that should be normalized together
    let logs = json!([
        {"level": "error", "message": "Failed to process order 12345", "service": "order-service"},
        {"level": "error", "message": "Failed to process order 67890", "service": "order-service"},
        {"level": "error", "message": "Failed to process order 11111", "service": "order-service"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/logs", logs).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query to verify logs are grouped
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    #[allow(dead_code)] // Fields used in assertions
    struct NormalizedLogCount {
        normalized_message: String,
        count: u64,
        sample_message: String,
    }

    let result: Vec<NormalizedLogCount> = client
        .query("SELECT normalized_message, sum(count) as count, any(sample_message) as sample_message FROM logs_1hour_counts WHERE service = 'order-service' GROUP BY normalized_message")
        .fetch_all()
        .await
        .expect("Failed to query logs_1hour_counts for normalized messages");

    assert!(!result.is_empty(), "Expected normalized log aggregations");

    // Verify we have sample messages preserved
    let has_sample = result.iter().any(|r| !r.sample_message.is_empty());
    assert!(has_sample, "Expected sample_message to be preserved");

    cleanup_test_data(&client).await;
}

// ============================================================================
// SPAN/TRACE AGGREGATION TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_spans_hourly_stats_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest spans with various durations
    let spans = json!([
        {
            "trace_id": "trace-agg-1",
            "span_id": "span-1",
            "name": "HTTP GET /users",
            "service": "api-gateway",
            "duration_ms": 100,
            "status": "ok"
        },
        {
            "trace_id": "trace-agg-2",
            "span_id": "span-2",
            "name": "HTTP GET /users",
            "service": "api-gateway",
            "duration_ms": 150,
            "status": "ok"
        },
        {
            "trace_id": "trace-agg-3",
            "span_id": "span-3",
            "name": "HTTP GET /users",
            "service": "api-gateway",
            "duration_ms": 200,
            "status": "ok"
        },
        {
            "trace_id": "trace-agg-4",
            "span_id": "span-4",
            "name": "HTTP GET /users",
            "service": "api-gateway",
            "duration_ms": 500,
            "status": "error"
        }
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 4);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query hourly span statistics
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    #[allow(dead_code)] // Fields used in assertions
    struct SpanStats {
        service: String,
        operation: String,
        span_count: u64,
        avg_duration_ns: f64,
        min_duration_ns: u64,
        max_duration_ns: u64,
    }

    let result: Vec<SpanStats> = client
        .query("SELECT service, operation, sum(span_count) as span_count, avg(avg_duration_ns) as avg_duration_ns, min(min_duration_ns) as min_duration_ns, max(max_duration_ns) as max_duration_ns FROM spans_1hour_stats WHERE service = 'api-gateway' GROUP BY service, operation")
        .fetch_all()
        .await
        .expect("Failed to query spans_1hour_stats");

    assert!(
        !result.is_empty(),
        "Expected span statistics in aggregation"
    );

    let stats = &result[0];
    assert_eq!(stats.span_count, 4, "Expected 4 spans");

    // Verify duration statistics (input was in ms, stored in ns)
    assert!(
        stats.min_duration_ns >= 100_000_000,
        "Expected min duration around 100ms"
    );
    assert!(
        stats.max_duration_ns >= 500_000_000,
        "Expected max duration around 500ms"
    );

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running ClickHouse instance"]
async fn test_spans_daily_stats_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest spans
    let spans = json!([
        {
            "trace_id": "trace-daily-1",
            "span_id": "span-d1",
            "name": "Database Query",
            "service": "db-service",
            "duration_ms": 50,
            "status": "ok"
        },
        {
            "trace_id": "trace-daily-2",
            "span_id": "span-d2",
            "name": "Database Query",
            "service": "db-service",
            "duration_ms": 75,
            "status": "ok"
        }
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify data exists in daily aggregation table
    let count: u64 = client
        .query("SELECT count() FROM spans_1day_stats WHERE service = 'db-service'")
        .fetch_one()
        .await
        .expect("Failed to query spans_1day_stats");

    assert!(count > 0, "Expected data in spans_1day_stats table");

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_spans_status_code_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest spans with different status codes
    let spans = json!([
        {"trace_id": "t1", "span_id": "s1", "name": "request", "service": "api", "duration_ms": 100, "status": "ok"},
        {"trace_id": "t2", "span_id": "s2", "name": "request", "service": "api", "duration_ms": 100, "status": "ok"},
        {"trace_id": "t3", "span_id": "s3", "name": "request", "service": "api", "duration_ms": 100, "status": "ok"},
        {"trace_id": "t4", "span_id": "s4", "name": "request", "service": "api", "duration_ms": 500, "status": "error"}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query aggregated stats by status code
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    struct StatusStats {
        status_code: String,
        span_count: u64,
    }

    let result: Vec<StatusStats> = client
        .query("SELECT status_code, sum(span_count) as span_count FROM spans_1hour_stats WHERE service = 'api' GROUP BY status_code")
        .fetch_all()
        .await
        .expect("Failed to query spans_1hour_stats by status");

    // Verify we have aggregations by status code
    let ok_count: u64 = result
        .iter()
        .filter(|r| r.status_code == "ok")
        .map(|r| r.span_count)
        .sum();
    let error_count: u64 = result
        .iter()
        .filter(|r| r.status_code == "error")
        .map(|r| r.span_count)
        .sum();

    assert_eq!(ok_count, 3, "Expected 3 OK spans");
    assert_eq!(error_count, 1, "Expected 1 error span");

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_traces_hourly_stats_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest multiple spans from different traces
    let spans = json!([
        {"trace_id": "trace-stats-1", "span_id": "s1", "name": "root", "service": "api", "duration_ms": 100},
        {"trace_id": "trace-stats-1", "span_id": "s2", "parent_span_id": "s1", "name": "child1", "service": "api", "duration_ms": 50},
        {"trace_id": "trace-stats-1", "span_id": "s3", "parent_span_id": "s1", "name": "child2", "service": "api", "duration_ms": 40},
        {"trace_id": "trace-stats-2", "span_id": "s4", "name": "root", "service": "api", "duration_ms": 200},
        {"trace_id": "trace-stats-2", "span_id": "s5", "parent_span_id": "s4", "name": "child", "service": "api", "duration_ms": 150},
        {"trace_id": "trace-stats-3", "span_id": "s6", "name": "root", "service": "api", "duration_ms": 50}
    ]);

    let (status, response) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(response["accepted"], 6);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query hourly trace statistics
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    #[allow(dead_code)] // Fields used in assertions
    struct TraceStats {
        service: String,
        unique_traces: u64,
        total_spans: u64,
    }

    let result: Vec<TraceStats> = client
        .query("SELECT service, sum(unique_traces) as unique_traces, sum(total_spans) as total_spans FROM traces_1hour_stats WHERE service = 'api' GROUP BY service")
        .fetch_all()
        .await
        .expect("Failed to query traces_1hour_stats");

    assert!(!result.is_empty(), "Expected trace statistics");

    let stats = &result[0];
    // Note: unique_traces uses HyperLogLog (uniq), so it's approximate
    assert!(
        stats.unique_traces >= 3,
        "Expected at least 3 unique traces"
    );
    assert_eq!(stats.total_spans, 6, "Expected 6 total spans");

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_traces_daily_stats_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest spans
    let spans = json!([
        {"trace_id": "daily-trace-1", "span_id": "ds1", "name": "op", "service": "daily-svc", "duration_ms": 100},
        {"trace_id": "daily-trace-2", "span_id": "ds2", "name": "op", "service": "daily-svc", "duration_ms": 100}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/traces", spans).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify data exists in daily trace stats table
    let count: u64 = client
        .query("SELECT count() FROM traces_1day_stats WHERE service = 'daily-svc'")
        .fetch_one()
        .await
        .expect("Failed to query traces_1day_stats");

    assert!(count > 0, "Expected data in traces_1day_stats table");

    cleanup_test_data(&client).await;
}

// ============================================================================
// CROSS-SERVICE AGGREGATION TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running ClickHouse instance"]
async fn test_multi_service_metric_aggregation() {
    let (app, _state) = test_app_with_clickhouse();
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Ingest metrics from multiple services
    let metrics = json!([
        {"name": "http_requests_total", "metric_type": "counter", "value": 100.0, "labels": {"service": "frontend", "method": "GET"}},
        {"name": "http_requests_total", "metric_type": "counter", "value": 200.0, "labels": {"service": "frontend", "method": "POST"}},
        {"name": "http_requests_total", "metric_type": "counter", "value": 150.0, "labels": {"service": "backend", "method": "GET"}},
        {"name": "http_requests_total", "metric_type": "counter", "value": 50.0, "labels": {"service": "backend", "method": "DELETE"}}
    ]);

    let (status, _) = post_json(app.clone(), "/api/v1/metrics", metrics).await;
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query all services
    #[derive(clickhouse::Row, serde::Deserialize, Debug)]
    struct ServiceSum {
        service: String,
        total: f64,
    }

    let result: Vec<ServiceSum> = client
        .query("SELECT service, sum(sum) as total FROM metrics_1min WHERE name = 'http_requests_total' GROUP BY service ORDER BY service")
        .fetch_all()
        .await
        .expect("Failed to query multi-service metrics");

    assert_eq!(result.len(), 2, "Expected 2 services");

    let backend = result.iter().find(|r| r.service == "backend").unwrap();
    let frontend = result.iter().find(|r| r.service == "frontend").unwrap();

    assert!((backend.total - 200.0).abs() < 0.01);
    assert!((frontend.total - 300.0).abs() < 0.01);

    cleanup_test_data(&client).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires running ClickHouse instance"]
async fn test_empty_aggregation_tables_query() {
    let client = create_clickhouse_client();

    cleanup_test_data(&client).await;

    // Query empty tables - should return 0, not error
    let metric_count: u64 = client
        .query("SELECT count() FROM metrics_1min")
        .fetch_one()
        .await
        .expect("Failed to query empty metrics_1min");

    let log_count: u64 = client
        .query("SELECT count() FROM logs_1hour_counts")
        .fetch_one()
        .await
        .expect("Failed to query empty logs_1hour_counts");

    let span_count: u64 = client
        .query("SELECT count() FROM spans_1hour_stats")
        .fetch_one()
        .await
        .expect("Failed to query empty spans_1hour_stats");

    let trace_count: u64 = client
        .query("SELECT count() FROM traces_1hour_stats")
        .fetch_one()
        .await
        .expect("Failed to query empty traces_1hour_stats");

    assert_eq!(metric_count, 0);
    assert_eq!(log_count, 0);
    assert_eq!(span_count, 0);
    assert_eq!(trace_count, 0);
}
