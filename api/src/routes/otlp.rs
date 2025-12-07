//! OTLP HTTP receiver endpoints.
//!
//! Implements OpenTelemetry Protocol HTTP endpoints for ingesting logs, metrics, and traces.
//! Supports both protobuf (`application/x-protobuf`) and JSON (`application/json`) content types.
//!
//! # Endpoints
//!
//! - `POST /v1/logs` - Ingest OTLP logs
//! - `POST /v1/metrics` - Ingest OTLP metrics
//! - `POST /v1/traces` - Ingest OTLP traces

use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use prost::Message;
use serde::{Deserialize, Serialize};
use shared::otlp::conversions::{
    otlp_log_to_log_entry, otlp_metrics_to_metrics, otlp_span_to_span,
};
use shared::otlp::proto;
use std::collections::HashMap;

/// Content type for protobuf requests.
const CONTENT_TYPE_PROTOBUF: &str = "application/x-protobuf";

/// Response for OTLP export requests.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResponse {
    /// Number of items accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_success: Option<PartialSuccess>,
}

/// Partial success information.
#[derive(Debug, Serialize, Deserialize)]
pub struct PartialSuccess {
    /// Number of rejected items.
    pub rejected_count: i64,
    /// Error message if any items were rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Error response for OTLP endpoints.
#[derive(Debug, Serialize, Deserialize)]
pub struct OtlpError {
    /// Error code.
    pub code: u32,
    /// Error message.
    pub message: String,
}

/// Creates the OTLP routes with application state.
pub fn otlp_routes(state: AppState) -> Router {
    Router::new()
        .route("/v1/logs", post(ingest_logs))
        .route("/v1/metrics", post(ingest_metrics))
        .route("/v1/traces", post(ingest_traces))
        .with_state(state)
}

/// Determines if the request is protobuf based on Content-Type header.
fn is_protobuf(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with(CONTENT_TYPE_PROTOBUF))
}

