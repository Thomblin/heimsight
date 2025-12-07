//! Data models for the Heimsight observability platform.
//!
//! This module contains the core data structures for logs, metrics, and traces.

pub mod log;

pub use log::{LogEntry, LogLevel, LogValidationError};
