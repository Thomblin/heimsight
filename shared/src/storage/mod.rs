//! Storage traits and implementations.
//!
//! This module provides abstractions for storing and querying observability data.
//! The `LogStore` trait defines the interface for log storage, allowing different
//! implementations (in-memory, database-backed, etc.).

pub mod log_store;

pub use log_store::{InMemoryLogStore, LogQuery, LogQueryResult, LogStore, LogStoreError};
