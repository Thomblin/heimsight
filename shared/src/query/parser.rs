//! SQL-like query parser using nom.
//!
//! Parses queries like:
//! - `SELECT * FROM logs`
//! - `SELECT * FROM logs WHERE level = 'error'`
//! - `SELECT * FROM logs WHERE level = 'error' AND service = 'api'`
//! - `SELECT * FROM logs WHERE message CONTAINS 'failed' LIMIT 100`

use super::ast::{
    ComparisonOp, Condition, LogicalOp, OrderBy, Query, SortOrder, Source, Value, WhereClause,
};
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, tag_no_case, take_while1},
    character::complete::{char, digit1, multispace0, multispace1, none_of},
    combinator::{map, map_res, opt, recognize, value},
    multi::many0,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use thiserror::Error;

/// Errors that can occur during query parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The query syntax is invalid.
    #[error("Invalid query syntax: {0}")]
    SyntaxError(String),

    /// An unexpected token was encountered.
    #[error("Unexpected token: expected {expected}, found '{found}'")]
    UnexpectedToken {
        /// What was expected.
        expected: String,
        /// What was found.
        found: String,
    },

    /// The query is empty.
    #[error("Empty query")]
    EmptyQuery,

    /// Unknown data source.
    #[error("Unknown data source: '{0}'. Expected 'logs', 'metrics', or 'traces'")]
    UnknownSource(String),
}

/// Parses a SQL-like query string into a Query AST.
///
/// # Arguments
///
/// * `input` - The query string to parse.
///
/// # Returns
///
/// Returns a parsed `Query` on success, or a `ParseError` on failure.
///
/// # Errors
///
/// Returns a `ParseError` if:
/// - The query is empty
/// - The syntax is invalid
/// - There is unexpected trailing content
///
/// # Examples
///
/// ```
/// use shared::query::{parse_query, Source};
///
/// let query = parse_query("SELECT * FROM logs WHERE level = 'error'").unwrap();
/// assert_eq!(query.source, Source::Logs);
/// ```
pub fn parse_query(input: &str) -> Result<Query, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::EmptyQuery);
    }

    match query(input) {
        Ok((remaining, query)) => {
            let remaining = remaining.trim();
            if remaining.is_empty() {
                Ok(query)
            } else {
                Err(ParseError::SyntaxError(format!(
                    "Unexpected trailing content: '{remaining}'"
                )))
            }
        }
        Err(e) => Err(ParseError::SyntaxError(format!("{e}"))),
    }
}

// ============================================================================
// Main query parser
// ============================================================================

fn query(input: &str) -> IResult<&str, Query> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag_no_case("SELECT")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = char('*')(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("FROM")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, source) = source(input)?;
    let (input, _) = multispace0(input)?;

    let (input, where_clause) = opt(where_clause)(input)?;
    let (input, _) = multispace0(input)?;

    let (input, order_by) = opt(order_by)(input)?;
    let (input, _) = multispace0(input)?;

    let (input, limit) = opt(limit_clause)(input)?;
    let (input, _) = multispace0(input)?;

    let (input, offset) = opt(offset_clause)(input)?;
    let (input, _) = multispace0(input)?;

    Ok((
        input,
        Query {
            source,
            where_clause,
            order_by,
            limit,
            offset,
        },
    ))
}

// ============================================================================
// Source parser
// ============================================================================

fn source(input: &str) -> IResult<&str, Source> {
    alt((
        value(Source::Logs, tag_no_case("logs")),
        value(Source::Metrics, tag_no_case("metrics")),
        value(Source::Traces, tag_no_case("traces")),
    ))(input)
}

// ============================================================================
// WHERE clause parser
// ============================================================================

fn where_clause(input: &str) -> IResult<&str, WhereClause> {
    let (input, _) = tag_no_case("WHERE")(input)?;
    let (input, _) = multispace1(input)?;
    where_expression(input)
}

fn where_expression(input: &str) -> IResult<&str, WhereClause> {
    or_expression(input)
}

fn or_expression(input: &str) -> IResult<&str, WhereClause> {
    let (input, first) = and_expression(input)?;
    let (input, rest) = many0(preceded(
        tuple((multispace1, tag_no_case("OR"), multispace1)),
        and_expression,
    ))(input)?;

    let result = rest
        .into_iter()
        .fold(first, |left, right| WhereClause::Combined {
            left: Box::new(left),
            operator: LogicalOp::Or,
            right: Box::new(right),
        });

    Ok((input, result))
}

