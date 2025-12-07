//! Heimsight Shared Library
//!
//! This crate contains shared types, models, and utilities used across
//! the Heimsight observability platform.
//!
//! # Modules
//!
//! - [`models`] - Data models for logs, metrics, and traces
//! - [`storage`] - Storage traits and implementations
//! - [`query`] - SQL-like query parsing and execution
//!
//! # Example
//!
//! ```
//! use shared::models::{LogEntry, LogLevel};
//!
//! let log = LogEntry::new(LogLevel::Info, "User logged in", "auth-service")
//!     .with_attribute("user_id", "12345")
//!     .with_trace_id("trace-abc");
//!
//! assert!(log.validate_entry().is_ok());
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod models;
pub mod query;
pub mod storage;

/// Re-export common dependencies for convenience.
pub use chrono;
pub use serde;
pub use serde_json;
pub use validator;
