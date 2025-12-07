//! Heimsight API Server
//!
//! This crate provides the main HTTP/gRPC server for the Heimsight observability platform.
//! It handles data ingestion (logs, metrics, traces), query execution, and serves the web UI.
//!
//! # Architecture
//!
//! The API server is built on Axum and Tokio, providing:
//! - REST API for data ingestion and querying
//! - OTLP endpoints for OpenTelemetry compatibility
//! - Web UI routes for the HTMX-based dashboard
//!
//! # Example
//!
//! ```no_run
//! use api::run_server;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     run_server().await
//! }
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

use anyhow::Result;

/// Runs the Heimsight API server.
///
/// This function initializes the server with configuration from environment variables
/// and starts listening for incoming connections.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a fatal error during operation.
#[allow(clippy::unused_async)] // Will have async operations in Step 1.2
pub async fn run_server() -> Result<()> {
    tracing::info!("Heimsight API server starting...");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_server_placeholder() {
        // Placeholder test to verify async runtime works
        let result = run_server().await;
        assert!(result.is_ok());
    }
}
