//! Conversions between OTLP protobuf types and internal Heimsight types.
//!
//! This module provides functions to convert OpenTelemetry Protocol (OTLP) data
//! into the internal data models used by Heimsight.

use crate::models::{
    HistogramBucket, HistogramData, LogEntry, LogLevel, Metric, MetricType, MetricValue, Span,
    SpanEvent, SpanKind, SpanStatus,
};
use crate::otlp::proto;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::{Duration, UNIX_EPOCH};

/// Converts an OTLP timestamp (nanoseconds since epoch) to a `DateTime<Utc>`.
fn timestamp_to_datetime(nanos: u64) -> DateTime<Utc> {
    let duration = Duration::from_nanos(nanos);
    DateTime::<Utc>::from(UNIX_EPOCH + duration)
}

/// Converts OTLP `AnyValue` to `serde_json::Value`.
fn any_value_to_json(value: &proto::common::v1::AnyValue) -> serde_json::Value {
    use proto::common::v1::any_value::Value;

    match &value.value {
        Some(Value::StringValue(s)) => serde_json::Value::String(s.clone()),
        Some(Value::BoolValue(b)) => serde_json::Value::Bool(*b),
        Some(Value::IntValue(i)) => serde_json::Value::Number((*i).into()),
        Some(Value::DoubleValue(d)) => serde_json::Number::from_f64(*d)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
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

/// Converts OTLP key-value pairs to a `HashMap`.
fn key_values_to_map(
    attributes: &[proto::common::v1::KeyValue],
) -> HashMap<String, serde_json::Value> {
    attributes
        .iter()
        .filter_map(|kv| {
            kv.value
                .as_ref()
                .map(|v| (kv.key.clone(), any_value_to_json(v)))
        })
        .collect()
}

/// Converts OTLP key-value pairs to a string-only `HashMap`.
fn key_values_to_string_map(attributes: &[proto::common::v1::KeyValue]) -> HashMap<String, String> {
    attributes
        .iter()
        .filter_map(|kv| {
            kv.value.as_ref().and_then(|v| {
                if let Some(proto::common::v1::any_value::Value::StringValue(s)) = &v.value {
                    Some((kv.key.clone(), s.clone()))
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Converts OTLP severity number to `LogLevel`.
fn severity_to_log_level(severity: i32) -> LogLevel {
    // OTLP severity numbers: https://opentelemetry.io/docs/specs/otel/logs/data-model/#field-severitynumber
    match severity {
        1..=4 => LogLevel::Trace,
        5..=8 => LogLevel::Debug,
        9..=12 => LogLevel::Info,
        13..=16 => LogLevel::Warn,
        17..=20 => LogLevel::Error,
        21..=24 => LogLevel::Fatal,
        _ => LogLevel::Info, // Default
    }
}

/// Converts OTLP `LogRecord` to `LogEntry`.
///
/// # Arguments
///
/// * `log_record` - The OTLP log record
/// * `resource_attrs` - Resource attributes from the resource
/// * `scope_name` - The instrumentation scope name (service name fallback)
///
/// # Returns
///
/// A `LogEntry` if conversion succeeds, `None` otherwise.
pub fn otlp_log_to_log_entry(
    log_record: &proto::logs::v1::LogRecord,
    resource_attrs: &HashMap<String, serde_json::Value>,
    scope_name: &str,
) -> Option<LogEntry> {
    let timestamp = if log_record.time_unix_nano > 0 {
        timestamp_to_datetime(log_record.time_unix_nano)
    } else {
        Utc::now()
    };

    let level = severity_to_log_level(log_record.severity_number);

    // Extract message from body
    let message = log_record
        .body
        .as_ref()
        .map(|body| match &body.value {
            Some(proto::common::v1::any_value::Value::StringValue(s)) => s.clone(),
            Some(v) => serde_json::to_string(&any_value_to_json(&proto::common::v1::AnyValue {
                value: Some(v.clone()),
            }))
            .unwrap_or_default(),
            None => String::new(),
        })
        .unwrap_or_default();

    if message.is_empty() {
        return None;
    }

    // Extract service name from resource attributes
    let service = resource_attrs
        .get("service.name")
        .and_then(|v| v.as_str())
        .unwrap_or(scope_name)
        .to_string();

    if service.is_empty() {
        return None;
    }

    // Merge attributes
    let mut attributes = key_values_to_map(&log_record.attributes);

    // Add selected resource attributes
    for (key, value) in resource_attrs {
        if key != "service.name" {
            attributes.insert(format!("resource.{key}"), value.clone());
        }
    }

    // Extract trace context
    let trace_id = if !log_record.trace_id.is_empty() {
        Some(hex::encode(&log_record.trace_id))
    } else {
        None
    };

    let span_id = if !log_record.span_id.is_empty() {
        Some(hex::encode(&log_record.span_id))
    } else {
        None
    };

    Some(LogEntry {
        timestamp,
        level,
        message,
        service,
        attributes,
        trace_id,
        span_id,
    })
}

/// Converts OTLP span status to `SpanStatus`.
fn otlp_span_status_to_status(status: Option<&proto::trace::v1::Status>) -> SpanStatus {
    use proto::trace::v1::status::StatusCode;

    match status {
        Some(s) => match StatusCode::try_from(s.code) {
            Ok(StatusCode::Ok) | Ok(StatusCode::Unset) => SpanStatus::Ok,
            Ok(StatusCode::Error) => SpanStatus::Error,
            Err(_) => SpanStatus::Ok,
        },
        None => SpanStatus::Ok,
    }
}

/// Converts OTLP span kind to `SpanKind`.
fn otlp_span_kind_to_kind(kind: i32) -> SpanKind {
    use proto::trace::v1::span::SpanKind as OtlpSpanKind;

    match OtlpSpanKind::try_from(kind) {
        Ok(OtlpSpanKind::Internal) | Ok(OtlpSpanKind::Unspecified) => SpanKind::Internal,
        Ok(OtlpSpanKind::Server) => SpanKind::Server,
        Ok(OtlpSpanKind::Client) => SpanKind::Client,
        Ok(OtlpSpanKind::Producer) => SpanKind::Producer,
        Ok(OtlpSpanKind::Consumer) => SpanKind::Consumer,
        Err(_) => SpanKind::Internal,
    }
}

/// Converts OTLP `Span` to Heimsight `Span`.
pub fn otlp_span_to_span(
    otlp_span: &proto::trace::v1::Span,
    resource_attrs: &HashMap<String, serde_json::Value>,
    scope_name: &str,
) -> Option<Span> {
    if otlp_span.trace_id.is_empty() || otlp_span.span_id.is_empty() {
        return None;
    }

    let trace_id = hex::encode(&otlp_span.trace_id);
    let span_id = hex::encode(&otlp_span.span_id);

    let parent_span_id = if !otlp_span.parent_span_id.is_empty() {
        Some(hex::encode(&otlp_span.parent_span_id))
    } else {
        None
    };

    let name = if otlp_span.name.is_empty() {
        "unknown".to_string()
    } else {
        otlp_span.name.clone()
    };

    let service = resource_attrs
        .get("service.name")
        .and_then(|v| v.as_str())
        .unwrap_or(scope_name)
        .to_string();

    let kind = otlp_span_kind_to_kind(otlp_span.kind);
    let status = otlp_span_status_to_status(otlp_span.status.as_ref());

    let start_time = timestamp_to_datetime(otlp_span.start_time_unix_nano);
    let end_time = timestamp_to_datetime(otlp_span.end_time_unix_nano);

    let mut attributes = key_values_to_map(&otlp_span.attributes);

    // Add selected resource attributes
    for (key, value) in resource_attrs {
        if key != "service.name" {
            attributes.insert(format!("resource.{key}"), value.clone());
        }
    }

    let events = otlp_span
        .events
        .iter()
        .map(|e| SpanEvent {
            name: e.name.clone(),
            timestamp: timestamp_to_datetime(e.time_unix_nano),
            attributes: key_values_to_map(&e.attributes),
        })
        .collect();

    Some(Span {
        trace_id,
        span_id,
        parent_span_id,
        name,
        service,
        kind,
        status,
        start_time,
        end_time,
        attributes,
        events,
    })
}

/// Converts OTLP metric data point to Heimsight `Metric`.
fn otlp_number_data_point_to_metric(
    name: &str,
    metric_type: MetricType,
    data_point: &proto::metrics::v1::NumberDataPoint,
    resource_attrs: &HashMap<String, serde_json::Value>,
    unit: &str,
    description: &str,
) -> Option<Metric> {
    let value = match &data_point.value {
        Some(proto::metrics::v1::number_data_point::Value::AsDouble(d)) => *d,
        Some(proto::metrics::v1::number_data_point::Value::AsInt(i)) => *i as f64,
        None => return None,
    };

    let timestamp = if data_point.time_unix_nano > 0 {
        timestamp_to_datetime(data_point.time_unix_nano)
    } else {
        Utc::now()
    };

    let mut labels = key_values_to_string_map(&data_point.attributes);

    // Add service name from resource
    if let Some(service) = resource_attrs.get("service.name").and_then(|v| v.as_str()) {
        labels.insert("service".to_string(), service.to_string());
    }

    let mut metric =
        Metric::new(name, metric_type, MetricValue::Simple(value)).with_timestamp(timestamp);

    for (k, v) in labels {
        metric = metric.with_label(k, v);
    }

    if !unit.is_empty() {
        metric = metric.with_unit(unit);
    }

    if !description.is_empty() {
        metric = metric.with_description(description);
    }

    Some(metric)
}

/// Converts OTLP histogram data point to Heimsight `Metric`.
fn otlp_histogram_data_point_to_metric(
    name: &str,
    data_point: &proto::metrics::v1::HistogramDataPoint,
    resource_attrs: &HashMap<String, serde_json::Value>,
    unit: &str,
    description: &str,
) -> Option<Metric> {
    let buckets: Vec<HistogramBucket> = data_point
        .explicit_bounds
        .iter()
        .zip(data_point.bucket_counts.iter())
        .map(|(bound, count)| HistogramBucket {
            upper_bound: *bound,
            count: *count,
        })
        .collect();

    let histogram_data = HistogramData {
        buckets,
        sum: data_point.sum.unwrap_or(0.0),
        count: data_point.count,
    };

    let timestamp = if data_point.time_unix_nano > 0 {
        timestamp_to_datetime(data_point.time_unix_nano)
    } else {
        Utc::now()
    };

    let mut labels = key_values_to_string_map(&data_point.attributes);

    if let Some(service) = resource_attrs.get("service.name").and_then(|v| v.as_str()) {
        labels.insert("service".to_string(), service.to_string());
    }

    let mut metric = Metric::histogram(name, histogram_data).with_timestamp(timestamp);

    for (k, v) in labels {
        metric = metric.with_label(k, v);
    }

    if !unit.is_empty() {
        metric = metric.with_unit(unit);
    }

    if !description.is_empty() {
        metric = metric.with_description(description);
    }

    Some(metric)
}

/// Converts OTLP metrics to Heimsight `Metric` vec.
pub fn otlp_metrics_to_metrics(
    otlp_metric: &proto::metrics::v1::Metric,
    resource_attrs: &HashMap<String, serde_json::Value>,
) -> Vec<Metric> {
    let mut metrics = Vec::new();
    let name = &otlp_metric.name;
    let unit = &otlp_metric.unit;
    let description = &otlp_metric.description;

    use proto::metrics::v1::metric::Data;

    match &otlp_metric.data {
        Some(Data::Gauge(gauge)) => {
            for data_point in &gauge.data_points {
                if let Some(metric) = otlp_number_data_point_to_metric(
                    name,
                    MetricType::Gauge,
                    data_point,
                    resource_attrs,
                    unit,
                    description,
                ) {
                    metrics.push(metric);
                }
            }
        }
        Some(Data::Sum(sum)) => {
            // Determine if it's a counter (monotonic) or gauge
            let metric_type = if sum.is_monotonic {
                MetricType::Counter
            } else {
                MetricType::Gauge
            };

            for data_point in &sum.data_points {
                if let Some(metric) = otlp_number_data_point_to_metric(
                    name,
                    metric_type,
                    data_point,
                    resource_attrs,
                    unit,
                    description,
                ) {
                    metrics.push(metric);
                }
            }
        }
        Some(Data::Histogram(histogram)) => {
            for data_point in &histogram.data_points {
                if let Some(metric) = otlp_histogram_data_point_to_metric(
                    name,
                    data_point,
                    resource_attrs,
                    unit,
                    description,
                ) {
                    metrics.push(metric);
                }
            }
        }
        Some(Data::ExponentialHistogram(_)) => {
            // Not yet supported - would need conversion to regular histogram
            tracing::warn!("Exponential histograms not yet supported");
        }
        Some(Data::Summary(_)) => {
            // Not yet supported
            tracing::warn!("Summary metrics not yet supported");
        }
        None => {}
    }

    metrics
}

#[cfg(test)]
mod basic_tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_timestamp_conversion() {
        let nanos = 1_700_000_000_000_000_000u64; // Nov 14, 2023
        let dt = timestamp_to_datetime(nanos);
        assert!(dt.year() == 2023);
    }

    #[test]
    fn test_severity_to_log_level() {
        assert_eq!(severity_to_log_level(1), LogLevel::Trace);
        assert_eq!(severity_to_log_level(9), LogLevel::Info);
        assert_eq!(severity_to_log_level(17), LogLevel::Error);
        assert_eq!(severity_to_log_level(21), LogLevel::Fatal);
    }

    #[test]
    fn test_any_value_string() {
        let value = proto::common::v1::AnyValue {
            value: Some(proto::common::v1::any_value::Value::StringValue(
                "test".to_string(),
            )),
        };
        let json = any_value_to_json(&value);
        assert_eq!(json, serde_json::Value::String("test".to_string()));
    }

    #[test]
    fn test_any_value_int() {
        let value = proto::common::v1::AnyValue {
            value: Some(proto::common::v1::any_value::Value::IntValue(42)),
        };
        let json = any_value_to_json(&value);
        assert_eq!(json, serde_json::json!(42));
    }

    #[test]
    fn test_any_value_bool() {
        let value = proto::common::v1::AnyValue {
            value: Some(proto::common::v1::any_value::Value::BoolValue(true)),
        };
        let json = any_value_to_json(&value);
        assert_eq!(json, serde_json::Value::Bool(true));
    }
}

#[cfg(test)]
#[path = "conversions_test.rs"]
mod conversions_test;
