//! Log storage trait and implementations.
//!
//! Provides the `LogStore` trait for abstracting log storage operations
//! and an `InMemoryLogStore` implementation for development and testing.

use crate::models::{LogEntry, LogLevel};
use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors that can occur during log store operations.
#[derive(Debug, Error)]
pub enum LogStoreError {
    /// Failed to acquire lock on the store.
    #[error("Failed to acquire lock on log store")]
    LockError,

    /// Generic storage error.
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Query parameters for retrieving logs.
#[derive(Debug, Clone, Default)]
pub struct LogQuery {
    /// Filter logs starting from this time (inclusive).
    pub start_time: Option<DateTime<Utc>>,

    /// Filter logs up to this time (exclusive).
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by log level.
    pub level: Option<LogLevel>,

    /// Filter by service name (exact match).
    pub service: Option<String>,

    /// Filter by message content (case-insensitive substring match).
    pub message_contains: Option<String>,

    /// Maximum number of logs to return.
    pub limit: Option<usize>,

    /// Number of logs to skip (for pagination).
    pub offset: Option<usize>,
}

impl LogQuery {
    /// Creates a new empty query (returns all logs).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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

    /// Sets the log level filter.
    #[must_use]
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Sets the service name filter (exact match).
    #[must_use]
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Sets the message contains filter (case-insensitive substring match).
    #[must_use]
    pub fn with_message_contains(mut self, pattern: impl Into<String>) -> Self {
        self.message_contains = Some(pattern.into());
        self
    }
}

/// Result of a log query operation.
#[derive(Debug, Clone)]
pub struct LogQueryResult {
    /// The logs matching the query.
    pub logs: Vec<LogEntry>,

    /// Total count of matching logs (before limit/offset applied).
    pub total_count: usize,
}

/// Trait for log storage implementations.
///
/// This trait defines the interface for storing and querying logs.
/// Implementations must be thread-safe (Send + Sync).
pub trait LogStore: Send + Sync {
    /// Inserts a single log entry into the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert(&self, entry: LogEntry) -> Result<(), LogStoreError>;

    /// Inserts multiple log entries into the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn insert_batch(&self, entries: Vec<LogEntry>) -> Result<(), LogStoreError>;

    /// Queries logs based on the provided parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the query operation fails.
    fn query(&self, query: LogQuery) -> Result<LogQueryResult, LogStoreError>;

    /// Returns the total number of logs in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the count operation fails.
    fn count(&self) -> Result<usize, LogStoreError>;

    /// Clears all logs from the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the clear operation fails.
    fn clear(&self) -> Result<(), LogStoreError>;

    /// Returns the timestamp of the oldest log entry in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    fn get_oldest_timestamp(&self) -> Result<Option<DateTime<Utc>>, LogStoreError>;

    /// Returns the timestamp of the newest log entry in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    fn get_newest_timestamp(&self) -> Result<Option<DateTime<Utc>>, LogStoreError>;
}

/// In-memory log store implementation.
///
/// This implementation stores logs in a `Vec` protected by a `RwLock`.
/// It is suitable for development, testing, and single-node deployments
/// with limited data volumes.
///
/// **Note:** Data is not persisted across restarts.
///
/// # Example
///
/// ```
/// use shared::storage::{InMemoryLogStore, LogStore, LogQuery};
/// use shared::models::{LogEntry, LogLevel};
///
/// let store = InMemoryLogStore::new();
///
/// // Insert a log
/// let log = LogEntry::new(LogLevel::Info, "Test message", "test-service");
/// store.insert(log).unwrap();
///
/// // Query logs
/// let result = store.query(LogQuery::new()).unwrap();
/// assert_eq!(result.logs.len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct InMemoryLogStore {
    logs: Arc<RwLock<Vec<LogEntry>>>,
}

