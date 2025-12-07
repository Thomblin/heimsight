//! Integration tests for Heimsight API.
//!
//! These tests verify the complete flow of ingesting and querying
//! logs, metrics, and traces through the HTTP API.
//!
//! Tests are organized into separate modules:
//! - `logs_tests` - Log ingestion and querying
//! - `query_tests` - SQL-like query functionality
//! - `metrics_tests` - Metrics ingestion and aggregation
//! - `traces_tests` - Trace ingestion and querying
//! - `health_tests` - Health check and general API functionality

mod integration_tests {
    pub mod common;
    pub mod health_tests;
    pub mod logs_tests;
    pub mod metrics_tests;
    pub mod query_tests;
    pub mod traces_tests;
}