/// Extracts resource attributes from OTLP resource.
fn extract_resource_attrs(
    resource: Option<&proto::resource::v1::Resource>,
) -> HashMap<String, serde_json::Value> {
    resource
        .map(|r| {
            r.attributes
                .iter()
                .filter_map(|kv| {
                    kv.value.as_ref().map(|v| {
                        let json_value = any_value_to_json(v);
                        (kv.key.clone(), json_value)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Converts OTLP `AnyValue` to `serde_json::Value`.
fn any_value_to_json(value: &proto::common::v1::AnyValue) -> serde_json::Value {
    use proto::common::v1::any_value::Value;

    match &value.value {
        Some(Value::StringValue(s)) => serde_json::Value::String(s.clone()),
        Some(Value::BoolValue(b)) => serde_json::Value::Bool(*b),
        Some(Value::IntValue(i)) => serde_json::Value::Number((*i).into()),
        Some(Value::DoubleValue(d)) => serde_json::Number::from_f64(*d)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        Some(Value::ArrayValue(arr)) => {
            let values: Vec<serde_json::Value> = arr.values.iter().map(any_value_to_json).collect();
            serde_json::Value::Array(values)
        }
        Some(Value::KvlistValue(kv)) => {
            let mut map = serde_json::Map::new();
            for pair in &kv.values {
                if let Some(ref v) = pair.value {
                    map.insert(pair.key.clone(), any_value_to_json(v));
                }
            }
            serde_json::Value::Object(map)
        }
        Some(Value::BytesValue(b)) => {
            use base64::Engine;
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
        None => serde_json::Value::Null,
    }
}

/// Handler for OTLP logs ingestion.
///
/// Accepts `ExportLogsServiceRequest` in protobuf or JSON format.
async fn ingest_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<ExportResponse>), (StatusCode, Json<OtlpError>)> {
    let request = if is_protobuf(&headers) {
        proto::collector::logs::v1::ExportLogsServiceRequest::decode(body).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode protobuf logs request");
            (
                StatusCode::BAD_REQUEST,
                Json(OtlpError {
                    code: 400,
                    message: format!("Failed to decode protobuf: {e}"),
                }),
            )
        })?
    } else {
        serde_json::from_slice(&body).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode JSON logs request");
            (
                StatusCode::BAD_REQUEST,
                Json(OtlpError {
                    code: 400,
                    message: format!("Failed to decode JSON: {e}"),
                }),
            )
        })?
    };

    let mut accepted = 0;
    let mut rejected = 0;

    for resource_logs in &request.resource_logs {
        let resource_attrs = extract_resource_attrs(resource_logs.resource.as_ref());

        for scope_logs in &resource_logs.scope_logs {
            let scope_name = scope_logs
                .scope
                .as_ref()
                .map_or("unknown", |s| s.name.as_str());

            for log_record in &scope_logs.log_records {
                if let Some(log_entry) =
                    otlp_log_to_log_entry(log_record, &resource_attrs, scope_name)
                {
                    if let Err(e) = state.log_store().insert(log_entry) {
                        tracing::error!(error = %e, "Failed to store log entry");
                        rejected += 1;
                    } else {
                        accepted += 1;
                    }
                } else {
                    rejected += 1;
                }
            }
        }
    }

    tracing::debug!(accepted, rejected, "Processed OTLP logs");

    let response = if rejected > 0 {
        ExportResponse {
            partial_success: Some(PartialSuccess {
                rejected_count: rejected,
                error_message: Some(format!("{rejected} log records were rejected")),
            }),
        }
    } else {
        ExportResponse {
            partial_success: None,
        }
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for OTLP metrics ingestion.
///
/// Accepts `ExportMetricsServiceRequest` in protobuf or JSON format.
async fn ingest_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<ExportResponse>), (StatusCode, Json<OtlpError>)> {
    let request = if is_protobuf(&headers) {
        proto::collector::metrics::v1::ExportMetricsServiceRequest::decode(body).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode protobuf metrics request");
            (
                StatusCode::BAD_REQUEST,
                Json(OtlpError {
                    code: 400,
                    message: format!("Failed to decode protobuf: {e}"),
                }),
            )
        })?
    } else {
        serde_json::from_slice(&body).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode JSON metrics request");
            (
                StatusCode::BAD_REQUEST,
                Json(OtlpError {
                    code: 400,
                    message: format!("Failed to decode JSON: {e}"),
                }),
            )
        })?
    };

    let mut accepted = 0;
    let mut rejected = 0;

    for resource_metrics in &request.resource_metrics {
        let resource_attrs = extract_resource_attrs(resource_metrics.resource.as_ref());

        for scope_metrics in &resource_metrics.scope_metrics {
            for metric in &scope_metrics.metrics {
                let converted = otlp_metrics_to_metrics(metric, &resource_attrs);

                for m in converted {
                    if let Err(e) = state.metric_store().insert(m) {
                        tracing::error!(error = %e, "Failed to store metric");
                        rejected += 1;
                    } else {
                        accepted += 1;
                    }
                }
            }
        }
    }

    tracing::debug!(accepted, rejected, "Processed OTLP metrics");

    let response = if rejected > 0 {
        ExportResponse {
            partial_success: Some(PartialSuccess {
                rejected_count: rejected,
                error_message: Some(format!("{rejected} metrics were rejected")),
            }),
        }
    } else {
        ExportResponse {
            partial_success: None,
        }
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for OTLP traces ingestion.
///
/// Accepts `ExportTraceServiceRequest` in protobuf or JSON format.
async fn ingest_traces(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<ExportResponse>), (StatusCode, Json<OtlpError>)> {
    let request = if is_protobuf(&headers) {
        proto::collector::trace::v1::ExportTraceServiceRequest::decode(body).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode protobuf trace request");
            (
                StatusCode::BAD_REQUEST,
                Json(OtlpError {
                    code: 400,
                    message: format!("Failed to decode protobuf: {e}"),
                }),
            )
        })?
    } else {
        serde_json::from_slice(&body).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode JSON trace request");
            (
                StatusCode::BAD_REQUEST,
                Json(OtlpError {
                    code: 400,
                    message: format!("Failed to decode JSON: {e}"),
                }),
            )
        })?
    };

    let mut accepted = 0;
    let mut rejected = 0;

    for resource_spans in &request.resource_spans {
        let resource_attrs = extract_resource_attrs(resource_spans.resource.as_ref());

        for scope_spans in &resource_spans.scope_spans {
            let scope_name = scope_spans
                .scope
                .as_ref()
                .map_or("unknown", |s| s.name.as_str());

            for span in &scope_spans.spans {
                if let Some(internal_span) = otlp_span_to_span(span, &resource_attrs, scope_name) {
                    if let Err(e) = state.trace_store().insert_span(internal_span) {
                        tracing::error!(error = %e, "Failed to store span");
                        rejected += 1;
                    } else {
                        accepted += 1;
                    }
                } else {
                    rejected += 1;
                }
            }
        }
    }

    tracing::debug!(accepted, rejected, "Processed OTLP traces");

    let response = if rejected > 0 {
        ExportResponse {
            partial_success: Some(PartialSuccess {
                rejected_count: rejected,
                error_message: Some(format!("{rejected} spans were rejected")),
            }),
        }
    } else {
        ExportResponse {
            partial_success: None,
        }
    };

    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use shared::storage::LogQuery;
    use tower::ServiceExt;

    const CONTENT_TYPE_JSON: &str = "application/json";

    fn create_test_router() -> Router {
        otlp_routes(AppState::with_in_memory_store())
    }

    fn create_test_router_with_state() -> (Router, AppState) {
        let state = AppState::with_in_memory_store();
        let router = otlp_routes(state.clone());
        (router, state)
    }

    // ========== Log endpoint tests ==========

    #[tokio::test]
    async fn test_ingest_logs_json_empty_request() {
        let app = create_test_router();

        let body = r#"{"resourceLogs": []}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/logs")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ingest_logs_json_valid() {
        let (app, state) = create_test_router_with_state();

        let body = r#"{
            "resourceLogs": [{
                "resource": {
                    "attributes": [{
                        "key": "service.name",
                        "value": {"stringValue": "test-service"}
                    }]
                },
                "scopeLogs": [{
                    "scope": {"name": "test-scope"},
                    "logRecords": [{
                        "timeUnixNano": "1700000000000000000",
                        "severityNumber": 9,
                        "body": {"stringValue": "Test log message"},
                        "attributes": []
                    }]
                }]
            }]
        }"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/logs")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify log was stored
        let result = state.log_store().query(LogQuery::new()).unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].message, "Test log message");
        assert_eq!(result.logs[0].service, "test-service");
    }

    #[tokio::test]
    async fn test_ingest_logs_protobuf_valid() {
        let (app, state) = create_test_router_with_state();

        // Create a valid OTLP logs request
        let request = proto::collector::logs::v1::ExportLogsServiceRequest {
            resource_logs: vec![proto::logs::v1::ResourceLogs {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "proto-test-service".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_logs: vec![proto::logs::v1::ScopeLogs {
                    scope: Some(proto::common::v1::InstrumentationScope {
                        name: "test-scope".to_string(),
                        version: String::new(),
                        attributes: vec![],
                        dropped_attributes_count: 0,
                    }),
                    log_records: vec![proto::logs::v1::LogRecord {
                        time_unix_nano: 1_700_000_000_000_000_000,
                        observed_time_unix_nano: 0,
                        severity_number: 9,
                        severity_text: String::new(),
                        body: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "Protobuf log message".to_string(),
                            )),
                        }),
                        attributes: vec![],
                        dropped_attributes_count: 0,
                        flags: 0,
                        trace_id: vec![],
                        span_id: vec![],
                    }],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        let body = request.encode_to_vec();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/logs")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify log was stored
        let result = state.log_store().query(LogQuery::new()).unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].message, "Protobuf log message");
        assert_eq!(result.logs[0].service, "proto-test-service");
    }

    #[tokio::test]
    async fn test_ingest_logs_invalid_json() {
        let app = create_test_router();

        let body = r"{ invalid json }";

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/logs")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: OtlpError = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.code, 400);
        assert!(error.message.contains("Failed to decode JSON"));
    }

    #[tokio::test]
    async fn test_ingest_logs_invalid_protobuf() {
        let app = create_test_router();

        let body = vec![0xFF, 0xFF, 0xFF]; // Invalid protobuf

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/logs")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_ingest_logs_with_trace_context() {
        let (app, state) = create_test_router_with_state();

        let trace_id = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        let request = proto::collector::logs::v1::ExportLogsServiceRequest {
            resource_logs: vec![proto::logs::v1::ResourceLogs {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "trace-test".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_logs: vec![proto::logs::v1::ScopeLogs {
                    scope: None,
                    log_records: vec![proto::logs::v1::LogRecord {
                        time_unix_nano: 1_700_000_000_000_000_000,
                        observed_time_unix_nano: 0,
                        severity_number: 9,
                        severity_text: String::new(),
                        body: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "Log with trace".to_string(),
                            )),
                        }),
                        attributes: vec![],
                        dropped_attributes_count: 0,
                        flags: 0,
                        trace_id: trace_id.clone(),
                        span_id: span_id.clone(),
                    }],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        let body = request.encode_to_vec();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/logs")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify trace context was stored
        let result = state.log_store().query(LogQuery::new()).unwrap();
        assert_eq!(result.total_count, 1);
        assert!(result.logs[0].trace_id.is_some());
        assert!(result.logs[0].span_id.is_some());
    }

    // ========== Metrics endpoint tests ==========

    #[tokio::test]
    async fn test_ingest_metrics_json_empty_request() {
        let app = create_test_router();

        let body = r#"{"resourceMetrics": []}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/metrics")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ingest_metrics_protobuf_gauge() {
        let (app, state) = create_test_router_with_state();

        let request = proto::collector::metrics::v1::ExportMetricsServiceRequest {
            resource_metrics: vec![proto::metrics::v1::ResourceMetrics {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "metrics-test".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_metrics: vec![proto::metrics::v1::ScopeMetrics {
                    scope: None,
                    metrics: vec![proto::metrics::v1::Metric {
                        name: "test_gauge".to_string(),
                        description: "A test gauge".to_string(),
                        unit: "bytes".to_string(),
                        metadata: vec![],
                        data: Some(proto::metrics::v1::metric::Data::Gauge(
                            proto::metrics::v1::Gauge {
                                data_points: vec![proto::metrics::v1::NumberDataPoint {
                                    attributes: vec![],
                                    start_time_unix_nano: 0,
                                    time_unix_nano: 1_700_000_000_000_000_000,
                                    exemplars: vec![],
                                    flags: 0,
                                    value: Some(
                                        proto::metrics::v1::number_data_point::Value::AsDouble(
                                            42.5,
                                        ),
                                    ),
                                }],
                            },
                        )),
                    }],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        let body = request.encode_to_vec();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/metrics")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify metric was stored
        assert_eq!(state.metric_store().count().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_ingest_metrics_protobuf_counter() {
        let (app, state) = create_test_router_with_state();

        let request = proto::collector::metrics::v1::ExportMetricsServiceRequest {
            resource_metrics: vec![proto::metrics::v1::ResourceMetrics {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "counter-test".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_metrics: vec![proto::metrics::v1::ScopeMetrics {
                    scope: None,
                    metrics: vec![proto::metrics::v1::Metric {
                        name: "request_count".to_string(),
                        description: "Total requests".to_string(),
                        unit: "1".to_string(),
                        metadata: vec![],
                        data: Some(proto::metrics::v1::metric::Data::Sum(
                            proto::metrics::v1::Sum {
                                data_points: vec![proto::metrics::v1::NumberDataPoint {
                                    attributes: vec![],
                                    start_time_unix_nano: 0,
                                    time_unix_nano: 1_700_000_000_000_000_000,
                                    exemplars: vec![],
                                    flags: 0,
                                    value: Some(
                                        proto::metrics::v1::number_data_point::Value::AsInt(100),
                                    ),
                                }],
                                aggregation_temporality: 2, // Cumulative
                                is_monotonic: true,
                            },
                        )),
                    }],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        let body = request.encode_to_vec();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/metrics")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.metric_store().count().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_ingest_metrics_invalid_json() {
        let app = create_test_router();

        let body = r"{ invalid }";

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/metrics")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ========== Traces endpoint tests ==========

    #[tokio::test]
    async fn test_ingest_traces_json_empty_request() {
        let app = create_test_router();

        let body = r#"{"resourceSpans": []}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/traces")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ingest_traces_protobuf_valid() {
        let (app, state) = create_test_router_with_state();

        let trace_id = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        let request = proto::collector::trace::v1::ExportTraceServiceRequest {
            resource_spans: vec![proto::trace::v1::ResourceSpans {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "trace-service".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_spans: vec![proto::trace::v1::ScopeSpans {
                    scope: Some(proto::common::v1::InstrumentationScope {
                        name: "test-tracer".to_string(),
                        version: "1.0.0".to_string(),
                        attributes: vec![],
                        dropped_attributes_count: 0,
                    }),
                    spans: vec![proto::trace::v1::Span {
                        trace_id: trace_id.clone(),
                        span_id: span_id.clone(),
                        trace_state: String::new(),
                        parent_span_id: vec![],
                        name: "test-operation".to_string(),
                        kind: 2, // Server
                        start_time_unix_nano: 1_700_000_000_000_000_000,
                        end_time_unix_nano: 1_700_000_001_000_000_000,
                        attributes: vec![],
                        dropped_attributes_count: 0,
                        events: vec![],
                        dropped_events_count: 0,
                        links: vec![],
                        dropped_links_count: 0,
                        status: Some(proto::trace::v1::Status {
                            message: String::new(),
                            code: 1, // Ok
                        }),
                        flags: 0,
                    }],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        let body = request.encode_to_vec();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/traces")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify span was stored
        assert_eq!(state.trace_store().span_count().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_ingest_traces_with_parent_span() {
        let (app, state) = create_test_router_with_state();

        let trace_id = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let parent_span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let child_span_id = vec![0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18];

        let request = proto::collector::trace::v1::ExportTraceServiceRequest {
            resource_spans: vec![proto::trace::v1::ResourceSpans {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "nested-service".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_spans: vec![proto::trace::v1::ScopeSpans {
                    scope: None,
                    spans: vec![
                        proto::trace::v1::Span {
                            trace_id: trace_id.clone(),
                            span_id: parent_span_id.clone(),
                            trace_state: String::new(),
                            parent_span_id: vec![],
                            name: "parent-operation".to_string(),
                            kind: 2, // Server
                            start_time_unix_nano: 1_700_000_000_000_000_000,
                            end_time_unix_nano: 1_700_000_002_000_000_000,
                            attributes: vec![],
                            dropped_attributes_count: 0,
                            events: vec![],
                            dropped_events_count: 0,
                            links: vec![],
                            dropped_links_count: 0,
                            status: None,
                            flags: 0,
                        },
                        proto::trace::v1::Span {
                            trace_id: trace_id.clone(),
                            span_id: child_span_id,
                            trace_state: String::new(),
                            parent_span_id: parent_span_id.clone(),
                            name: "child-operation".to_string(),
                            kind: 1, // Internal
                            start_time_unix_nano: 1_700_000_000_500_000_000,
                            end_time_unix_nano: 1_700_000_001_500_000_000,
                            attributes: vec![],
                            dropped_attributes_count: 0,
                            events: vec![],
                            dropped_events_count: 0,
                            links: vec![],
                            dropped_links_count: 0,
                            status: None,
                            flags: 0,
                        },
                    ],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        let body = request.encode_to_vec();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/traces")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.trace_store().span_count().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_ingest_traces_invalid_json() {
        let app = create_test_router();

        let body = r"{ invalid }";

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/traces")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_ingest_traces_invalid_span_rejected() {
        let (app, _state) = create_test_router_with_state();

        // Span with empty trace_id should be rejected
        let request = proto::collector::trace::v1::ExportTraceServiceRequest {
            resource_spans: vec![proto::trace::v1::ResourceSpans {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "test".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_spans: vec![proto::trace::v1::ScopeSpans {
                    scope: None,
                    spans: vec![proto::trace::v1::Span {
                        trace_id: vec![], // Invalid - empty
                        span_id: vec![],  // Invalid - empty
                        trace_state: String::new(),
                        parent_span_id: vec![],
                        name: "invalid".to_string(),
                        kind: 0,
                        start_time_unix_nano: 0,
                        end_time_unix_nano: 0,
                        attributes: vec![],
                        dropped_attributes_count: 0,
                        events: vec![],
                        dropped_events_count: 0,
                        links: vec![],
                        dropped_links_count: 0,
                        status: None,
                        flags: 0,
                    }],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };

        let body = request.encode_to_vec();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/traces")
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Response should indicate partial success with rejected spans
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: ExportResponse = serde_json::from_slice(&body).unwrap();
        assert!(result.partial_success.is_some());
        assert_eq!(result.partial_success.unwrap().rejected_count, 1);
    }

    // ========== Content-Type detection tests ==========

    #[tokio::test]
    async fn test_content_type_detection_protobuf() {
        let headers = {
            let mut h = HeaderMap::new();
            h.insert(
                header::CONTENT_TYPE,
                "application/x-protobuf".parse().unwrap(),
            );
            h
        };
        assert!(is_protobuf(&headers));
    }

    #[tokio::test]
    async fn test_content_type_detection_protobuf_with_charset() {
        let headers = {
            let mut h = HeaderMap::new();
            h.insert(
                header::CONTENT_TYPE,
                "application/x-protobuf; charset=utf-8".parse().unwrap(),
            );
            h
        };
        assert!(is_protobuf(&headers));
    }

    #[tokio::test]
    async fn test_content_type_detection_json() {
        let headers = {
            let mut h = HeaderMap::new();
            h.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            h
        };
        assert!(!is_protobuf(&headers));
    }

    #[tokio::test]
    async fn test_content_type_detection_missing() {
        let headers = HeaderMap::new();
        assert!(!is_protobuf(&headers));
    }
}
