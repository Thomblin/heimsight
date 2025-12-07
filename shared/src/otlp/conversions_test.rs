//! Tests for OTLP conversions.

#[cfg(test)]
mod tests {
    use crate::models::{LogLevel, MetricType, SpanKind, SpanStatus};
    use crate::otlp::conversions::*;
    use crate::otlp::proto;
    use std::collections::HashMap;

    #[test]
    fn test_otlp_log_to_log_entry() {
        let mut resource_attrs = HashMap::new();
        resource_attrs.insert(
            "service.name".to_string(),
            serde_json::Value::String("test-service".to_string()),
        );

        let log_record = proto::logs::v1::LogRecord {
            time_unix_nano: 1_700_000_000_000_000_000,
            severity_number: 17, // Error level
            severity_text: "ERROR".to_string(),
            body: Some(proto::common::v1::AnyValue {
                value: Some(proto::common::v1::any_value::Value::StringValue(
                    "Test error message".to_string(),
                )),
            }),
            attributes: vec![proto::common::v1::KeyValue {
                key: "user_id".to_string(),
                value: Some(proto::common::v1::AnyValue {
                    value: Some(proto::common::v1::any_value::Value::StringValue(
                        "12345".to_string(),
                    )),
                }),
            }],
            trace_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            span_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
            ..Default::default()
        };

        let log_entry = otlp_log_to_log_entry(&log_record, &resource_attrs, "fallback-service");

        assert!(log_entry.is_some());
        let log = log_entry.unwrap();
        assert_eq!(log.level, LogLevel::Error);
        assert_eq!(log.message, "Test error message");
        assert_eq!(log.service, "test-service");
        assert!(log.trace_id.is_some());
        assert!(log.span_id.is_some());
        assert!(log.attributes.contains_key("user_id"));
    }

    #[test]
    fn test_otlp_log_empty_message() {
        let resource_attrs = HashMap::new();

        let log_record = proto::logs::v1::LogRecord {
            time_unix_nano: 1_700_000_000_000_000_000,
            severity_number: 9,
            body: None,
            ..Default::default()
        };

        let log_entry = otlp_log_to_log_entry(&log_record, &resource_attrs, "test-service");
        assert!(log_entry.is_none());
    }

    #[test]
    fn test_otlp_span_to_span() {
        let mut resource_attrs = HashMap::new();
        resource_attrs.insert(
            "service.name".to_string(),
            serde_json::Value::String("api-service".to_string()),
        );

        let otlp_span = proto::trace::v1::Span {
            trace_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            span_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
            parent_span_id: vec![],
            name: "GET /api/users".to_string(),
            kind: proto::trace::v1::span::SpanKind::Server as i32,
            start_time_unix_nano: 1_700_000_000_000_000_000,
            end_time_unix_nano: 1_700_000_000_100_000_000,
            attributes: vec![proto::common::v1::KeyValue {
                key: "http.method".to_string(),
                value: Some(proto::common::v1::AnyValue {
                    value: Some(proto::common::v1::any_value::Value::StringValue(
                        "GET".to_string(),
                    )),
                }),
            }],
            status: Some(proto::trace::v1::Status {
                message: String::new(),
                code: proto::trace::v1::status::StatusCode::Ok as i32,
            }),
            ..Default::default()
        };

        let span = otlp_span_to_span(&otlp_span, &resource_attrs, "fallback-service");

        assert!(span.is_some());
        let s = span.unwrap();
        assert_eq!(s.name, "GET /api/users");
        assert_eq!(s.service, "api-service");
        assert_eq!(s.kind, SpanKind::Server);
        assert_eq!(s.status, SpanStatus::Ok);
        assert!(s.is_root());
        assert!(s.attributes.contains_key("http.method"));
    }

    #[test]
    fn test_otlp_span_with_parent() {
        let resource_attrs = HashMap::new();

        let otlp_span = proto::trace::v1::Span {
            trace_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            span_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
            parent_span_id: vec![9, 10, 11, 12, 13, 14, 15, 16],
            name: "DB query".to_string(),
            kind: proto::trace::v1::span::SpanKind::Client as i32,
            start_time_unix_nano: 1_700_000_000_000_000_000,
            end_time_unix_nano: 1_700_000_000_050_000_000,
            status: Some(proto::trace::v1::Status {
                message: String::new(),
                code: proto::trace::v1::status::StatusCode::Error as i32,
            }),
            ..Default::default()
        };

        let span = otlp_span_to_span(&otlp_span, &resource_attrs, "db-service");

        assert!(span.is_some());
        let s = span.unwrap();
        assert!(!s.is_root());
        assert!(s.parent_span_id.is_some());
        assert_eq!(s.kind, SpanKind::Client);
        assert_eq!(s.status, SpanStatus::Error);
    }

