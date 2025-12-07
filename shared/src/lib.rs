//! Heimsight Shared Library
//!
//! This crate contains shared types, models, and utilities used across
//! the Heimsight observability platform.
//!
//! # Modules
//!
//! - `models` - Data models for logs, metrics, and traces (coming soon)
//! - `storage` - Storage traits and implementations (coming soon)
//! - `query` - Query parsing and execution (coming soon)

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

/// Re-export common dependencies for convenience.
pub use chrono;
pub use serde;
pub use serde_json;
pub use validator;

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test to verify the crate compiles
        assert!(true);
    }
}
