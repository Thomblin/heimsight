//! Configuration module for Heimsight.
//!
//! This module contains configuration structures for retention policies and other settings.

pub mod retention;

pub use retention::{DataType, RetentionConfig, RetentionPolicy};
