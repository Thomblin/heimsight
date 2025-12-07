//! Abstract Syntax Tree definitions for the query language.

use serde::{Deserialize, Serialize};

/// The data source to query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// Query logs.
    Logs,
    /// Query metrics.
    Metrics,
    /// Query traces.
    Traces,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Logs => write!(f, "logs"),
            Self::Metrics => write!(f, "metrics"),
            Self::Traces => write!(f, "traces"),
        }
    }
}

/// Comparison operators for WHERE clauses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    /// Equal (=)
    Eq,
    /// Not equal (!=, <>)
    NotEq,
    /// Less than (<)
    Lt,
    /// Less than or equal (<=)
    LtEq,
    /// Greater than (>)
    Gt,
    /// Greater than or equal (>=)
    GtEq,
    /// Contains (case-insensitive substring match)
    Contains,
    /// Starts with
    StartsWith,
    /// Ends with
    EndsWith,
}

impl std::fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eq => write!(f, "="),
            Self::NotEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::LtEq => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::GtEq => write!(f, ">="),
            Self::Contains => write!(f, "CONTAINS"),
            Self::StartsWith => write!(f, "STARTS WITH"),
            Self::EndsWith => write!(f, "ENDS WITH"),
        }
    }
}

/// Logical operators for combining conditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalOp {
    /// Logical AND
    And,
    /// Logical OR
    Or,
}

impl std::fmt::Display for LogicalOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::And => write!(f, "AND"),
            Self::Or => write!(f, "OR"),
        }
    }
}

/// A value in the query (string, number, or boolean).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// String value (e.g., 'error', "api-service")
    String(String),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Boolean(bool),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "'{s}'"),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Float(fl) => write!(f, "{fl}"),
            Self::Boolean(b) => write!(f, "{b}"),
        }
    }
}

/// A single comparison condition (e.g., level = 'error').
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    /// The field name to compare.
    pub field: String,
    /// The comparison operator.
    pub operator: ComparisonOp,
    /// The value to compare against.
    pub value: Value,
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.field, self.operator, self.value)
    }
}

/// A WHERE clause expression (can be a single condition or combined with AND/OR).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WhereClause {
    /// A single condition.
    Condition(Condition),
    /// Two clauses combined with a logical operator.
    Combined {
        /// Left-hand side clause.
        left: Box<WhereClause>,
        /// The logical operator.
        operator: LogicalOp,
        /// Right-hand side clause.
        right: Box<WhereClause>,
    },
    /// A grouped expression (parentheses).
    Grouped(Box<WhereClause>),
}

impl std::fmt::Display for WhereClause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Condition(c) => write!(f, "{c}"),
            Self::Combined {
                left,
                operator,
                right,
            } => write!(f, "{left} {operator} {right}"),
            Self::Grouped(inner) => write!(f, "({inner})"),
        }
    }
}

/// Sort order for ORDER BY clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortOrder {
    /// Ascending order (oldest first for timestamps).
    Asc,
    /// Descending order (newest first for timestamps).
    #[default]
    Desc,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asc => write!(f, "ASC"),
            Self::Desc => write!(f, "DESC"),
        }
    }
}

/// ORDER BY clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBy {
    /// The field to sort by.
    pub field: String,
    /// The sort order.
    pub order: SortOrder,
}

impl std::fmt::Display for OrderBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.field, self.order)
    }
}

/// A parsed SQL-like query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    /// The data source to query (logs, metrics, traces).
    pub source: Source,
    /// Optional WHERE clause with conditions.
    pub where_clause: Option<WhereClause>,
    /// Optional ORDER BY clause.
    pub order_by: Option<OrderBy>,
    /// Optional LIMIT clause.
    pub limit: Option<usize>,
    /// Optional OFFSET clause.
    pub offset: Option<usize>,
}

impl Query {
    /// Creates a new query for the given source with no filters.
    #[must_use]
    pub fn new(source: Source) -> Self {
        Self {
            source,
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        }
    }

    /// Sets the WHERE clause.
    #[must_use]
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.where_clause = Some(clause);
        self
    }

    /// Sets the ORDER BY clause.
    #[must_use]
    pub fn with_order_by(mut self, field: impl Into<String>, order: SortOrder) -> Self {
        self.order_by = Some(OrderBy {
            field: field.into(),
            order,
        });
        self
    }

    /// Sets the LIMIT.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the OFFSET.
    #[must_use]
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT * FROM {}", self.source)?;

        if let Some(ref where_clause) = self.where_clause {
            write!(f, " WHERE {where_clause}")?;
        }

        if let Some(ref order_by) = self.order_by {
            write!(f, " ORDER BY {order_by}")?;
        }

        if let Some(limit) = self.limit {
            write!(f, " LIMIT {limit}")?;
        }

        if let Some(offset) = self.offset {
            write!(f, " OFFSET {offset}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_display_simple() {
        let query = Query::new(Source::Logs);
        assert_eq!(query.to_string(), "SELECT * FROM logs");
    }

    #[test]
    fn test_query_display_with_where() {
        let query = Query::new(Source::Logs).with_where(WhereClause::Condition(Condition {
            field: "level".to_string(),
            operator: ComparisonOp::Eq,
            value: Value::String("error".to_string()),
        }));
        assert_eq!(
            query.to_string(),
            "SELECT * FROM logs WHERE level = 'error'"
        );
    }

    #[test]
    fn test_query_display_full() {
        let query = Query::new(Source::Logs)
            .with_where(WhereClause::Condition(Condition {
                field: "level".to_string(),
                operator: ComparisonOp::Eq,
                value: Value::String("error".to_string()),
            }))
            .with_order_by("timestamp", SortOrder::Desc)
            .with_limit(100)
            .with_offset(10);

        assert_eq!(
            query.to_string(),
            "SELECT * FROM logs WHERE level = 'error' ORDER BY timestamp DESC LIMIT 100 OFFSET 10"
        );
    }

    #[test]
    fn test_condition_display() {
        let condition = Condition {
            field: "service".to_string(),
            operator: ComparisonOp::Contains,
            value: Value::String("api".to_string()),
        };
        assert_eq!(condition.to_string(), "service CONTAINS 'api'");
    }

    #[test]
    fn test_combined_where_clause() {
        let clause = WhereClause::Combined {
            left: Box::new(WhereClause::Condition(Condition {
                field: "level".to_string(),
                operator: ComparisonOp::Eq,
                value: Value::String("error".to_string()),
            })),
            operator: LogicalOp::And,
            right: Box::new(WhereClause::Condition(Condition {
                field: "service".to_string(),
                operator: ComparisonOp::Eq,
                value: Value::String("api".to_string()),
            })),
        };
        assert_eq!(clause.to_string(), "level = 'error' AND service = 'api'");
    }
}
