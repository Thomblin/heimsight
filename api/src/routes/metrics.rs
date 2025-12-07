//! Metrics ingestion and query endpoints.

use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use shared::models::{Metric, MetricType, MetricValue};
use shared::storage::{AggregationFunction, MetricQuery};
use std::collections::HashMap;

/// Request for metric ingestion.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MetricIngestRequest {
    Single(MetricRequest),
    Batch(Vec<MetricRequest>),
}

/// A single metric request.
#[derive(Debug, Deserialize)]
pub struct MetricRequest {
    pub name: String,
    #[serde(default)]
    pub metric_type: MetricType,
    pub value: f64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl From<MetricRequest> for Metric {
    fn from(req: MetricRequest) -> Self {
        let mut metric = Metric::new(req.name, req.metric_type, MetricValue::Simple(req.value));
        for (k, v) in req.labels {
            metric = metric.with_label(k, v);
        }
        if let Some(desc) = req.description {
            metric = metric.with_description(desc);
        }
        if let Some(unit) = req.unit {
            metric = metric.with_unit(unit);
        }
        metric
    }
}

/// Response for metric ingestion.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricIngestResponse {
    pub accepted: usize,
    pub message: String,
}

/// Query parameters for metrics.
#[derive(Debug, Deserialize)]
pub struct MetricQueryParams {
    pub name: Option<String>,
    pub metric_type: Option<MetricType>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub aggregate: Option<String>,
}

/// Response for metric queries.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricQueryResponse {
    pub metrics: Vec<Metric>,
    pub total_count: usize,
    pub aggregation: Option<AggregationResponse>,
}

/// Aggregation result.
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregationResponse {
    pub function: String,
    pub value: f64,
    pub count: usize,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct MetricError {
    pub error: String,
    pub message: String,
}

/// Creates the metrics routes.
pub fn metrics_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/metrics", post(ingest_metrics).get(query_metrics))
        .with_state(state)
}

async fn ingest_metrics(
    State(state): State<AppState>,
    Json(request): Json<MetricIngestRequest>,
) -> Result<(StatusCode, Json<MetricIngestResponse>), (StatusCode, Json<MetricError>)> {
    let metrics: Vec<MetricRequest> = match request {
        MetricIngestRequest::Single(m) => vec![m],
        MetricIngestRequest::Batch(m) => m,
    };

    if metrics.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(MetricError {
                error: "empty_batch".to_string(),
                message: "At least one metric is required".to_string(),
            }),
        ));
    }

    let count = metrics.len();
    let converted: Vec<Metric> = metrics.into_iter().map(Into::into).collect();

    state.metric_store().insert_batch(converted).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MetricError {
                error: "storage_error".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(MetricIngestResponse {
            accepted: count,
            message: format!("Ingested {count} metric(s)"),
        }),
    ))
}

async fn query_metrics(
    State(state): State<AppState>,
    Query(params): Query<MetricQueryParams>,
) -> Result<Json<MetricQueryResponse>, (StatusCode, Json<MetricError>)> {
    let mut query = MetricQuery::new();

    if let Some(name) = params.name {
        query = query.with_name(name);
    }
    if let Some(metric_type) = params.metric_type {
        query = query.with_type(metric_type);
    }
    if let Some(limit) = params.limit {
        query = query.with_limit(limit.min(1000));
    }
    if let Some(offset) = params.offset {
        query = query.with_offset(offset);
    }

    // Handle aggregation
    let aggregation = if let Some(ref agg) = params.aggregate {
        let func = match agg.to_lowercase().as_str() {
            "sum" => AggregationFunction::Sum,
            "avg" => AggregationFunction::Avg,
            "min" => AggregationFunction::Min,
            "max" => AggregationFunction::Max,
            "count" => AggregationFunction::Count,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(MetricError {
                        error: "invalid_aggregation".to_string(),
                        message: format!("Unknown aggregation: {agg}"),
                    }),
                ))
            }
        };

        let result = state
            .metric_store()
            .aggregate(query.clone(), func)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(MetricError {
                        error: "storage_error".to_string(),
                        message: e.to_string(),
                    }),
                )
            })?;

        Some(AggregationResponse {
            function: agg.clone(),
            value: result.value,
            count: result.count,
        })
    } else {
        None
    };

    let result = state.metric_store().query(query).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MetricError {
                error: "storage_error".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(MetricQueryResponse {
        metrics: result.metrics,
        total_count: result.total_count,
        aggregation,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        metrics_routes(AppState::with_in_memory_store())
    }

    #[tokio::test]
    async fn test_ingest_single_metric() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/metrics")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"name": "cpu_usage", "value": 75.5, "metric_type": "gauge"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_ingest_batch_metrics() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/metrics")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"[
                            {"name": "cpu_usage", "value": 75.5},
                            {"name": "memory_usage", "value": 1024}
                        ]"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: MetricIngestResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.accepted, 2);
    }

    #[tokio::test]
    async fn test_query_metrics() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_query_with_aggregation() {
        let (app, state) = {
            let state = AppState::with_in_memory_store();
            (metrics_routes(state.clone()), state)
        };

        // Insert some metrics
        state
            .metric_store()
            .insert(Metric::gauge("test", 10.0))
            .unwrap();
        state
            .metric_store()
            .insert(Metric::gauge("test", 20.0))
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/metrics?aggregate=avg")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: MetricQueryResponse = serde_json::from_slice(&body).unwrap();

        assert!(result.aggregation.is_some());
        assert_eq!(result.aggregation.unwrap().value, 15.0);
    }
}