impl InMemoryLogStore {
    /// Creates a new empty in-memory log store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Creates a new in-memory log store wrapped in an Arc.
    ///
    /// This is useful when sharing the store across multiple handlers.
    #[must_use]
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl LogStore for InMemoryLogStore {
    fn insert(&self, entry: LogEntry) -> Result<(), LogStoreError> {
        let mut logs = self.logs.write().map_err(|_| LogStoreError::LockError)?;
        logs.push(entry);
        Ok(())
    }

    fn insert_batch(&self, entries: Vec<LogEntry>) -> Result<(), LogStoreError> {
        let mut logs = self.logs.write().map_err(|_| LogStoreError::LockError)?;
        logs.extend(entries);
        Ok(())
    }

    fn query(&self, query: LogQuery) -> Result<LogQueryResult, LogStoreError> {
        let logs = self.logs.read().map_err(|_| LogStoreError::LockError)?;

        // Prepare case-insensitive message search pattern
        let message_pattern = query.message_contains.as_ref().map(|s| s.to_lowercase());

        // Apply all filters
        let filtered: Vec<LogEntry> = logs
            .iter()
            .filter(|log| {
                // Time range filter
                if let Some(start) = query.start_time {
                    if log.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = query.end_time {
                    if log.timestamp >= end {
                        return false;
                    }
                }

                // Level filter
                if let Some(ref level) = query.level {
                    if &log.level != level {
                        return false;
                    }
                }

                // Service filter (exact match)
                if let Some(ref service) = query.service {
                    if &log.service != service {
                        return false;
                    }
                }

                // Message contains filter (case-insensitive)
                if let Some(ref pattern) = message_pattern {
                    if !log.message.to_lowercase().contains(pattern) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        let total_count = filtered.len();

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let result: Vec<LogEntry> = filtered
            .into_iter()
            .skip(offset)
            .take(query.limit.unwrap_or(usize::MAX))
            .collect();

        Ok(LogQueryResult {
            logs: result,
            total_count,
        })
    }

    fn count(&self) -> Result<usize, LogStoreError> {
        let logs = self.logs.read().map_err(|_| LogStoreError::LockError)?;
        Ok(logs.len())
    }

    fn clear(&self) -> Result<(), LogStoreError> {
        let mut logs = self.logs.write().map_err(|_| LogStoreError::LockError)?;
        logs.clear();
        Ok(())
    }

    fn get_oldest_timestamp(&self) -> Result<Option<DateTime<Utc>>, LogStoreError> {
        let logs = self.logs.read().map_err(|_| LogStoreError::LockError)?;
        Ok(logs.iter().map(|log| log.timestamp).min())
    }

    fn get_newest_timestamp(&self) -> Result<Option<DateTime<Utc>>, LogStoreError> {
        let logs = self.logs.read().map_err(|_| LogStoreError::LockError)?;
        Ok(logs.iter().map(|log| log.timestamp).max())
    }
}

/// `ClickHouse`-backed log store implementation.
///
/// This implementation stores logs in `ClickHouse` for production use.
/// It provides persistent storage and efficient time-series queries.
#[derive(Clone)]
pub struct ClickHouseLogStore {
    client: Arc<clickhouse::Client>,
}

impl ClickHouseLogStore {
    /// Creates a new `ClickHouse` log store with the given client.
    #[must_use]
    pub fn new(client: Arc<clickhouse::Client>) -> Self {
        Self { client }
    }

    /// Creates a new `ClickHouse` log store wrapped in an Arc.
    #[must_use]
    pub fn new_shared(client: Arc<clickhouse::Client>) -> Arc<Self> {
        Arc::new(Self::new(client))
    }

    /// Helper to execute async operations synchronously.
    fn block_on<F, T>(future: F) -> Result<T, LogStoreError>
    where
        F: std::future::Future<Output = Result<T, clickhouse::error::Error>>,
    {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(future)
                .map_err(|e| LogStoreError::StorageError(e.to_string()))
        })
    }
}

impl LogStore for ClickHouseLogStore {
    fn insert(&self, entry: LogEntry) -> Result<(), LogStoreError> {
        self.insert_batch(vec![entry])
    }

    fn insert_batch(&self, entries: Vec<LogEntry>) -> Result<(), LogStoreError> {
        if entries.is_empty() {
            return Ok(());
        }

        let client = Arc::clone(&self.client);
        Self::block_on(async move {
            #[derive(clickhouse::Row, serde::Serialize)]
            struct LogRow {
                timestamp: i64,
                trace_id: String,
                span_id: String,
                level: String,
                message: String,
                service: String,
                attributes: std::collections::HashMap<String, String>,
            }

            let mut inserter = client.insert::<LogRow>("logs").await?;

            for entry in entries {
                // Convert attributes HashMap<String, serde_json::Value> to Map(String, String)
                let attributes: std::collections::HashMap<String, String> = entry
                    .attributes
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect();

                let row = LogRow {
                    timestamp: entry.timestamp.timestamp_nanos_opt().unwrap_or(0),
                    trace_id: entry.trace_id.unwrap_or_default(),
                    span_id: entry.span_id.unwrap_or_default(),
                    level: entry.level.to_string(),
                    message: entry.message,
                    service: entry.service,
                    attributes,
                };

                inserter.write(&row).await?;
            }

            inserter.end().await?;
            Ok(())
        })
    }

    fn query(&self, query: LogQuery) -> Result<LogQueryResult, LogStoreError> {
        use std::fmt::Write as _;

        // Define row structure for deserialization
        #[derive(clickhouse::Row, serde::Deserialize)]
        struct LogRow {
            timestamp: i64,
            trace_id: String,
            span_id: String,
            level: String,
            message: String,
            service: String,
            attributes: std::collections::HashMap<String, String>,
        }

        // Build SQL query
        let mut sql = String::from("SELECT timestamp, trace_id, span_id, level, message, service, attributes FROM logs WHERE 1=1");

        // Add time range filters
        if let Some(start) = query.start_time {
            write!(
                &mut sql,
                " AND timestamp >= {}",
                start.timestamp_nanos_opt().unwrap_or(0)
            )
            .unwrap();
        }
        if let Some(end) = query.end_time {
            write!(
                &mut sql,
                " AND timestamp < {}",
                end.timestamp_nanos_opt().unwrap_or(0)
            )
            .unwrap();
        }

        // Add level filter
        if let Some(ref level) = query.level {
            write!(&mut sql, " AND level = '{level}'").unwrap();
        }

        // Add service filter
        if let Some(ref service) = query.service {
            write!(&mut sql, " AND service = '{}'", service.replace('\'', "''")).unwrap();
        }

        // Add message search filter
        if let Some(ref pattern) = query.message_contains {
            write!(
                &mut sql,
                " AND position(lower(message), '{}') > 0",
                pattern.to_lowercase().replace('\'', "''")
            )
            .unwrap();
        }

        // Add ordering
        sql.push_str(" ORDER BY timestamp DESC");

        // Calculate total count query
        let count_sql = sql.replace(
            "SELECT timestamp, trace_id, span_id, level, message, service, attributes FROM logs",
            "SELECT count() FROM logs",
        );

        // Add limit and offset
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(1000);
        write!(&mut sql, " LIMIT {limit} OFFSET {offset}").unwrap();

        let client = Arc::clone(&self.client);
        let count_sql_clone = count_sql.clone();

        // Execute queries
        Self::block_on(async move {
            // Execute count query
            let total_count: u64 = client.query(&count_sql_clone).fetch_one::<u64>().await?;

            // Execute main query
            let rows: Vec<LogRow> = client.query(&sql).fetch_all::<LogRow>().await?;

            // Convert rows to LogEntry
            let logs: Vec<LogEntry> = rows
                .into_iter()
                .map(|row| {
                    let timestamp = DateTime::from_timestamp_nanos(row.timestamp);
                    let level = match row.level.as_str() {
                        "trace" => LogLevel::Trace,
                        "debug" => LogLevel::Debug,
                        "warn" => LogLevel::Warn,
                        "error" => LogLevel::Error,
                        "fatal" => LogLevel::Fatal,
                        _ => LogLevel::Info,
                    };
                    let attributes: std::collections::HashMap<String, serde_json::Value> = row
                        .attributes
                        .into_iter()
                        .map(|(k, v)| (k, serde_json::Value::String(v)))
                        .collect();

                    LogEntry {
                        timestamp,
                        level,
                        message: row.message,
                        service: row.service,
                        attributes,
                        trace_id: if row.trace_id.is_empty() {
                            None
                        } else {
                            Some(row.trace_id)
                        },
                        span_id: if row.span_id.is_empty() {
                            None
                        } else {
                            Some(row.span_id)
                        },
                    }
                })
                .collect();

            Ok(LogQueryResult {
                logs,
                total_count: usize::try_from(total_count).unwrap_or(usize::MAX),
            })
        })
    }

    fn count(&self) -> Result<usize, LogStoreError> {
        let client = Arc::clone(&self.client);
        let count: u64 = Self::block_on(async move {
            client
                .query("SELECT count() FROM logs")
                .fetch_one::<u64>()
                .await
        })?;

        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    fn clear(&self) -> Result<(), LogStoreError> {
        let client = Arc::clone(&self.client);
        Self::block_on(async move { client.query("TRUNCATE TABLE logs").execute().await })
    }

    fn get_oldest_timestamp(&self) -> Result<Option<DateTime<Utc>>, LogStoreError> {
        let client = Arc::clone(&self.client);
        Self::block_on(async move {
            let sql = "SELECT min(timestamp) FROM logs";
            let result: Option<i64> = client.query(sql).fetch_optional::<i64>().await?;
            Ok(result.map(DateTime::from_timestamp_nanos))
        })
    }

    fn get_newest_timestamp(&self) -> Result<Option<DateTime<Utc>>, LogStoreError> {
        let client = Arc::clone(&self.client);
        Self::block_on(async move {
            let sql = "SELECT max(timestamp) FROM logs";
            let result: Option<i64> = client.query(sql).fetch_optional::<i64>().await?;
            Ok(result.map(DateTime::from_timestamp_nanos))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LogLevel;
    use chrono::Duration;

    fn create_test_log(message: &str) -> LogEntry {
        LogEntry::new(LogLevel::Info, message, "test-service")
    }

    fn create_test_log_with_timestamp(message: &str, timestamp: DateTime<Utc>) -> LogEntry {
        LogEntry {
            timestamp,
            level: LogLevel::Info,
            message: message.to_string(),
            service: "test-service".to_string(),
            attributes: std::collections::HashMap::new(),
            trace_id: None,
            span_id: None,
        }
    }

    #[test]
    fn test_new_store_is_empty() {
        let store = InMemoryLogStore::new();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_insert_single_log() {
        let store = InMemoryLogStore::new();
        let log = create_test_log("Test message");

        store.insert(log).unwrap();

        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_insert_batch() {
        let store = InMemoryLogStore::new();
        let logs = vec![
            create_test_log("Log 1"),
            create_test_log("Log 2"),
            create_test_log("Log 3"),
        ];

        store.insert_batch(logs).unwrap();

        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn test_query_all_logs() {
        let store = InMemoryLogStore::new();
        store.insert(create_test_log("Log 1")).unwrap();
        store.insert(create_test_log("Log 2")).unwrap();

        let result = store.query(LogQuery::new()).unwrap();

        assert_eq!(result.logs.len(), 2);
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_query_with_limit() {
        let store = InMemoryLogStore::new();
        for i in 0..10 {
            store.insert(create_test_log(&format!("Log {i}"))).unwrap();
        }

        let result = store.query(LogQuery::new().with_limit(5)).unwrap();

        assert_eq!(result.logs.len(), 5);
        assert_eq!(result.total_count, 10);
    }

    #[test]
    fn test_query_with_offset() {
        let store = InMemoryLogStore::new();
        for i in 0..10 {
            store.insert(create_test_log(&format!("Log {i}"))).unwrap();
        }

        let result = store.query(LogQuery::new().with_offset(5)).unwrap();

        assert_eq!(result.logs.len(), 5);
        assert_eq!(result.total_count, 10);
        assert_eq!(result.logs[0].message, "Log 5");
    }

    #[test]
    fn test_query_with_limit_and_offset() {
        let store = InMemoryLogStore::new();
        for i in 0..10 {
            store.insert(create_test_log(&format!("Log {i}"))).unwrap();
        }

        let result = store
            .query(LogQuery::new().with_offset(3).with_limit(3))
            .unwrap();

        assert_eq!(result.logs.len(), 3);
        assert_eq!(result.total_count, 10);
        assert_eq!(result.logs[0].message, "Log 3");
        assert_eq!(result.logs[2].message, "Log 5");
    }

    #[test]
    fn test_query_with_time_range() {
        let store = InMemoryLogStore::new();
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let two_hours_ago = now - Duration::hours(2);
        let three_hours_ago = now - Duration::hours(3);

        store
            .insert(create_test_log_with_timestamp("Old log", three_hours_ago))
            .unwrap();
        store
            .insert(create_test_log_with_timestamp(
                "Medium old log",
                two_hours_ago,
            ))
            .unwrap();
        store
            .insert(create_test_log_with_timestamp("Recent log", one_hour_ago))
            .unwrap();
        store
            .insert(create_test_log_with_timestamp("Current log", now))
            .unwrap();

        // Query logs from 2.5 hours ago to 30 minutes ago
        let query = LogQuery::new()
            .with_start_time(now - Duration::minutes(150))
            .with_end_time(now - Duration::minutes(30));

        let result = store.query(query).unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result.logs.iter().any(|l| l.message == "Medium old log"));
        assert!(result.logs.iter().any(|l| l.message == "Recent log"));
    }

    #[test]
    fn test_query_start_time_inclusive() {
        let store = InMemoryLogStore::new();
        let timestamp = Utc::now();

        store
            .insert(create_test_log_with_timestamp("Exact time log", timestamp))
            .unwrap();

        let result = store
            .query(LogQuery::new().with_start_time(timestamp))
            .unwrap();

        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn test_query_end_time_exclusive() {
        let store = InMemoryLogStore::new();
        let timestamp = Utc::now();

        store
            .insert(create_test_log_with_timestamp("Exact time log", timestamp))
            .unwrap();

        let result = store
            .query(LogQuery::new().with_end_time(timestamp))
            .unwrap();

        assert_eq!(result.total_count, 0);
    }

    #[test]
    fn test_clear_store() {
        let store = InMemoryLogStore::new();
        store.insert(create_test_log("Log 1")).unwrap();
        store.insert(create_test_log("Log 2")).unwrap();

        assert_eq!(store.count().unwrap(), 2);

        store.clear().unwrap();

        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_store_is_thread_safe() {
        use std::thread;

        let store = InMemoryLogStore::new_shared();
        let mut handles = vec![];

        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = thread::spawn(move || {
                store_clone
                    .insert(create_test_log(&format!("Thread {i} log")))
                    .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(store.count().unwrap(), 10);
    }

    #[test]
    fn test_query_empty_store() {
        let store = InMemoryLogStore::new();

        let result = store.query(LogQuery::new()).unwrap();

        assert_eq!(result.logs.len(), 0);
        assert_eq!(result.total_count, 0);
    }

    #[test]
    fn test_query_builder_pattern() {
        let query = LogQuery::new()
            .with_start_time(Utc::now() - Duration::hours(1))
            .with_end_time(Utc::now())
            .with_limit(100)
            .with_offset(10);

        assert!(query.start_time.is_some());
        assert!(query.end_time.is_some());
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(10));
    }

    // ========== Filter tests ==========

    fn create_test_log_with_level(message: &str, level: LogLevel) -> LogEntry {
        LogEntry::new(level, message, "test-service")
    }

    fn create_test_log_with_service(message: &str, service: &str) -> LogEntry {
        LogEntry::new(LogLevel::Info, message, service)
    }

    #[test]
    fn test_query_filter_by_level() {
        let store = InMemoryLogStore::new();

        store
            .insert(create_test_log_with_level("Debug message", LogLevel::Debug))
            .unwrap();
        store
            .insert(create_test_log_with_level("Info message", LogLevel::Info))
            .unwrap();
        store
            .insert(create_test_log_with_level("Error message", LogLevel::Error))
            .unwrap();
        store
            .insert(create_test_log_with_level("Another error", LogLevel::Error))
            .unwrap();

        let result = store
            .query(LogQuery::new().with_level(LogLevel::Error))
            .unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result.logs.iter().all(|l| l.level == LogLevel::Error));
    }

    #[test]
    fn test_query_filter_by_service() {
        let store = InMemoryLogStore::new();

        store
            .insert(create_test_log_with_service("Log from api", "api"))
            .unwrap();
        store
            .insert(create_test_log_with_service(
                "Log from auth",
                "auth-service",
            ))
            .unwrap();
        store
            .insert(create_test_log_with_service("Another api log", "api"))
            .unwrap();
        store
            .insert(create_test_log_with_service("Database log", "db-service"))
            .unwrap();

        let result = store.query(LogQuery::new().with_service("api")).unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result.logs.iter().all(|l| l.service == "api"));
    }

    #[test]
    fn test_query_filter_by_message_contains() {
        let store = InMemoryLogStore::new();

        store.insert(create_test_log("User logged in")).unwrap();
        store
            .insert(create_test_log("Payment processed successfully"))
            .unwrap();
        store.insert(create_test_log("User logged out")).unwrap();
        store
            .insert(create_test_log("Database connection failed"))
            .unwrap();

        let result = store
            .query(LogQuery::new().with_message_contains("user"))
            .unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result
            .logs
            .iter()
            .all(|l| l.message.to_lowercase().contains("user")));
    }

    #[test]
    fn test_query_filter_message_contains_case_insensitive() {
        let store = InMemoryLogStore::new();

        store.insert(create_test_log("ERROR occurred")).unwrap();
        store.insert(create_test_log("Error in module")).unwrap();
        store.insert(create_test_log("error message")).unwrap();
        store.insert(create_test_log("No problems here")).unwrap();

        let result = store
            .query(LogQuery::new().with_message_contains("ERROR"))
            .unwrap();

        assert_eq!(result.total_count, 3);
    }

    #[test]
    fn test_query_combined_filters() {
        let store = InMemoryLogStore::new();

        // Insert logs with various combinations
        store
            .insert(LogEntry::new(
                LogLevel::Error,
                "Database connection failed",
                "db-service",
            ))
            .unwrap();
        store
            .insert(LogEntry::new(
                LogLevel::Error,
                "Auth token expired",
                "auth-service",
            ))
            .unwrap();
        store
            .insert(LogEntry::new(
                LogLevel::Info,
                "Database query completed",
                "db-service",
            ))
            .unwrap();
        store
            .insert(LogEntry::new(
                LogLevel::Error,
                "Database timeout",
                "db-service",
            ))
            .unwrap();

        // Query: errors from db-service containing "database"
        let result = store
            .query(
                LogQuery::new()
                    .with_level(LogLevel::Error)
                    .with_service("db-service")
                    .with_message_contains("database"),
            )
            .unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result.logs.iter().all(|l| l.level == LogLevel::Error
            && l.service == "db-service"
            && l.message.to_lowercase().contains("database")));
    }

    #[test]
    fn test_query_filter_with_pagination() {
        let store = InMemoryLogStore::new();

        for i in 0..10 {
            store
                .insert(LogEntry::new(LogLevel::Error, format!("Error {i}"), "api"))
                .unwrap();
        }
        for i in 0..5 {
            store
                .insert(LogEntry::new(LogLevel::Info, format!("Info {i}"), "api"))
                .unwrap();
        }

        let result = store
            .query(
                LogQuery::new()
                    .with_level(LogLevel::Error)
                    .with_limit(3)
                    .with_offset(2),
            )
            .unwrap();

        assert_eq!(result.total_count, 10); // Total errors before pagination
        assert_eq!(result.logs.len(), 3); // After limit
        assert_eq!(result.logs[0].message, "Error 2"); // After offset
    }

    #[test]
    fn test_query_filter_no_matches() {
        let store = InMemoryLogStore::new();

        store.insert(create_test_log("Some message")).unwrap();
        store.insert(create_test_log("Another message")).unwrap();

        let result = store
            .query(LogQuery::new().with_level(LogLevel::Fatal))
            .unwrap();

        assert_eq!(result.total_count, 0);
        assert!(result.logs.is_empty());
    }

    #[test]
    fn test_query_filter_service_exact_match() {
        let store = InMemoryLogStore::new();

        store
            .insert(create_test_log_with_service("Log", "api"))
            .unwrap();
        store
            .insert(create_test_log_with_service("Log", "api-gateway"))
            .unwrap();
        store
            .insert(create_test_log_with_service("Log", "internal-api"))
            .unwrap();

        // Should only match exact "api", not "api-gateway" or "internal-api"
        let result = store.query(LogQuery::new().with_service("api")).unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].service, "api");
    }

    #[test]
    fn test_query_builder_with_all_filters() {
        let query = LogQuery::new()
            .with_start_time(Utc::now() - Duration::hours(1))
            .with_end_time(Utc::now())
            .with_level(LogLevel::Error)
            .with_service("api")
            .with_message_contains("failed")
            .with_limit(100)
            .with_offset(10);

        assert!(query.start_time.is_some());
        assert!(query.end_time.is_some());
        assert_eq!(query.level, Some(LogLevel::Error));
        assert_eq!(query.service, Some("api".to_string()));
        assert_eq!(query.message_contains, Some("failed".to_string()));
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(10));
    }
}
