//! Application state module.
//!
//! Defines the shared application state that is passed to route handlers.

use shared::storage::{InMemoryLogStore, LogStore};
use std::sync::Arc;

/// Application state shared across all request handlers.
///
/// This struct contains all the shared resources needed by the API,
/// such as storage backends and configuration.
#[derive(Clone)]
pub struct AppState {
    /// The log storage backend.
    log_store: Arc<dyn LogStore>,
}

impl AppState {
    /// Creates a new application state with the given log store.
    pub fn new(log_store: Arc<dyn LogStore>) -> Self {
        Self { log_store }
    }

    /// Creates a new application state with an in-memory log store.
    ///
    /// This is useful for development and testing.
    #[must_use]
    pub fn with_in_memory_store() -> Self {
        Self {
            log_store: Arc::new(InMemoryLogStore::new()),
        }
    }

    /// Returns a reference to the log store.
    #[must_use]
    pub fn log_store(&self) -> &dyn LogStore {
        self.log_store.as_ref()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::with_in_memory_store()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::{LogEntry, LogLevel};

    #[test]
    fn test_app_state_with_in_memory_store() {
        let state = AppState::with_in_memory_store();

        // Verify we can use the store
        let log = LogEntry::new(LogLevel::Info, "Test", "test-service");
        state.log_store().insert(log).unwrap();

        assert_eq!(state.log_store().count().unwrap(), 1);
    }

    #[test]
    fn test_app_state_is_clone() {
        let state = AppState::with_in_memory_store();
        let state2 = state.clone();

        // Both should share the same store
        let log = LogEntry::new(LogLevel::Info, "Test", "test-service");
        state.log_store().insert(log).unwrap();

        assert_eq!(state2.log_store().count().unwrap(), 1);
    }
}
