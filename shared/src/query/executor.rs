//! Query execution engine.
//!
//! Executes parsed SQL-like queries against the log store.

use super::ast::{
    ComparisonOp, Condition, LogicalOp, OrderBy, Query, SortOrder, Source, Value, WhereClause,
};
use crate::models::{LogEntry, LogLevel};
use crate::storage::{LogQuery, LogQueryResult, LogStore, LogStoreError};
use thiserror::Error;

/// Errors that can occur during query execution.
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// The query source is not supported.
    #[error("Unsupported query source: {0}. Only 'logs' is currently supported.")]
    UnsupportedSource(String),

    /// The field is not recognized.
    #[error("Unknown field: '{0}'")]
    UnknownField(String),

    /// Type mismatch in comparison.
    #[error("Type mismatch: cannot compare {field} with {value_type}")]
    TypeMismatch {
        /// The field being compared.
        field: String,
        /// The type of the value.
        value_type: String,
    },

    /// Storage error during execution.
    #[error("Storage error: {0}")]
    StorageError(#[from] LogStoreError),
}

/// Executes a parsed query against a log store.
///
/// # Arguments
///
/// * `query` - The parsed query AST.
/// * `store` - The log store to query.
///
/// # Returns
///
/// Returns a `LogQueryResult` containing the matching logs.
///
/// # Errors
///
/// Returns an error if the query cannot be executed.
///
/// # Example
///
/// ```ignore
/// use shared::query::{parse_query, execute_query};
/// use shared::storage::InMemoryLogStore;
///
/// let store = InMemoryLogStore::new();
/// let query = parse_query("SELECT * FROM logs WHERE level = 'error'").unwrap();
/// let result = execute_query(&query, &store).unwrap();
/// ```
pub fn execute_query(
    query: &Query,
    store: &dyn LogStore,
) -> Result<LogQueryResult, ExecutionError> {
    // Currently only logs are supported
    if query.source != Source::Logs {
        return Err(ExecutionError::UnsupportedSource(query.source.to_string()));
    }

    // Get all logs first (we'll filter in-memory for complex conditions)
    let all_logs = store.query(LogQuery::new())?;

    // Apply WHERE clause filter
    let filtered: Vec<LogEntry> = if let Some(ref where_clause) = query.where_clause {
        all_logs
            .logs
            .into_iter()
            .filter(|log| evaluate_where_clause(where_clause, log))
            .collect()
    } else {
        all_logs.logs
    };

    // Apply ORDER BY
    let mut sorted = filtered;
    if let Some(ref order_by) = query.order_by {
        sort_logs(&mut sorted, order_by);
    }

    let total_count = sorted.len();

    // Apply OFFSET and LIMIT
    let offset = query.offset.unwrap_or(0);
    let result: Vec<LogEntry> = sorted
        .into_iter()
        .skip(offset)
        .take(query.limit.unwrap_or(usize::MAX))
        .collect();

    Ok(LogQueryResult {
        logs: result,
        total_count,
    })
}

/// Evaluates a WHERE clause against a log entry.
fn evaluate_where_clause(clause: &WhereClause, log: &LogEntry) -> bool {
    match clause {
        WhereClause::Condition(condition) => evaluate_condition(condition, log),
        WhereClause::Combined {
            left,
            operator,
            right,
        } => {
            let left_result = evaluate_where_clause(left, log);
            let right_result = evaluate_where_clause(right, log);

            match operator {
                LogicalOp::And => left_result && right_result,
                LogicalOp::Or => left_result || right_result,
            }
        }
        WhereClause::Grouped(inner) => evaluate_where_clause(inner, log),
    }
}

/// Evaluates a single condition against a log entry.
fn evaluate_condition(condition: &Condition, log: &LogEntry) -> bool {
    let field = condition.field.to_lowercase();

    match field.as_str() {
        "level" => evaluate_level_condition(condition, log),
        "service" => evaluate_string_field(&log.service, condition),
        "message" => evaluate_string_field(&log.message, condition),
        "trace_id" => {
            if let Some(ref trace_id) = log.trace_id {
                evaluate_string_field(trace_id, condition)
            } else {
                // If trace_id is None, only match if comparing with empty string or not-equal
                matches!(&condition.value, Value::String(s) if s.is_empty())
                    || condition.operator == ComparisonOp::NotEq
            }
        }
        "span_id" => {
            if let Some(ref span_id) = log.span_id {
                evaluate_string_field(span_id, condition)
            } else {
                matches!(&condition.value, Value::String(s) if s.is_empty())
                    || condition.operator == ComparisonOp::NotEq
            }
        }
        "timestamp" => evaluate_timestamp_condition(condition, log),
        _ => {
            // Check in attributes
            if let Some(attr_value) = log.attributes.get(&condition.field) {
                evaluate_attribute_condition(attr_value, condition)
            } else {
                // Field not found - return false for equality, true for not-equal
                condition.operator == ComparisonOp::NotEq
            }
        }
    }
}