fn and_expression(input: &str) -> IResult<&str, WhereClause> {
    let (input, first) = primary_condition(input)?;
    let (input, rest) = many0(preceded(
        tuple((multispace1, tag_no_case("AND"), multispace1)),
        primary_condition,
    ))(input)?;

    let result = rest
        .into_iter()
        .fold(first, |left, right| WhereClause::Combined {
            left: Box::new(left),
            operator: LogicalOp::And,
            right: Box::new(right),
        });

    Ok((input, result))
}

fn primary_condition(input: &str) -> IResult<&str, WhereClause> {
    alt((grouped_condition, map(condition, WhereClause::Condition)))(input)
}

fn grouped_condition(input: &str) -> IResult<&str, WhereClause> {
    let (input, _) = char('(')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, expr) = where_expression(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;

    Ok((input, WhereClause::Grouped(Box::new(expr))))
}

fn condition(input: &str) -> IResult<&str, Condition> {
    let (input, field) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, operator) = comparison_op(input)?;
    let (input, _) = multispace0(input)?;
    let (input, value) = query_value(input)?;

    Ok((
        input,
        Condition {
            field: field.to_string(),
            operator,
            value,
        },
    ))
}

// ============================================================================
// Comparison operators
// ============================================================================

fn comparison_op(input: &str) -> IResult<&str, ComparisonOp> {
    alt((
        value(ComparisonOp::NotEq, alt((tag("!="), tag("<>")))),
        value(ComparisonOp::LtEq, tag("<=")),
        value(ComparisonOp::GtEq, tag(">=")),
        value(ComparisonOp::Eq, char('=')),
        value(ComparisonOp::Lt, char('<')),
        value(ComparisonOp::Gt, char('>')),
        value(ComparisonOp::Contains, tag_no_case("CONTAINS")),
        value(
            ComparisonOp::StartsWith,
            tuple((tag_no_case("STARTS"), multispace1, tag_no_case("WITH"))),
        ),
        value(
            ComparisonOp::EndsWith,
            tuple((tag_no_case("ENDS"), multispace1, tag_no_case("WITH"))),
        ),
    ))(input)
}

// ============================================================================
// Value parsers
// ============================================================================

fn query_value(input: &str) -> IResult<&str, Value> {
    alt((boolean_value, float_value, integer_value, string_value))(input)
}

fn string_value(input: &str) -> IResult<&str, Value> {
    alt((single_quoted_string, double_quoted_string))(input)
}

fn single_quoted_string(input: &str) -> IResult<&str, Value> {
    let (input, s) = delimited(
        char('\''),
        alt((escaped(none_of("'\\"), '\\', char('\'')), tag(""))),
        char('\''),
    )(input)?;
    Ok((input, Value::String(s.to_string())))
}

fn double_quoted_string(input: &str) -> IResult<&str, Value> {
    let (input, s) = delimited(
        char('"'),
        alt((escaped(none_of("\"\\"), '\\', char('"')), tag(""))),
        char('"'),
    )(input)?;
    Ok((input, Value::String(s.to_string())))
}

fn integer_value(input: &str) -> IResult<&str, Value> {
    let (input, num) = map_res(recognize(pair(opt(char('-')), digit1)), |s: &str| {
        s.parse::<i64>()
    })(input)?;
    Ok((input, Value::Integer(num)))
}

fn float_value(input: &str) -> IResult<&str, Value> {
    let (input, num) = map_res(
        recognize(tuple((opt(char('-')), digit1, char('.'), digit1))),
        |s: &str| s.parse::<f64>(),
    )(input)?;
    Ok((input, Value::Float(num)))
}

fn boolean_value(input: &str) -> IResult<&str, Value> {
    alt((
        value(Value::Boolean(true), tag_no_case("true")),
        value(Value::Boolean(false), tag_no_case("false")),
    ))(input)
}

// ============================================================================
// ORDER BY clause
// ============================================================================

fn order_by(input: &str) -> IResult<&str, OrderBy> {
    let (input, _) = tag_no_case("ORDER")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag_no_case("BY")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, field) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, order) = opt(sort_order)(input)?;

    Ok((
        input,
        OrderBy {
            field: field.to_string(),
            order: order.unwrap_or_default(),
        },
    ))
}

fn sort_order(input: &str) -> IResult<&str, SortOrder> {
    alt((
        value(SortOrder::Asc, tag_no_case("ASC")),
        value(SortOrder::Desc, tag_no_case("DESC")),
    ))(input)
}

// ============================================================================
// LIMIT and OFFSET clauses
// ============================================================================

fn limit_clause(input: &str) -> IResult<&str, usize> {
    let (input, _) = tag_no_case("LIMIT")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, n) = map_res(digit1, |s: &str| s.parse::<usize>())(input)?;
    Ok((input, n))
}

