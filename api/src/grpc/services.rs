//! gRPC service implementations for OTLP collectors.

use crate::state::AppState;
use shared::otlp::conversions::{
    otlp_log_to_log_entry, otlp_metrics_to_metrics, otlp_span_to_span,
};
use shared::otlp::proto;
use std::collections::HashMap;
use tonic::{Request, Response, Status};

/// Implementation of the OTLP `LogsService` gRPC service.
#[derive(Clone)]
pub struct LogsServiceImpl {
    state: AppState,
}

impl LogsServiceImpl {
    /// Creates a new `LogsServiceImpl` with the given application state.
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
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
                            let json_value = Self::any_value_to_json(v);
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
                let values: Vec<serde_json::Value> =
                    arr.values.iter().map(Self::any_value_to_json).collect();
                serde_json::Value::Array(values)
            }
            Some(Value::KvlistValue(kv)) => {
                let mut map = serde_json::Map::new();
                for pair in &kv.values {
                    if let Some(ref v) = pair.value {
                        map.insert(pair.key.clone(), Self::any_value_to_json(v));
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
}

#[tonic::async_trait]
impl proto::collector::logs::v1::logs_service_server::LogsService for LogsServiceImpl {
    async fn export(
        &self,
        request: Request<proto::collector::logs::v1::ExportLogsServiceRequest>,
    ) -> Result<Response<proto::collector::logs::v1::ExportLogsServiceResponse>, Status> {
        let req = request.into_inner();
        let mut accepted = 0;
        let mut rejected = 0;

        for resource_logs in &req.resource_logs {
            let resource_attrs = Self::extract_resource_attrs(resource_logs.resource.as_ref());

            for scope_logs in &resource_logs.scope_logs {
                let scope_name = scope_logs
                    .scope
                    .as_ref()
                    .map_or("unknown", |s| s.name.as_str());

                for log_record in &scope_logs.log_records {
                    if let Some(log_entry) =
                        otlp_log_to_log_entry(log_record, &resource_attrs, scope_name)
                    {
                        if let Err(e) = self.state.log_store().insert(log_entry) {
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

        tracing::debug!(accepted, rejected, "Processed OTLP gRPC logs");

        let response = proto::collector::logs::v1::ExportLogsServiceResponse {
            partial_success: if rejected > 0 {
                Some(proto::collector::logs::v1::ExportLogsPartialSuccess {
                    rejected_log_records: rejected,
                    error_message: format!("{rejected} log records were rejected"),
                })
            } else {
                None
            },
        };

        Ok(Response::new(response))
    }
}

/// Implementation of the OTLP `MetricsService` gRPC service.
#[derive(Clone)]
pub struct MetricsServiceImpl {
    state: AppState,
}

impl MetricsServiceImpl {
    /// Creates a new `MetricsServiceImpl` with the given application state.
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
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
                            let json_value = Self::any_value_to_json(v);
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
                let values: Vec<serde_json::Value> =
                    arr.values.iter().map(Self::any_value_to_json).collect();
                serde_json::Value::Array(values)
            }
            Some(Value::KvlistValue(kv)) => {
                let mut map = serde_json::Map::new();
                for pair in &kv.values {
                    if let Some(ref v) = pair.value {
                        map.insert(pair.key.clone(), Self::any_value_to_json(v));
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
}

#[tonic::async_trait]
impl proto::collector::metrics::v1::metrics_service_server::MetricsService for MetricsServiceImpl {
    async fn export(
        &self,
        request: Request<proto::collector::metrics::v1::ExportMetricsServiceRequest>,
    ) -> Result<Response<proto::collector::metrics::v1::ExportMetricsServiceResponse>, Status> {
        let req = request.into_inner();
        let mut accepted = 0;
        let mut rejected = 0;

        for resource_metrics in &req.resource_metrics {
            let resource_attrs = Self::extract_resource_attrs(resource_metrics.resource.as_ref());

            for scope_metrics in &resource_metrics.scope_metrics {
                for metric in &scope_metrics.metrics {
                    let converted = otlp_metrics_to_metrics(metric, &resource_attrs);

                    for m in converted {
                        if let Err(e) = self.state.metric_store().insert(m) {
                            tracing::error!(error = %e, "Failed to store metric");
                            rejected += 1;
                        } else {
                            accepted += 1;
                        }
                    }
                }
            }
        }

        tracing::debug!(accepted, rejected, "Processed OTLP gRPC metrics");

        let response = proto::collector::metrics::v1::ExportMetricsServiceResponse {
            partial_success: if rejected > 0 {
                Some(proto::collector::metrics::v1::ExportMetricsPartialSuccess {
                    rejected_data_points: rejected,
                    error_message: format!("{rejected} metrics were rejected"),
                })
            } else {
                None
            },
        };

        Ok(Response::new(response))
    }
}

/// Implementation of the OTLP `TracesService` gRPC service.
#[derive(Clone)]
pub struct TracesServiceImpl {
    state: AppState,
}

impl TracesServiceImpl {
    /// Creates a new `TracesServiceImpl` with the given application state.
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
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
                            let json_value = Self::any_value_to_json(v);
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
                let values: Vec<serde_json::Value> =
                    arr.values.iter().map(Self::any_value_to_json).collect();
                serde_json::Value::Array(values)
            }
            Some(Value::KvlistValue(kv)) => {
                let mut map = serde_json::Map::new();
                for pair in &kv.values {
                    if let Some(ref v) = pair.value {
                        map.insert(pair.key.clone(), Self::any_value_to_json(v));
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
}

#[tonic::async_trait]
impl proto::collector::trace::v1::trace_service_server::TraceService for TracesServiceImpl {
    async fn export(
        &self,
        request: Request<proto::collector::trace::v1::ExportTraceServiceRequest>,
    ) -> Result<Response<proto::collector::trace::v1::ExportTraceServiceResponse>, Status> {
        let req = request.into_inner();
        let mut accepted = 0;
        let mut rejected = 0;

        for resource_spans in &req.resource_spans {
            let resource_attrs = Self::extract_resource_attrs(resource_spans.resource.as_ref());

            for scope_spans in &resource_spans.scope_spans {
                let scope_name = scope_spans
                    .scope
                    .as_ref()
                    .map_or("unknown", |s| s.name.as_str());

                for span in &scope_spans.spans {
                    if let Some(internal_span) =
                        otlp_span_to_span(span, &resource_attrs, scope_name)
                    {
                        if let Err(e) = self.state.trace_store().insert_span(internal_span) {
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

        tracing::debug!(accepted, rejected, "Processed OTLP gRPC traces");

        let response = proto::collector::trace::v1::ExportTraceServiceResponse {
            partial_success: if rejected > 0 {
                Some(proto::collector::trace::v1::ExportTracePartialSuccess {
                    rejected_spans: rejected,
                    error_message: format!("{rejected} spans were rejected"),
                })
            } else {
                None
            },
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::otlp::proto::collector::logs::v1::logs_service_server::LogsService;
    use shared::otlp::proto::collector::metrics::v1::metrics_service_server::MetricsService;
    use shared::otlp::proto::collector::trace::v1::trace_service_server::TraceService;
    use shared::storage::LogQuery;

    /// Helper to create test state for gRPC services.
    fn create_test_state() -> AppState {
        AppState::with_in_memory_store()
    }

    // ========== LogsService tests ==========

    #[tokio::test]
    async fn test_logs_service_empty_request() {
        let state = create_test_state();
        let service = LogsServiceImpl::new(state);

        let request = Request::new(proto::collector::logs::v1::ExportLogsServiceRequest {
            resource_logs: vec![],
        });

        let response = service.export(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.partial_success.is_none());
    }

    #[tokio::test]
    async fn test_logs_service_valid_log() {
        let state = create_test_state();
        let service = LogsServiceImpl::new(state.clone());

        let request = Request::new(proto::collector::logs::v1::ExportLogsServiceRequest {
            resource_logs: vec![proto::logs::v1::ResourceLogs {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "grpc-test-service".to_string(),
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
                                "gRPC log message".to_string(),
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

        let response = service.export(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.partial_success.is_none());

        // Verify log was stored
        let result = state.log_store().query(LogQuery::new()).unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].message, "gRPC log message");
        assert_eq!(result.logs[0].service, "grpc-test-service");
    }

    #[tokio::test]
    async fn test_logs_service_with_trace_context() {
        let state = create_test_state();
        let service = LogsServiceImpl::new(state.clone());

        let trace_id = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        let request = Request::new(proto::collector::logs::v1::ExportLogsServiceRequest {
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
        });

        let response = service.export(request).await.unwrap();
        assert!(response.into_inner().partial_success.is_none());

        // Verify trace context was stored
        let result = state.log_store().query(LogQuery::new()).unwrap();
        assert_eq!(result.total_count, 1);
        assert!(result.logs[0].trace_id.is_some());
        assert!(result.logs[0].span_id.is_some());
    }

    // ========== MetricsService tests ==========

    #[tokio::test]
    async fn test_metrics_service_empty_request() {
        let state = create_test_state();
        let service = MetricsServiceImpl::new(state);

        let request = Request::new(proto::collector::metrics::v1::ExportMetricsServiceRequest {
            resource_metrics: vec![],
        });

        let response = service.export(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.partial_success.is_none());
    }

    #[tokio::test]
    async fn test_metrics_service_gauge() {
        let state = create_test_state();
        let service = MetricsServiceImpl::new(state.clone());

        let request = Request::new(proto::collector::metrics::v1::ExportMetricsServiceRequest {
            resource_metrics: vec![proto::metrics::v1::ResourceMetrics {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "grpc-metrics-test".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_metrics: vec![proto::metrics::v1::ScopeMetrics {
                    scope: None,
                    metrics: vec![proto::metrics::v1::Metric {
                        name: "grpc_test_gauge".to_string(),
                        description: "A test gauge via gRPC".to_string(),
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
                                            99.9,
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
        });

        let response = service.export(request).await.unwrap();
        assert!(response.into_inner().partial_success.is_none());

        // Verify metric was stored
        assert_eq!(state.metric_store().count().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_metrics_service_counter() {
        let state = create_test_state();
        let service = MetricsServiceImpl::new(state.clone());

        let request = Request::new(proto::collector::metrics::v1::ExportMetricsServiceRequest {
            resource_metrics: vec![proto::metrics::v1::ResourceMetrics {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "grpc-counter-test".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_metrics: vec![proto::metrics::v1::ScopeMetrics {
                    scope: None,
                    metrics: vec![proto::metrics::v1::Metric {
                        name: "grpc_request_count".to_string(),
                        description: "Total requests via gRPC".to_string(),
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
                                        proto::metrics::v1::number_data_point::Value::AsInt(200),
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
        });

        let response = service.export(request).await.unwrap();
        assert!(response.into_inner().partial_success.is_none());

        assert_eq!(state.metric_store().count().unwrap(), 1);
    }

    // ========== TracesService tests ==========

    #[tokio::test]
    async fn test_traces_service_empty_request() {
        let state = create_test_state();
        let service = TracesServiceImpl::new(state);

        let request = Request::new(proto::collector::trace::v1::ExportTraceServiceRequest {
            resource_spans: vec![],
        });

        let response = service.export(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.partial_success.is_none());
    }

    #[tokio::test]
    async fn test_traces_service_valid_span() {
        let state = create_test_state();
        let service = TracesServiceImpl::new(state.clone());

        let trace_id = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        let request = Request::new(proto::collector::trace::v1::ExportTraceServiceRequest {
            resource_spans: vec![proto::trace::v1::ResourceSpans {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "grpc-trace-service".to_string(),
                            )),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_spans: vec![proto::trace::v1::ScopeSpans {
                    scope: Some(proto::common::v1::InstrumentationScope {
                        name: "grpc-test-tracer".to_string(),
                        version: "1.0.0".to_string(),
                        attributes: vec![],
                        dropped_attributes_count: 0,
                    }),
                    spans: vec![proto::trace::v1::Span {
                        trace_id: trace_id.clone(),
                        span_id: span_id.clone(),
                        trace_state: String::new(),
                        parent_span_id: vec![],
                        name: "grpc-test-operation".to_string(),
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

        let response = service.export(request).await.unwrap();
        assert!(response.into_inner().partial_success.is_none());

        // Verify span was stored
        assert_eq!(state.trace_store().span_count().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_traces_service_with_parent_span() {
        let state = create_test_state();
        let service = TracesServiceImpl::new(state.clone());

        let trace_id = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ];
        let parent_span_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let child_span_id = vec![0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18];

        let request = Request::new(proto::collector::trace::v1::ExportTraceServiceRequest {
            resource_spans: vec![proto::trace::v1::ResourceSpans {
                resource: Some(proto::resource::v1::Resource {
                    attributes: vec![proto::common::v1::KeyValue {
                        key: "service.name".to_string(),
                        value: Some(proto::common::v1::AnyValue {
                            value: Some(proto::common::v1::any_value::Value::StringValue(
                                "grpc-nested-service".to_string(),
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
                            name: "grpc-parent-operation".to_string(),
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
                            name: "grpc-child-operation".to_string(),
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
        });

        let response = service.export(request).await.unwrap();
        assert!(response.into_inner().partial_success.is_none());

        assert_eq!(state.trace_store().span_count().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_traces_service_invalid_span_rejected() {
        let state = create_test_state();
        let service = TracesServiceImpl::new(state);

        // Span with empty trace_id should be rejected
        let request = Request::new(proto::collector::trace::v1::ExportTraceServiceRequest {
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
        });

        let response = service.export(request).await.unwrap();
        let inner = response.into_inner();

        // Response should indicate partial success with rejected spans
        assert!(inner.partial_success.is_some());
        let partial = inner.partial_success.unwrap();
        assert_eq!(partial.rejected_spans, 1);
    }
}