/// Evaluates a condition against the log level.
fn evaluate_level_condition(condition: &Condition, log: &LogEntry) -> bool {
    let log_level_str = log.level.to_string();

    match &condition.value {
        Value::String(s) => {
            let value_lower = s.to_lowercase();
            match condition.operator {
                ComparisonOp::Eq => log_level_str == value_lower,
                ComparisonOp::NotEq => log_level_str != value_lower,
                ComparisonOp::Contains => log_level_str.contains(&value_lower),
                ComparisonOp::StartsWith => log_level_str.starts_with(&value_lower),
                ComparisonOp::EndsWith => log_level_str.ends_with(&value_lower),
                // Comparison operators for levels (using severity order)
                ComparisonOp::Lt | ComparisonOp::LtEq | ComparisonOp::Gt | ComparisonOp::GtEq => {
                    if let Some(val_ord) = level_order_from_str(&value_lower) {
                        let log_ord = level_order(log.level);
                        match condition.operator {
                            ComparisonOp::Lt => log_ord < val_ord,
                            ComparisonOp::LtEq => log_ord <= val_ord,
                            ComparisonOp::Gt => log_ord > val_ord,
                            ComparisonOp::GtEq => log_ord >= val_ord,
                            _ => false,
                        }
                    } else {
                        false
                    }
                }
            }
        }
        _ => false, // Level must be compared with string
    }
}

/// Returns the severity order of a log level (lower = less severe).
fn level_order(level: LogLevel) -> u8 {
    match level {
        LogLevel::Trace => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
        LogLevel::Fatal => 5,
    }
}

/// Returns the severity order from a level string.
fn level_order_from_str(s: &str) -> Option<u8> {
    match s {
        "trace" => Some(0),
        "debug" => Some(1),
        "info" => Some(2),
        "warn" => Some(3),
        "error" => Some(4),
        "fatal" => Some(5),
        _ => None,
    }
}

/// Evaluates a string field condition.
fn evaluate_string_field(field_value: &str, condition: &Condition) -> bool {
    match &condition.value {
        Value::String(s) => {
            let field_lower = field_value.to_lowercase();
            let value_lower = s.to_lowercase();

            match condition.operator {
                ComparisonOp::Eq => field_lower == value_lower,
                ComparisonOp::NotEq => field_lower != value_lower,
                ComparisonOp::Contains => field_lower.contains(&value_lower),
                ComparisonOp::StartsWith => field_lower.starts_with(&value_lower),
                ComparisonOp::EndsWith => field_lower.ends_with(&value_lower),
                ComparisonOp::Lt => field_value < s.as_str(),
                ComparisonOp::LtEq => field_value <= s.as_str(),
                ComparisonOp::Gt => field_value > s.as_str(),
                ComparisonOp::GtEq => field_value >= s.as_str(),
            }
        }
        _ => false, // String fields must be compared with strings
    }
}