fn offset_clause(input: &str) -> IResult<&str, usize> {
    let (input, _) = tag_no_case("OFFSET")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, n) = map_res(digit1, |s: &str| s.parse::<usize>())(input)?;
    Ok((input, n))
}

// ============================================================================
// Identifier parser
// ============================================================================

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let query = parse_query("SELECT * FROM logs").unwrap();
        assert_eq!(query.source, Source::Logs);
        assert!(query.where_clause.is_none());
        assert!(query.order_by.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_parse_select_from_metrics() {
        let query = parse_query("SELECT * FROM metrics").unwrap();
        assert_eq!(query.source, Source::Metrics);
    }

    #[test]
    fn test_parse_select_from_traces() {
        let query = parse_query("SELECT * FROM traces").unwrap();
        assert_eq!(query.source, Source::Traces);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let query = parse_query("select * from LOGS").unwrap();
        assert_eq!(query.source, Source::Logs);

        let query = parse_query("Select * From Logs").unwrap();
        assert_eq!(query.source, Source::Logs);
    }

    #[test]
    fn test_parse_where_string_eq() {
        let query = parse_query("SELECT * FROM logs WHERE level = 'error'").unwrap();
        assert_eq!(query.source, Source::Logs);

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.field, "level");
                assert_eq!(c.operator, ComparisonOp::Eq);
                assert_eq!(c.value, Value::String("error".to_string()));
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_where_double_quoted_string() {
        let query = parse_query("SELECT * FROM logs WHERE service = \"api-gateway\"").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.field, "service");
                assert_eq!(c.value, Value::String("api-gateway".to_string()));
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_where_integer() {
        let query = parse_query("SELECT * FROM logs WHERE status_code = 200").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.field, "status_code");
                assert_eq!(c.value, Value::Integer(200));
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_where_negative_integer() {
        let query = parse_query("SELECT * FROM metrics WHERE value = -42").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.value, Value::Integer(-42));
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_where_float() {
        let query = parse_query("SELECT * FROM metrics WHERE value = 3.5").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.value, Value::Float(3.5));
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_where_boolean() {
        let query = parse_query("SELECT * FROM logs WHERE success = true").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.value, Value::Boolean(true));
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_comparison_operators() {
        let operators = vec![
            ("=", ComparisonOp::Eq),
            ("!=", ComparisonOp::NotEq),
            ("<>", ComparisonOp::NotEq),
            ("<", ComparisonOp::Lt),
            ("<=", ComparisonOp::LtEq),
            (">", ComparisonOp::Gt),
            (">=", ComparisonOp::GtEq),
        ];

        for (op_str, expected_op) in operators {
            let query_str = format!("SELECT * FROM logs WHERE count {op_str} 10");
            let query = parse_query(&query_str).unwrap();

            match query.where_clause {
                Some(WhereClause::Condition(c)) => {
                    assert_eq!(c.operator, expected_op, "Failed for operator {op_str}");
                }
                _ => panic!("Expected single condition for operator {op_str}"),
            }
        }
    }

    #[test]
    fn test_parse_contains_operator() {
        let query = parse_query("SELECT * FROM logs WHERE message CONTAINS 'error'").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.operator, ComparisonOp::Contains);
                assert_eq!(c.value, Value::String("error".to_string()));
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_starts_with_operator() {
        let query = parse_query("SELECT * FROM logs WHERE message STARTS WITH 'Error:'").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.operator, ComparisonOp::StartsWith);
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_ends_with_operator() {
        let query = parse_query("SELECT * FROM logs WHERE message ENDS WITH 'failed'").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.operator, ComparisonOp::EndsWith);
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_parse_where_and() {
        let query =
            parse_query("SELECT * FROM logs WHERE level = 'error' AND service = 'api'").unwrap();

        match query.where_clause {
            Some(WhereClause::Combined {
                left,
                operator,
                right,
            }) => {
                assert_eq!(operator, LogicalOp::And);
                match (*left, *right) {
                    (WhereClause::Condition(l), WhereClause::Condition(r)) => {
                        assert_eq!(l.field, "level");
                        assert_eq!(r.field, "service");
                    }
                    _ => panic!("Expected two conditions"),
                }
            }
            _ => panic!("Expected combined clause"),
        }
    }

    #[test]
    fn test_parse_where_or() {
        let query =
            parse_query("SELECT * FROM logs WHERE level = 'error' OR level = 'fatal'").unwrap();

        match query.where_clause {
            Some(WhereClause::Combined { operator, .. }) => {
                assert_eq!(operator, LogicalOp::Or);
            }
            _ => panic!("Expected combined clause"),
        }
    }

    #[test]
    fn test_parse_where_multiple_and() {
        let query = parse_query(
            "SELECT * FROM logs WHERE level = 'error' AND service = 'api' AND status = 500",
        )
        .unwrap();

        // Should create left-associative tree: ((level AND service) AND status)
        assert!(query.where_clause.is_some());
    }

    #[test]
    fn test_parse_where_and_or_precedence() {
        // AND should have higher precedence than OR
        let query = parse_query("SELECT * FROM logs WHERE a = 1 OR b = 2 AND c = 3").unwrap();

        // Should parse as: a = 1 OR (b = 2 AND c = 3)
        match query.where_clause {
            Some(WhereClause::Combined { operator, .. }) => {
                assert_eq!(operator, LogicalOp::Or);
            }
            _ => panic!("Expected OR at top level"),
        }
    }

    #[test]
    fn test_parse_where_grouped() {
        let query = parse_query(
            "SELECT * FROM logs WHERE (level = 'error' OR level = 'fatal') AND service = 'api'",
        )
        .unwrap();

        match query.where_clause {
            Some(WhereClause::Combined {
                left,
                operator,
                right,
            }) => {
                assert_eq!(operator, LogicalOp::And);
                assert!(matches!(*left, WhereClause::Grouped(_)));
                assert!(matches!(*right, WhereClause::Condition(_)));
            }
            _ => panic!("Expected combined clause"),
        }
    }

    #[test]
    fn test_parse_order_by() {
        let query = parse_query("SELECT * FROM logs ORDER BY timestamp DESC").unwrap();

        match query.order_by {
            Some(ob) => {
                assert_eq!(ob.field, "timestamp");
                assert_eq!(ob.order, SortOrder::Desc);
            }
            None => panic!("Expected ORDER BY"),
        }
    }

    #[test]
    fn test_parse_order_by_asc() {
        let query = parse_query("SELECT * FROM logs ORDER BY timestamp ASC").unwrap();

        match query.order_by {
            Some(ob) => {
                assert_eq!(ob.order, SortOrder::Asc);
            }
            None => panic!("Expected ORDER BY"),
        }
    }

    #[test]
    fn test_parse_order_by_default_desc() {
        let query = parse_query("SELECT * FROM logs ORDER BY timestamp").unwrap();

        match query.order_by {
            Some(ob) => {
                assert_eq!(ob.order, SortOrder::Desc); // Default is DESC
            }
            None => panic!("Expected ORDER BY"),
        }
    }

    #[test]
    fn test_parse_limit() {
        let query = parse_query("SELECT * FROM logs LIMIT 100").unwrap();
        assert_eq!(query.limit, Some(100));
    }

    #[test]
    fn test_parse_offset() {
        let query = parse_query("SELECT * FROM logs LIMIT 100 OFFSET 50").unwrap();
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(50));
    }

    #[test]
    fn test_parse_full_query() {
        let query = parse_query(
            "SELECT * FROM logs WHERE level = 'error' AND service = 'api' ORDER BY timestamp DESC LIMIT 100 OFFSET 10"
        ).unwrap();

        assert_eq!(query.source, Source::Logs);
        assert!(query.where_clause.is_some());
        assert!(query.order_by.is_some());
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn test_parse_with_extra_whitespace() {
        let query = parse_query("  SELECT  *  FROM  logs  WHERE  level  =  'error'  ").unwrap();
        assert_eq!(query.source, Source::Logs);
        assert!(query.where_clause.is_some());
    }

    #[test]
    fn test_parse_empty_query() {
        let result = parse_query("");
        assert!(matches!(result, Err(ParseError::EmptyQuery)));
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_query("   ");
        assert!(matches!(result, Err(ParseError::EmptyQuery)));
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let result = parse_query("SELECT FROM logs");
        assert!(matches!(result, Err(ParseError::SyntaxError(_))));
    }

    #[test]
    fn test_parse_trailing_content() {
        let result = parse_query("SELECT * FROM logs WHERE level = 'error' INVALID");
        assert!(matches!(result, Err(ParseError::SyntaxError(_))));
    }

    #[test]
    fn test_parse_empty_string_value() {
        let query = parse_query("SELECT * FROM logs WHERE message = ''").unwrap();

        match query.where_clause {
            Some(WhereClause::Condition(c)) => {
                assert_eq!(c.value, Value::String(String::new()));
            }
            _ => panic!("Expected single condition"),
        }
    }
}
