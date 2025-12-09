//! Metrics collection module for internal observability.
//!
//! This module provides functionality for collecting and exposing metrics about
//! the Heimsight system itself, including data age statistics.

pub mod data_age;

pub use data_age::{DataAgeMetrics, DataAgeMonitor};