/// Evaluates a timestamp condition.
fn evaluate_timestamp_condition(condition: &Condition, log: &LogEntry) -> bool {
    match &condition.value {
        Value::String(s) => {
            // Try to parse as ISO 8601 timestamp
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(s) {
                let parsed_utc = parsed.with_timezone(&chrono::Utc);
                match condition.operator {
                    ComparisonOp::Eq => log.timestamp == parsed_utc,
                    ComparisonOp::NotEq => log.timestamp != parsed_utc,
                    ComparisonOp::Lt => log.timestamp < parsed_utc,
                    ComparisonOp::LtEq => log.timestamp <= parsed_utc,
                    ComparisonOp::Gt => log.timestamp > parsed_utc,
                    ComparisonOp::GtEq => log.timestamp >= parsed_utc,
                    _ => false,
                }
            } else {
                false
            }
        }
        Value::Integer(epoch_secs) => {
            if let Some(parsed) = chrono::DateTime::from_timestamp(*epoch_secs, 0) {
                match condition.operator {
                    ComparisonOp::Eq => log.timestamp == parsed,
                    ComparisonOp::NotEq => log.timestamp != parsed,
                    ComparisonOp::Lt => log.timestamp < parsed,
                    ComparisonOp::LtEq => log.timestamp <= parsed,
                    ComparisonOp::Gt => log.timestamp > parsed,
                    ComparisonOp::GtEq => log.timestamp >= parsed,
                    _ => false,
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Evaluates an attribute condition.
fn evaluate_attribute_condition(attr_value: &serde_json::Value, condition: &Condition) -> bool {
    match (&condition.value, attr_value) {
        (Value::String(query_val), serde_json::Value::String(attr_str)) => {
            let attr_lower = attr_str.to_lowercase();
            let query_lower = query_val.to_lowercase();

            match condition.operator {
                ComparisonOp::Eq => attr_lower == query_lower,
                ComparisonOp::NotEq => attr_lower != query_lower,
                ComparisonOp::Contains => attr_lower.contains(&query_lower),
                ComparisonOp::StartsWith => attr_lower.starts_with(&query_lower),
                ComparisonOp::EndsWith => attr_lower.ends_with(&query_lower),
                _ => attr_str.as_str().cmp(query_val.as_str()) == std::cmp::Ordering::Equal,
            }
        }
        (Value::Integer(query_val), serde_json::Value::Number(attr_num)) => {
            if let Some(attr_int) = attr_num.as_i64() {
                match condition.operator {
                    ComparisonOp::Eq => attr_int == *query_val,
                    ComparisonOp::NotEq => attr_int != *query_val,
                    ComparisonOp::Lt => attr_int < *query_val,
                    ComparisonOp::LtEq => attr_int <= *query_val,
                    ComparisonOp::Gt => attr_int > *query_val,
                    ComparisonOp::GtEq => attr_int >= *query_val,
                    _ => false,
                }
            } else {
                false
            }
        }
        (Value::Float(query_val), serde_json::Value::Number(attr_num)) => {
            if let Some(attr_float) = attr_num.as_f64() {
                match condition.operator {
                    ComparisonOp::Eq => (attr_float - query_val).abs() < f64::EPSILON,
                    ComparisonOp::NotEq => (attr_float - query_val).abs() >= f64::EPSILON,
                    ComparisonOp::Lt => attr_float < *query_val,
                    ComparisonOp::LtEq => attr_float <= *query_val,
                    ComparisonOp::Gt => attr_float > *query_val,
                    ComparisonOp::GtEq => attr_float >= *query_val,
                    _ => false,
                }
            } else {
                false
            }
        }
        (Value::Boolean(query_val), serde_json::Value::Bool(attr_bool)) => {
            match condition.operator {
                ComparisonOp::Eq => attr_bool == query_val,
                ComparisonOp::NotEq => attr_bool != query_val,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Sorts logs by the specified field and order.
fn sort_logs(logs: &mut [LogEntry], order_by: &OrderBy) {
    let field = order_by.field.to_lowercase();

    logs.sort_by(|a, b| {
        let cmp = match field.as_str() {
            "timestamp" => a.timestamp.cmp(&b.timestamp),
            "level" => level_order(a.level).cmp(&level_order(b.level)),
            "service" => a.service.cmp(&b.service),
            "message" => a.message.cmp(&b.message),
            _ => std::cmp::Ordering::Equal,
        };

        match order_by.order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryLogStore;

    fn create_test_store() -> InMemoryLogStore {
        let store = InMemoryLogStore::new();

        store
            .insert(LogEntry::new(LogLevel::Info, "Info message", "api"))
            .unwrap();
        store
            .insert(LogEntry::new(LogLevel::Error, "Error occurred", "api"))
            .unwrap();
        store
            .insert(LogEntry::new(
                LogLevel::Debug,
                "Debug message",
                "auth-service",
            ))
            .unwrap();
        store
            .insert(LogEntry::new(
                LogLevel::Error,
                "Database connection failed",
                "db-service",
            ))
            .unwrap();
        store
            .insert(LogEntry::new(LogLevel::Warn, "High memory usage", "api"))
            .unwrap();

        store
    }

    #[test]
    fn test_execute_simple_select() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs").unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.total_count, 5);
        assert_eq!(result.logs.len(), 5);
    }

    #[test]
    fn test_execute_where_level_eq() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs WHERE level = 'error'").unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result.logs.iter().all(|l| l.level == LogLevel::Error));
    }

    #[test]
    fn test_execute_where_service_eq() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs WHERE service = 'api'").unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.total_count, 3);
        assert!(result.logs.iter().all(|l| l.service == "api"));
    }

    #[test]
    fn test_execute_where_message_contains() {
        let store = create_test_store();
        let query =
            super::super::parse_query("SELECT * FROM logs WHERE message CONTAINS 'message'")
                .unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.total_count, 2); // "Info message" and "Debug message"
    }

    #[test]
    fn test_execute_where_and() {
        let store = create_test_store();
        let query = super::super::parse_query(
            "SELECT * FROM logs WHERE level = 'error' AND service = 'api'",
        )
        .unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.logs[0].message, "Error occurred");
    }

    #[test]
    fn test_execute_where_or() {
        let store = create_test_store();
        let query =
            super::super::parse_query("SELECT * FROM logs WHERE level = 'error' OR level = 'warn'")
                .unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.total_count, 3); // 2 errors + 1 warn
    }

    #[test]
    fn test_execute_where_grouped() {
        let store = create_test_store();
        let query = super::super::parse_query(
            "SELECT * FROM logs WHERE (level = 'error' OR level = 'warn') AND service = 'api'",
        )
        .unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.total_count, 2); // "Error occurred" and "High memory usage"
    }

    #[test]
    fn test_execute_order_by_timestamp_desc() {
        let store = create_test_store();
        let query =
            super::super::parse_query("SELECT * FROM logs ORDER BY timestamp DESC").unwrap();

        let result = execute_query(&query, &store).unwrap();

        // Check that timestamps are in descending order
        for i in 1..result.logs.len() {
            assert!(result.logs[i - 1].timestamp >= result.logs[i].timestamp);
        }
    }

    #[test]
    fn test_execute_order_by_timestamp_asc() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs ORDER BY timestamp ASC").unwrap();

        let result = execute_query(&query, &store).unwrap();

        // Check that timestamps are in ascending order
        for i in 1..result.logs.len() {
            assert!(result.logs[i - 1].timestamp <= result.logs[i].timestamp);
        }
    }

    #[test]
    fn test_execute_order_by_level() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs ORDER BY level DESC").unwrap();

        let result = execute_query(&query, &store).unwrap();

        // Check that levels are in descending severity order
        let level_orders: Vec<u8> = result.logs.iter().map(|l| level_order(l.level)).collect();

        for i in 1..level_orders.len() {
            assert!(level_orders[i - 1] >= level_orders[i]);
        }
    }

    #[test]
    fn test_execute_limit() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs LIMIT 2").unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.logs.len(), 2);
        assert_eq!(result.total_count, 5);
    }

    #[test]
    fn test_execute_offset() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs LIMIT 2 OFFSET 2").unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.logs.len(), 2);
        assert_eq!(result.total_count, 5);
    }

    #[test]
    fn test_execute_full_query() {
        let store = create_test_store();
        let query = super::super::parse_query(
            "SELECT * FROM logs WHERE service = 'api' ORDER BY timestamp DESC LIMIT 2",
        )
        .unwrap();

        let result = execute_query(&query, &store).unwrap();

        assert_eq!(result.logs.len(), 2);
        assert_eq!(result.total_count, 3); // 3 api logs total
        assert!(result.logs.iter().all(|l| l.service == "api"));
    }

    #[test]
    fn test_execute_unsupported_source() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM metrics").unwrap();

        let result = execute_query(&query, &store);

        assert!(matches!(result, Err(ExecutionError::UnsupportedSource(_))));
    }

    #[test]
    fn test_execute_level_comparison() {
        let store = create_test_store();
        let query = super::super::parse_query("SELECT * FROM logs WHERE level >= 'warn'").unwrap();

        let result = execute_query(&query, &store).unwrap();

        // Should include warn, error, fatal
        assert_eq!(result.total_count, 3); // 1 warn + 2 errors
    }

    #[test]
    fn test_execute_with_attributes() {
        let store = InMemoryLogStore::new();

        let log = LogEntry::new(LogLevel::Info, "Test", "api")
            .with_attribute("user_id", "123")
            .with_attribute("count", 42);
        store.insert(log).unwrap();

        let query = super::super::parse_query("SELECT * FROM logs WHERE user_id = '123'").unwrap();
        let result = execute_query(&query, &store).unwrap();
        assert_eq!(result.total_count, 1);

        let query = super::super::parse_query("SELECT * FROM logs WHERE count = 42").unwrap();
        let result = execute_query(&query, &store).unwrap();
        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn test_execute_case_insensitive() {
        let store = create_test_store();

        // Level should be case-insensitive
        let query = super::super::parse_query("SELECT * FROM logs WHERE level = 'ERROR'").unwrap();
        let result = execute_query(&query, &store).unwrap();
        assert_eq!(result.total_count, 2);

        // Service should be case-insensitive
        let query = super::super::parse_query("SELECT * FROM logs WHERE service = 'API'").unwrap();
        let result = execute_query(&query, &store).unwrap();
        assert_eq!(result.total_count, 3);
    }
}
