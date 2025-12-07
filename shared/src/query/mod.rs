//! SQL-like query language for Heimsight.
//!
//! This module provides a parser and executor for SQL-like queries that can be used to
//! query logs, metrics, and traces.
//!
//! # Supported Syntax
//!
//! ```sql
//! SELECT * FROM logs WHERE level = 'error' AND service = 'api'
//! SELECT * FROM logs WHERE message CONTAINS 'failed' LIMIT 100
//! SELECT * FROM logs WHERE level = 'error' ORDER BY timestamp DESC LIMIT 50
//! ```
//!
//! # Example
//!
//! ```
//! use shared::query::{parse_query, Query, Source};
//!
//! let query = parse_query("SELECT * FROM logs WHERE level = 'error' LIMIT 10").unwrap();
//! assert_eq!(query.source, Source::Logs);
//! assert_eq!(query.limit, Some(10));
//! ```

mod ast;
mod executor;
mod parser;

pub use ast::*;
pub use executor::{execute_query, ExecutionError};
pub use parser::{parse_query, ParseError};
