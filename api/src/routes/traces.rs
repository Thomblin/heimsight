//! Trace ingestion and query endpoints.

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use shared::models::{Span, SpanKind, SpanStatus, Trace};
use shared::storage::TraceQuery;
use std::collections::HashMap;

/// Request for span ingestion.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SpanIngestRequest {
    Single(SpanRequest),
    Batch(Vec<SpanRequest>),
}

/// A single span request.
#[derive(Debug, Deserialize)]
pub struct SpanRequest {
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub name: String,
    pub service: String,
    #[serde(default)]
    pub kind: SpanKind,
    #[serde(default)]
    pub status: SpanStatus,
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
    pub duration_ms: Option<i64>,
}

impl From<SpanRequest> for Span {
    fn from(req: SpanRequest) -> Self {
        let mut span = Span::new(req.trace_id, req.span_id, req.name, req.service)
            .with_kind(req.kind)
            .with_status(req.status);

        if let Some(parent) = req.parent_span_id {
            span = span.with_parent(parent);
        }

        if let Some(duration_ms) = req.duration_ms {
            let end_time = span.start_time + chrono::Duration::milliseconds(duration_ms);
            span = span.with_end_time(end_time);
        }

        for (k, v) in req.attributes {
            span.attributes.insert(k, v);
        }

        span
    }
}

/// Response for span ingestion.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpanIngestResponse {
    pub accepted: usize,
    pub message: String,
}

/// Query parameters for traces.
#[derive(Debug, Deserialize)]
pub struct TraceQueryParams {
    pub service: Option<String>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Response for trace queries.
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceQueryResponse {
    pub traces: Vec<TraceResponse>,
    pub total_count: usize,
}

/// A trace in the response.
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceResponse {
    pub trace_id: String,
    pub span_count: usize,
    pub services: Vec<String>,
    pub duration_ms: Option<i64>,
    pub spans: Vec<Span>,
}

impl From<Trace> for TraceResponse {
    fn from(trace: Trace) -> Self {
        Self {
            trace_id: trace.trace_id.clone(),
            span_count: trace.span_count(),
            services: trace.services().into_iter().map(String::from).collect(),
            duration_ms: trace.duration().map(|d| d.num_milliseconds()),
            spans: trace.spans,
        }
    }
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct TraceError {
    pub error: String,
    pub message: String,
}

/// Creates the traces routes.
pub fn traces_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/traces", post(ingest_spans).get(query_traces))
        .route("/api/v1/traces/{trace_id}", get(get_trace))
        .with_state(state)
}

async fn ingest_spans(
    State(state): State<AppState>,
    Json(request): Json<SpanIngestRequest>,
) -> Result<(StatusCode, Json<SpanIngestResponse>), (StatusCode, Json<TraceError>)> {
    let spans: Vec<SpanRequest> = match request {
        SpanIngestRequest::Single(s) => vec![s],
        SpanIngestRequest::Batch(s) => s,
    };

    if spans.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(TraceError {
                error: "empty_batch".to_string(),
                message: "At least one span is required".to_string(),
            }),
        ));
    }

    let count = spans.len();
    let converted: Vec<Span> = spans.into_iter().map(Into::into).collect();

    state.trace_store().insert_spans(converted).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TraceError {
                error: "storage_error".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(SpanIngestResponse {
            accepted: count,
            message: format!("Ingested {count} span(s)"),
        }),
    ))
}

async fn query_traces(
    State(state): State<AppState>,
    Query(params): Query<TraceQueryParams>,
) -> Result<Json<TraceQueryResponse>, (StatusCode, Json<TraceError>)> {
    let mut query = TraceQuery::new();

    if let Some(service) = params.service {
        query = query.with_service(service);
    }
    if let Some(min) = params.min_duration_ms {
        query = query.with_min_duration_ms(min);
    }
    if let Some(max) = params.max_duration_ms {
        query = query.with_max_duration_ms(max);
    }
    if let Some(limit) = params.limit {
        query = query.with_limit(limit.min(100));
    }
    if let Some(offset) = params.offset {
        query = query.with_offset(offset);
    }

    let result = state.trace_store().query(query).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TraceError {
                error: "storage_error".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(TraceQueryResponse {
        traces: result.traces.into_iter().map(Into::into).collect(),
        total_count: result.total_count,
    }))
}

async fn get_trace(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Json<TraceResponse>, (StatusCode, Json<TraceError>)> {
    let trace = state.trace_store().get_trace(&trace_id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(TraceError {
                error: "not_found".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(trace.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        traces_routes(AppState::with_in_memory_store())
    }

    #[tokio::test]
    async fn test_ingest_single_span() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/traces")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{
                            "trace_id": "trace-123",
                            "span_id": "span-456",
                            "name": "HTTP GET",
                            "service": "api"
                        }"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_ingest_batch_spans() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/traces")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"[
                            {"trace_id": "trace-123", "span_id": "span-1", "name": "root", "service": "api"},
                            {"trace_id": "trace-123", "span_id": "span-2", "name": "db", "service": "db", "parent_span_id": "span-1"}
                        ]"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: SpanIngestResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.accepted, 2);
    }

    #[tokio::test]
    async fn test_query_traces() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/traces")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_trace_by_id() {
        let state = AppState::with_in_memory_store();
        let app = traces_routes(state.clone());

        // Insert a span
        let span = Span::new("trace-123", "span-1", "test", "api");
        state.trace_store().insert_span(span).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/traces/trace-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: TraceResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.trace_id, "trace-123");
    }

    #[tokio::test]
    async fn test_get_trace_not_found() {
        let app = create_test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/traces/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