    #[test]
    fn test_otlp_metrics_gauge() {
        let mut resource_attrs = HashMap::new();
        resource_attrs.insert(
            "service.name".to_string(),
            serde_json::Value::String("metrics-service".to_string()),
        );

        let otlp_metric = proto::metrics::v1::Metric {
            name: "cpu_usage".to_string(),
            description: "CPU usage percentage".to_string(),
            unit: "percent".to_string(),
            metadata: vec![],
            data: Some(proto::metrics::v1::metric::Data::Gauge(
                proto::metrics::v1::Gauge {
                    data_points: vec![proto::metrics::v1::NumberDataPoint {
                        attributes: vec![proto::common::v1::KeyValue {
                            key: "host".to_string(),
                            value: Some(proto::common::v1::AnyValue {
                                value: Some(proto::common::v1::any_value::Value::StringValue(
                                    "server1".to_string(),
                                )),
                            }),
                        }],
                        time_unix_nano: 1_700_000_000_000_000_000,
                        value: Some(proto::metrics::v1::number_data_point::Value::AsDouble(75.5)),
                        ..Default::default()
                    }],
                },
            )),
        };

        let metrics = otlp_metrics_to_metrics(&otlp_metric, &resource_attrs);

        assert_eq!(metrics.len(), 1);
        let metric = &metrics[0];
        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.metric_type, MetricType::Gauge);
        assert_eq!(metric.simple_value(), Some(75.5));
        assert_eq!(metric.labels.get("host"), Some(&"server1".to_string()));
        assert_eq!(
            metric.labels.get("service"),
            Some(&"metrics-service".to_string())
        );
        assert_eq!(metric.unit, Some("percent".to_string()));
    }

    #[test]
    fn test_otlp_metrics_counter() {
        let resource_attrs = HashMap::new();

        let otlp_metric = proto::metrics::v1::Metric {
            name: "requests_total".to_string(),
            description: "Total requests".to_string(),
            unit: "1".to_string(),
            metadata: vec![],
            data: Some(proto::metrics::v1::metric::Data::Sum(
                proto::metrics::v1::Sum {
                    data_points: vec![proto::metrics::v1::NumberDataPoint {
                        attributes: vec![],
                        time_unix_nano: 1_700_000_000_000_000_000,
                        value: Some(proto::metrics::v1::number_data_point::Value::AsInt(1234)),
                        ..Default::default()
                    }],
                    aggregation_temporality: proto::metrics::v1::AggregationTemporality::Cumulative
                        as i32,
                    is_monotonic: true,
                },
            )),
        };

        let metrics = otlp_metrics_to_metrics(&otlp_metric, &resource_attrs);

        assert_eq!(metrics.len(), 1);
        let metric = &metrics[0];
        assert_eq!(metric.name, "requests_total");
        assert_eq!(metric.metric_type, MetricType::Counter);
        assert_eq!(metric.simple_value(), Some(1234.0));
    }

    #[test]
    fn test_otlp_metrics_histogram() {
        let resource_attrs = HashMap::new();

        let otlp_metric = proto::metrics::v1::Metric {
            name: "request_duration_seconds".to_string(),
            description: "Request duration".to_string(),
            unit: "s".to_string(),
            metadata: vec![],
            data: Some(proto::metrics::v1::metric::Data::Histogram(
                proto::metrics::v1::Histogram {
                    data_points: vec![proto::metrics::v1::HistogramDataPoint {
                        attributes: vec![],
                        time_unix_nano: 1_700_000_000_000_000_000,
                        count: 100,
                        sum: Some(50.0),
                        bucket_counts: vec![10, 30, 50, 10],
                        explicit_bounds: vec![0.1, 0.5, 1.0, 5.0],
                        ..Default::default()
                    }],
                    aggregation_temporality: proto::metrics::v1::AggregationTemporality::Cumulative
                        as i32,
                },
            )),
        };

        let metrics = otlp_metrics_to_metrics(&otlp_metric, &resource_attrs);

        assert_eq!(metrics.len(), 1);
        let metric = &metrics[0];
        assert_eq!(metric.name, "request_duration_seconds");
        assert_eq!(metric.metric_type, MetricType::Histogram);

        let histogram = metric.value.as_histogram();
        assert!(histogram.is_some());
        let hist = histogram.unwrap();
        assert_eq!(hist.count, 100);
        assert!((hist.sum - 50.0).abs() < f64::EPSILON);
        assert_eq!(hist.buckets.len(), 4);
    }
}
