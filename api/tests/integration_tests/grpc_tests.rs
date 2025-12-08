//! Integration tests for OTLP gRPC endpoints.
//!
//! These tests verify the gRPC server can accept and process
//! OTLP data (logs, metrics, traces) over gRPC.

use super::common::test_app;
use shared::otlp::proto;
use shared::storage::LogQuery;

#[tokio::test]
async fn test_grpc_logs_service_integration() {
    // Create test state
    let (_router, state) = test_app();

    // Create gRPC service
    let service = api::grpc::LogsServiceImpl::new(state.clone());

    // Create test request
    let request = tonic::Request::new(proto::collector::logs::v1::ExportLogsServiceRequest {
        resource_logs: vec![proto::logs::v1::ResourceLogs {
            resource: Some(proto::resource::v1::Resource {
                attributes: vec![proto::common::v1::KeyValue {
                    key: "service.name".to_string(),
                    value: Some(proto::common::v1::AnyValue {
                        value: Some(proto::common::v1::any_value::Value::StringValue(
                            "integration-test-service".to_string(),
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
                    severity_number: 9, // INFO
                    severity_text: String::new(),
                    body: Some(proto::common::v1::AnyValue {
                        value: Some(proto::common::v1::any_value::Value::StringValue(
                            "Integration test log message".to_string(),
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
    });

    // Call the gRPC service
    use proto::collector::logs::v1::logs_service_server::LogsService;
    let response = service.export(request).await.unwrap();
    let inner = response.into_inner();

    // Verify response
    assert!(inner.partial_success.is_none());

    // Verify log was stored
    let result = state.log_store().query(LogQuery::new()).unwrap();
    assert_eq!(result.total_count, 1);
    assert_eq!(result.logs[0].message, "Integration test log message");
    assert_eq!(result.logs[0].service, "integration-test-service");
}

#[tokio::test]
async fn test_grpc_metrics_service_integration() {
    // Create test state
    let (_router, state) = test_app();

    // Create gRPC service
    let service = api::grpc::MetricsServiceImpl::new(state.clone());

    // Create test request with a gauge metric
    let request = tonic::Request::new(proto::collector::metrics::v1::ExportMetricsServiceRequest {
        resource_metrics: vec![proto::metrics::v1::ResourceMetrics {
            resource: Some(proto::resource::v1::Resource {
                attributes: vec![proto::common::v1::KeyValue {
                    key: "service.name".to_string(),
                    value: Some(proto::common::v1::AnyValue {
                        value: Some(proto::common::v1::any_value::Value::StringValue(
                            "integration-test-metrics".to_string(),
                        )),
                    }),
                }],
                dropped_attributes_count: 0,
            }),
            scope_metrics: vec![proto::metrics::v1::ScopeMetrics {
                scope: None,
                metrics: vec![proto::metrics::v1::Metric {
                    name: "test_memory_usage".to_string(),
                    description: "Memory usage in bytes".to_string(),
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
                                    proto::metrics::v1::number_data_point::Value::AsDouble(1024.0),
                                ),
                            }],
                        },
                    )),
                }],
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }],
    });

    // Call the gRPC service
    use proto::collector::metrics::v1::metrics_service_server::MetricsService;
    let response = service.export(request).await.unwrap();
    let inner = response.into_inner();

    // Verify response
    assert!(inner.partial_success.is_none());

    // Verify metric was stored
    assert_eq!(state.metric_store().count().unwrap(), 1);
}

#[tokio::test]
async fn test_grpc_traces_service_integration() {
    // Create test state
    let (_router, state) = test_app();

    // Create gRPC service
    let service = api::grpc::TracesServiceImpl::new(state.clone());

    let trace_id = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10,
    ];
    let span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    // Create test request with a span
    let request = tonic::Request::new(proto::collector::trace::v1::ExportTraceServiceRequest {
        resource_spans: vec![proto::trace::v1::ResourceSpans {
            resource: Some(proto::resource::v1::Resource {
                attributes: vec![proto::common::v1::KeyValue {
                    key: "service.name".to_string(),
                    value: Some(proto::common::v1::AnyValue {
                        value: Some(proto::common::v1::any_value::Value::StringValue(
                            "integration-test-traces".to_string(),
                        )),
                    }),
                }],
                dropped_attributes_count: 0,
            }),
            scope_spans: vec![proto::trace::v1::ScopeSpans {
                scope: None,
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
    });

    // Call the gRPC service
    use proto::collector::trace::v1::trace_service_server::TraceService;
    let response = service.export(request).await.unwrap();
    let inner = response.into_inner();

    // Verify response
    assert!(inner.partial_success.is_none());

    // Verify span was stored
    assert_eq!(state.trace_store().span_count().unwrap(), 1);
}

#[tokio::test]
async fn test_grpc_partial_success_response() {
    // Create test state
    let (_router, state) = test_app();

    // Create gRPC service
    let service = api::grpc::TracesServiceImpl::new(state);

    // Create request with one valid and one invalid span
    let valid_trace_id = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10,
    ];
    let valid_span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    let request = tonic::Request::new(proto::collector::trace::v1::ExportTraceServiceRequest {
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
                spans: vec![
                    // Valid span
                    proto::trace::v1::Span {
                        trace_id: valid_trace_id.clone(),
                        span_id: valid_span_id.clone(),
                        trace_state: String::new(),
                        parent_span_id: vec![],
                        name: "valid-span".to_string(),
                        kind: 2,
                        start_time_unix_nano: 1_700_000_000_000_000_000,
                        end_time_unix_nano: 1_700_000_001_000_000_000,
                        attributes: vec![],
                        dropped_attributes_count: 0,
                        events: vec![],
                        dropped_events_count: 0,
                        links: vec![],
                        dropped_links_count: 0,
                        status: None,
                        flags: 0,
                    },
                    // Invalid span (empty trace_id and span_id)
                    proto::trace::v1::Span {
                        trace_id: vec![],
                        span_id: vec![],
                        trace_state: String::new(),
                        parent_span_id: vec![],
                        name: "invalid-span".to_string(),
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
                    },
                ],
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }],
    });

    // Call the gRPC service
    use proto::collector::trace::v1::trace_service_server::TraceService;
    let response = service.export(request).await.unwrap();
    let inner = response.into_inner();

    // Verify partial success response
    assert!(inner.partial_success.is_some());
    let partial = inner.partial_success.unwrap();
    assert_eq!(partial.rejected_spans, 1);
    assert!(partial.error_message.contains("rejected"));
}
