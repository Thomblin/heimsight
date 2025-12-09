//! Trace storage trait and implementations.
//!
//! Provides the `TraceStore` trait for abstracting trace storage operations
//! and an `InMemoryTraceStore` implementation for development and testing.

use crate::models::{Span, SpanStatus, Trace};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors that can occur during trace store operations.
#[derive(Debug, Error)]
pub enum TraceStoreError {
    /// Failed to acquire lock on the store.
    #[error("Failed to acquire lock on trace store")]
    LockError,

    /// Trace not found.
    #[error("Trace not found: {0}")]
    NotFound(String),

    /// Generic storage error.
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Query parameters for retrieving traces.
#[derive(Debug, Clone, Default)]
pub struct TraceQuery {
    /// Filter by service name.
    pub service: Option<String>,

    /// Filter traces starting from this time (inclusive).
    pub start_time: Option<DateTime<Utc>>,

    /// Filter traces up to this time (exclusive).
    pub end_time: Option<DateTime<Utc>>,

    /// Minimum duration in milliseconds.
    pub min_duration_ms: Option<i64>,

    /// Maximum duration in milliseconds.
    pub max_duration_ms: Option<i64>,

    /// Filter by span status.
    pub status: Option<SpanStatus>,

    /// Maximum number of traces to return.
    pub limit: Option<usize>,

    /// Number of traces to skip (for pagination).
    pub offset: Option<usize>,
}

impl TraceQuery {
    /// Creates a new empty query (returns all traces).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the service filter.
    #[must_use]
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Sets the start time filter.
    #[must_use]
    pub fn with_start_time(mut self, start: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self
    }

    /// Sets the end time filter.
    #[must_use]
    pub fn with_end_time(mut self, end: DateTime<Utc>) -> Self {
        self.end_time = Some(end);
        self
    }

    /// Sets the minimum duration filter.
    #[must_use]
    pub fn with_min_duration_ms(mut self, ms: i64) -> Self {
        self.min_duration_ms = Some(ms);
        self
    }

    /// Sets the maximum duration filter.
    #[must_use]
    pub fn with_max_duration_ms(mut self, ms: i64) -> Self {
        self.max_duration_ms = Some(ms);
        self
    }

