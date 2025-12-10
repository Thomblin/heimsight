//! Configuration module for Heimsight.
//!
//! This module contains configuration structures for retention policies and other settings.

pub mod aggregation;
pub mod retention;

pub use aggregation::{AggregationConfig, AggregationInterval, AggregationPolicy};
pub use retention::{DataType, RetentionConfig, RetentionPolicy};