    /// Sets the status filter.
    #[must_use]
    pub fn with_status(mut self, status: SpanStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets the maximum number of results.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the offset for pagination.
    #[must_use]
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Result of a trace query operation.
#[derive(Debug, Clone)]
pub struct TraceQueryResult {
    /// The traces matching the query.
    pub traces: Vec<Trace>,

    /// Total count of matching traces (before limit/offset applied).
    pub total_count: usize,
}

/// Trait for trace storage implementations.
///
/// This trait defines the interface for storing and querying traces.
/// Implementations must be thread-safe (Send + Sync).
pub trait TraceStore: Send + Sync {
    /// Inserts a single span into the store.
    ///
    /// Spans are grouped by `trace_id` to form traces.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert_span(&self, span: Span) -> Result<(), TraceStoreError>;

    /// Inserts multiple spans into the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert_spans(&self, spans: Vec<Span>) -> Result<(), TraceStoreError>;

    /// Gets a trace by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the trace is not found or the operation fails.
    fn get_trace(&self, trace_id: &str) -> Result<Trace, TraceStoreError>;

    /// Queries traces based on the provided parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the query operation fails.
    fn query(&self, query: TraceQuery) -> Result<TraceQueryResult, TraceStoreError>;

    /// Returns the total number of spans in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the count operation fails.
    fn span_count(&self) -> Result<usize, TraceStoreError>;

    /// Returns the total number of unique traces in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the count operation fails.
    fn trace_count(&self) -> Result<usize, TraceStoreError>;

    /// Clears all traces from the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the clear operation fails.
    fn clear(&self) -> Result<(), TraceStoreError>;
}

/// In-memory trace store implementation.
#[derive(Debug, Default)]
pub struct InMemoryTraceStore {
    /// Spans grouped by `trace_id`.
    spans: Arc<RwLock<HashMap<String, Vec<Span>>>>,
}

impl InMemoryTraceStore {
    /// Creates a new empty in-memory trace store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new in-memory trace store wrapped in an Arc.
    #[must_use]
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl TraceStore for InMemoryTraceStore {
    fn insert_span(&self, span: Span) -> Result<(), TraceStoreError> {
        let mut spans = self.spans.write().map_err(|_| TraceStoreError::LockError)?;
        spans.entry(span.trace_id.clone()).or_default().push(span);
        Ok(())
    }

    fn insert_spans(&self, new_spans: Vec<Span>) -> Result<(), TraceStoreError> {
        let mut spans = self.spans.write().map_err(|_| TraceStoreError::LockError)?;
        for span in new_spans {
            spans.entry(span.trace_id.clone()).or_default().push(span);
        }
        Ok(())
    }

    fn get_trace(&self, trace_id: &str) -> Result<Trace, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;

        spans
            .get(trace_id)
            .and_then(|s| Trace::from_spans(s.clone()))
            .ok_or_else(|| TraceStoreError::NotFound(trace_id.to_string()))
    }

    fn query(&self, query: TraceQuery) -> Result<TraceQueryResult, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;

        let mut traces: Vec<Trace> = spans
            .values()
            .filter_map(|s| Trace::from_spans(s.clone()))
            .filter(|trace| {
                // Service filter
                if let Some(ref service) = query.service {
                    if !trace.spans.iter().any(|s| &s.service == service) {
                        return false;
                    }
                }

                // Time range filter (based on any span in the trace)
                if let Some(start) = query.start_time {
                    if !trace.spans.iter().any(|s| s.start_time >= start) {
                        return false;
                    }
                }
                if let Some(end) = query.end_time {
                    if !trace.spans.iter().any(|s| s.start_time < end) {
                        return false;
                    }
                }

                // Duration filter
                if let Some(duration) = trace.duration() {
                    let duration_ms = duration.num_milliseconds();

                    if let Some(min) = query.min_duration_ms {
                        if duration_ms < min {
                            return false;
                        }
                    }
                    if let Some(max) = query.max_duration_ms {
                        if duration_ms > max {
                            return false;
                        }
                    }
                }

                // Status filter
                if let Some(ref status) = query.status {
                    if !trace.spans.iter().any(|s| &s.status == status) {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort by start time (most recent first)
        traces.sort_by(|a, b| {
            let a_start = a.spans.iter().map(|s| s.start_time).min();
            let b_start = b.spans.iter().map(|s| s.start_time).min();
            b_start.cmp(&a_start)
        });

        let total_count = traces.len();

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let result: Vec<Trace> = traces
            .into_iter()
            .skip(offset)
            .take(query.limit.unwrap_or(usize::MAX))
            .collect();

        Ok(TraceQueryResult {
            traces: result,
            total_count,
        })
    }

    fn span_count(&self) -> Result<usize, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;
        Ok(spans.values().map(std::vec::Vec::len).sum())
    }

    fn trace_count(&self) -> Result<usize, TraceStoreError> {
        let spans = self.spans.read().map_err(|_| TraceStoreError::LockError)?;
        Ok(spans.len())
    }

    fn clear(&self) -> Result<(), TraceStoreError> {
        let mut spans = self.spans.write().map_err(|_| TraceStoreError::LockError)?;
        spans.clear();
        Ok(())
    }
}

/// `ClickHouse`-backed trace store implementation.
///
/// This implementation stores spans in `ClickHouse` for production use.
/// It provides persistent storage and efficient distributed trace queries.
#[derive(Clone)]
pub struct ClickHouseTraceStore {
    client: Arc<clickhouse::Client>,
}

impl ClickHouseTraceStore {
    /// Creates a new `ClickHouse` trace store with the given client.
    #[must_use]
    pub fn new(client: Arc<clickhouse::Client>) -> Self {
        Self { client }
    }

    /// Creates a new `ClickHouse` trace store wrapped in an Arc.
    #[must_use]
    pub fn new_shared(client: Arc<clickhouse::Client>) -> Arc<Self> {
        Arc::new(Self::new(client))
    }

    /// Helper to execute async operations synchronously.
    fn block_on<F, T>(future: F) -> Result<T, TraceStoreError>
    where
        F: std::future::Future<Output = Result<T, clickhouse::error::Error>>,
    {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(future)
                .map_err(|e| TraceStoreError::StorageError(e.to_string()))
        })
    }
}

impl TraceStore for ClickHouseTraceStore {
    fn insert_span(&self, span: Span) -> Result<(), TraceStoreError> {
        self.insert_spans(vec![span])
    }

    fn insert_spans(&self, spans: Vec<Span>) -> Result<(), TraceStoreError> {
        if spans.is_empty() {
            return Ok(());
        }

        let client = Arc::clone(&self.client);
        Self::block_on(async move {
            #[derive(clickhouse::Row, serde::Serialize)]
            struct SpanRow {
                trace_id: String,
                span_id: String,
                parent_span_id: String,
                start_time: i64,
                end_time: i64,
                duration_ns: u64,
                name: String,
                span_kind: String,
                service: String,
                operation: String,
                status_code: String,
                status_message: String,
                attributes: HashMap<String, String>,
                resource_attributes: HashMap<String, String>,
                events: Vec<(i64, String, HashMap<String, String>)>,
                links: Vec<(String, String, HashMap<String, String>)>,
            }

            let mut inserter = client.insert::<SpanRow>("spans").await?;

            for span in spans {
                // Calculate duration in nanoseconds
                let duration_ns = (span.end_time - span.start_time)
                    .num_nanoseconds()
                    .unwrap_or(0);

                // Convert attributes to Map
                let attributes: HashMap<String, String> = span
                    .attributes
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect();

                // Convert events to array of tuples
                let events: Vec<(i64, String, HashMap<String, String>)> = span
                    .events
                    .iter()
                    .map(|e| {
                        let attrs: HashMap<String, String> = e
                            .attributes
                            .iter()
                            .map(|(k, v)| (k.clone(), v.to_string()))
                            .collect();
                        (
                            e.timestamp.timestamp_nanos_opt().unwrap_or(0),
                            e.name.clone(),
                            attrs,
                        )
                    })
                    .collect();

                let row = SpanRow {
                    trace_id: span.trace_id,
                    span_id: span.span_id,
                    parent_span_id: span.parent_span_id.unwrap_or_default(),
                    start_time: span.start_time.timestamp_nanos_opt().unwrap_or(0),
                    end_time: span.end_time.timestamp_nanos_opt().unwrap_or(0),
                    duration_ns: u64::try_from(duration_ns).unwrap_or(0),
                    name: span.name,
                    span_kind: span.kind.to_string(),
                    service: span.service.clone(),
                    operation: span.service,
                    status_code: span.status.to_string(),
                    status_message: String::new(),
                    attributes,
                    resource_attributes: HashMap::new(),
                    events,
                    links: Vec::new(),
                };

                inserter.write(&row).await?;
            }

            inserter.end().await?;
            Ok(())
        })
    }

    #[allow(clippy::too_many_lines)]
    fn get_trace(&self, trace_id: &str) -> Result<Trace, TraceStoreError> {
        // Define row structure for deserialization
        #[derive(clickhouse::Row, serde::Deserialize)]
        #[allow(dead_code)]
        struct SpanRow {
            trace_id: String,
            span_id: String,
            parent_span_id: String,
            start_time: i64,
            end_time: i64,
            duration_ns: u64,
            name: String,
            span_kind: String,
            service: String,
            operation: String,
            status_code: String,
            status_message: String,
            attributes: HashMap<String, String>,
            resource_attributes: HashMap<String, String>,
            events: Vec<(i64, String, HashMap<String, String>)>,
            links: Vec<(String, String, HashMap<String, String>)>,
        }

        let trace_id = trace_id.to_string();
        let trace_id_for_error = trace_id.clone();
        let client = Arc::clone(&self.client);

        Self::block_on(async move {
            let sql = format!(
                "SELECT trace_id, span_id, parent_span_id, \
                 start_time, end_time, duration_ns, name, span_kind, service, operation, \
                 status_code, status_message, attributes, resource_attributes, \
                 events, links \
                 FROM spans WHERE trace_id = '{}' ORDER BY start_time",
                trace_id.replace('\'', "''")
            );

            let rows: Vec<SpanRow> = client.query(&sql).fetch_all::<SpanRow>().await?;

            if rows.is_empty() {
                return Err(clickhouse::error::Error::Custom(format!(
                    "Trace not found: {trace_id}"
                )));
            }

            // Convert rows to Spans
            let spans: Vec<Span> = rows
                .into_iter()
                .map(|row| {
                    let start_time = DateTime::from_timestamp_nanos(row.start_time);
                    let end_time = DateTime::from_timestamp_nanos(row.end_time);

                    let status = match row.status_code.as_str() {
                        "error" => crate::models::trace::SpanStatus::Error,
                        "cancelled" => crate::models::trace::SpanStatus::Cancelled,
                        _ => crate::models::trace::SpanStatus::Ok,
                    };

                    let kind = match row.span_kind.as_str() {
                        "server" => crate::models::trace::SpanKind::Server,
                        "client" => crate::models::trace::SpanKind::Client,
                        "producer" => crate::models::trace::SpanKind::Producer,
                        "consumer" => crate::models::trace::SpanKind::Consumer,
                        _ => crate::models::trace::SpanKind::Internal,
                    };

                    let attributes: HashMap<String, serde_json::Value> = row
                        .attributes
                        .into_iter()
                        .map(|(k, v)| (k, serde_json::Value::String(v)))
                        .collect();

                    let events: Vec<crate::models::trace::SpanEvent> = row
                        .events
                        .into_iter()
                        .map(|(ts, name, attrs)| {
                            let timestamp = DateTime::from_timestamp_nanos(ts);
                            let event_attrs: HashMap<String, serde_json::Value> = attrs
                                .into_iter()
                                .map(|(k, v)| (k, serde_json::Value::String(v)))
                                .collect();
                            crate::models::trace::SpanEvent {
                                name,
                                timestamp,
                                attributes: event_attrs,
                            }
                        })
                        .collect();

                    Span {
                        trace_id: row.trace_id,
                        span_id: row.span_id,
                        parent_span_id: if row.parent_span_id.is_empty() {
                            None
                        } else {
                            Some(row.parent_span_id)
                        },
                        name: row.name,
                        service: row.service,
                        kind,
                        status,
                        start_time,
                        end_time,
                        attributes,
                        events,
                    }
                })
                .collect();

            Trace::from_spans(spans).ok_or_else(|| {
                clickhouse::error::Error::Custom("Failed to construct trace".to_string())
            })
        })
        .map_err(|e| match e {
            TraceStoreError::StorageError(msg) if msg.contains("Trace not found") => {
                TraceStoreError::NotFound(trace_id_for_error.clone())
            }
            _ => e,
        })
    }

    #[allow(clippy::too_many_lines)]
    fn query(&self, query: TraceQuery) -> Result<TraceQueryResult, TraceStoreError> {
        use std::fmt::Write as _;

        // Define row structure for deserialization
        #[derive(clickhouse::Row, serde::Deserialize)]
        #[allow(dead_code)]
        struct SpanRow {
            trace_id: String,
            span_id: String,
            parent_span_id: String,
            start_time: i64,
            end_time: i64,
            duration_ns: u64,
            name: String,
            span_kind: String,
            service: String,
            operation: String,
            status_code: String,
            status_message: String,
            attributes: HashMap<String, String>,
            resource_attributes: HashMap<String, String>,
            events: Vec<(i64, String, HashMap<String, String>)>,
            links: Vec<(String, String, HashMap<String, String>)>,
        }

        let client = Arc::clone(&self.client);

        Self::block_on(async move {
            // Build SQL to get unique trace IDs matching filters
            let mut sql = String::from("SELECT DISTINCT trace_id FROM spans WHERE 1=1");

            // Add service filter
            if let Some(ref service) = query.service {
                write!(&mut sql, " AND service = '{}'", service.replace('\'', "''")).unwrap();
            }

            // Add time range filters
            if let Some(start) = query.start_time {
                write!(
                    &mut sql,
                    " AND start_time >= {}",
                    start.timestamp_nanos_opt().unwrap_or(0)
                )
                .unwrap();
            }
            if let Some(end) = query.end_time {
                write!(
                    &mut sql,
                    " AND start_time < {}",
                    end.timestamp_nanos_opt().unwrap_or(0)
                )
                .unwrap();
            }

            // Add duration filters
            if let Some(min_duration) = query.min_duration_ms {
                write!(&mut sql, " AND duration_ns >= {}", min_duration * 1_000_000).unwrap();
            }
            if let Some(max_duration) = query.max_duration_ms {
                write!(&mut sql, " AND duration_ns <= {}", max_duration * 1_000_000).unwrap();
            }

            // Add status filter
            if let Some(ref status) = query.status {
                write!(&mut sql, " AND status_code = '{status}'").unwrap();
            }

            sql.push_str(" ORDER BY trace_id DESC");

            // Calculate total count
            let count_sql = sql.replace(
                "SELECT DISTINCT trace_id FROM spans",
                "SELECT count(DISTINCT trace_id) FROM spans",
            );

            // Add limit and offset
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(100);
            write!(&mut sql, " LIMIT {limit} OFFSET {offset}").unwrap();

            // Execute count query
            let total_count: u64 = client.query(&count_sql).fetch_one::<u64>().await?;

            // Execute main query to get trace IDs
            let trace_ids: Vec<String> = client.query(&sql).fetch_all::<String>().await?;

            // Fetch full traces for each ID
            let mut traces = Vec::new();
            for trace_id in trace_ids {
                // Use internal get_trace implementation
                let span_sql = format!(
                    "SELECT trace_id, span_id, parent_span_id, \
                     start_time, end_time, duration_ns, name, span_kind, service, operation, \
                     status_code, status_message, attributes, resource_attributes, \
                     events, links \
                     FROM spans WHERE trace_id = '{}' ORDER BY start_time",
                    trace_id.replace('\'', "''")
                );

                let rows: Vec<SpanRow> = client.query(&span_sql).fetch_all::<SpanRow>().await?;

                let spans: Vec<Span> = rows
                    .into_iter()
                    .map(|row| {
                        let start_time = DateTime::from_timestamp_nanos(row.start_time);
                        let end_time = DateTime::from_timestamp_nanos(row.end_time);

                        let status = match row.status_code.as_str() {
                            "error" => crate::models::trace::SpanStatus::Error,
                            "cancelled" => crate::models::trace::SpanStatus::Cancelled,
                            _ => crate::models::trace::SpanStatus::Ok,
                        };

                        let kind = match row.span_kind.as_str() {
                            "server" => crate::models::trace::SpanKind::Server,
                            "client" => crate::models::trace::SpanKind::Client,
                            "producer" => crate::models::trace::SpanKind::Producer,
                            "consumer" => crate::models::trace::SpanKind::Consumer,
                            _ => crate::models::trace::SpanKind::Internal,
                        };

                        let attributes: HashMap<String, serde_json::Value> = row
                            .attributes
                            .into_iter()
                            .map(|(k, v)| (k, serde_json::Value::String(v)))
                            .collect();

                        let events: Vec<crate::models::trace::SpanEvent> = row
                            .events
                            .into_iter()
                            .map(|(ts, name, attrs)| {
                                let timestamp = DateTime::from_timestamp_nanos(ts);
                                let event_attrs: HashMap<String, serde_json::Value> = attrs
                                    .into_iter()
                                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                                    .collect();
                                crate::models::trace::SpanEvent {
                                    name,
                                    timestamp,
                                    attributes: event_attrs,
                                }
                            })
                            .collect();

                        Span {
                            trace_id: row.trace_id,
                            span_id: row.span_id,
                            parent_span_id: if row.parent_span_id.is_empty() {
                                None
                            } else {
                                Some(row.parent_span_id)
                            },
                            name: row.name,
                            service: row.service,
                            kind,
                            status,
                            start_time,
                            end_time,
                            attributes,
                            events,
                        }
                    })
                    .collect();

                if let Some(trace) = Trace::from_spans(spans) {
                    traces.push(trace);
                }
            }

            Ok(TraceQueryResult {
                traces,
                total_count: usize::try_from(total_count).unwrap_or(usize::MAX),
            })
        })
    }

    fn span_count(&self) -> Result<usize, TraceStoreError> {
        let client = Arc::clone(&self.client);
        let count: u64 = Self::block_on(async move {
            client
                .query("SELECT count() FROM spans")
                .fetch_one::<u64>()
                .await
        })?;

        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    fn trace_count(&self) -> Result<usize, TraceStoreError> {
        let client = Arc::clone(&self.client);
        let count: u64 = Self::block_on(async move {
            client
                .query("SELECT count(DISTINCT trace_id) FROM spans")
                .fetch_one::<u64>()
                .await
        })?;

        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    fn clear(&self) -> Result<(), TraceStoreError> {
        let client = Arc::clone(&self.client);
        Self::block_on(async move { client.query("TRUNCATE TABLE spans").execute().await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_span(trace_id: &str, span_id: &str, service: &str) -> Span {
        Span::new(trace_id, span_id, "test operation", service)
    }

    #[test]
    fn test_new_store_is_empty() {
        let store = InMemoryTraceStore::new();
        assert_eq!(store.span_count().unwrap(), 0);
        assert_eq!(store.trace_count().unwrap(), 0);
    }

    #[test]
    fn test_insert_span() {
        let store = InMemoryTraceStore::new();
        let span = create_test_span("trace-1", "span-1", "api");

        store.insert_span(span).unwrap();

        assert_eq!(store.span_count().unwrap(), 1);
        assert_eq!(store.trace_count().unwrap(), 1);
    }

    #[test]
    fn test_insert_spans_same_trace() {
        let store = InMemoryTraceStore::new();
        let spans = vec![
            create_test_span("trace-1", "span-1", "api"),
            create_test_span("trace-1", "span-2", "db"),
            create_test_span("trace-1", "span-3", "cache"),
        ];

        store.insert_spans(spans).unwrap();

        assert_eq!(store.span_count().unwrap(), 3);
        assert_eq!(store.trace_count().unwrap(), 1);
    }

    #[test]
    fn test_insert_spans_different_traces() {
        let store = InMemoryTraceStore::new();
        let spans = vec![
            create_test_span("trace-1", "span-1", "api"),
            create_test_span("trace-2", "span-2", "api"),
        ];

        store.insert_spans(spans).unwrap();

        assert_eq!(store.span_count().unwrap(), 2);
        assert_eq!(store.trace_count().unwrap(), 2);
    }

    #[test]
    fn test_get_trace() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-1", "span-2", "db"))
            .unwrap();

        let trace = store.get_trace("trace-1").unwrap();

        assert_eq!(trace.trace_id, "trace-1");
        assert_eq!(trace.span_count(), 2);
    }

    #[test]
    fn test_get_trace_not_found() {
        let store = InMemoryTraceStore::new();

        let result = store.get_trace("nonexistent");

        assert!(matches!(result, Err(TraceStoreError::NotFound(_))));
    }

    #[test]
    fn test_query_all_traces() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-2", "span-2", "db"))
            .unwrap();

        let result = store.query(TraceQuery::new()).unwrap();

        assert_eq!(result.total_count, 2);
        assert_eq!(result.traces.len(), 2);
    }

    #[test]
    fn test_query_by_service() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-2", "span-2", "db"))
            .unwrap();
        store
            .insert_span(create_test_span("trace-3", "span-3", "api"))
            .unwrap();

        let result = store.query(TraceQuery::new().with_service("api")).unwrap();

        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_query_by_duration() {
        let store = InMemoryTraceStore::new();

        // Fast trace (10ms)
        let fast = Span::new("trace-1", "span-1", "fast", "api")
            .with_start_time(Utc::now())
            .with_end_time(Utc::now() + Duration::milliseconds(10));
        store.insert_span(fast).unwrap();

        // Slow trace (500ms)
        let slow = Span::new("trace-2", "span-2", "slow", "api")
            .with_start_time(Utc::now())
            .with_end_time(Utc::now() + Duration::milliseconds(500));
        store.insert_span(slow).unwrap();

        let result = store
            .query(TraceQuery::new().with_min_duration_ms(100))
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.traces[0].trace_id, "trace-2");
    }

    #[test]
    fn test_query_by_status() {
        let store = InMemoryTraceStore::new();

        store
            .insert_span(
                Span::new("trace-1", "span-1", "success", "api").with_status(SpanStatus::Ok),
            )
            .unwrap();
        store
            .insert_span(
                Span::new("trace-2", "span-2", "failure", "api").with_status(SpanStatus::Error),
            )
            .unwrap();

        let result = store
            .query(TraceQuery::new().with_status(SpanStatus::Error))
            .unwrap();

        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn test_query_with_limit() {
        let store = InMemoryTraceStore::new();

        for i in 0..10 {
            store
                .insert_span(create_test_span(
                    &format!("trace-{i}"),
                    &format!("span-{i}"),
                    "api",
                ))
                .unwrap();
        }

        let result = store.query(TraceQuery::new().with_limit(5)).unwrap();

        assert_eq!(result.traces.len(), 5);
        assert_eq!(result.total_count, 10);
    }

    #[test]
    fn test_clear_store() {
        let store = InMemoryTraceStore::new();
        store
            .insert_span(create_test_span("trace-1", "span-1", "api"))
            .unwrap();

        store.clear().unwrap();

        assert_eq!(store.span_count().unwrap(), 0);
        assert_eq!(store.trace_count().unwrap(), 0);
    }
}
