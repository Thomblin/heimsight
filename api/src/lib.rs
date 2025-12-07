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

mod config;
mod routes;

pub use config::Config;

use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

/// Runs the Heimsight API server.
///
/// This function initializes the server with configuration from environment variables
/// and starts listening for incoming connections. It handles graceful shutdown on
/// SIGTERM/SIGINT signals.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration cannot be loaded from environment
/// - The server fails to bind to the configured address
/// - A fatal error occurs during operation
pub async fn run_server() -> Result<()> {
    let config = Config::from_env()?;
    run_server_with_config(config).await
}

/// Runs the Heimsight API server with the provided configuration.
///
/// This is useful for testing or when you want to provide configuration programmatically.
///
/// # Errors
///
/// Returns an error if:
/// - The server fails to bind to the configured address
/// - A fatal error occurs during operation
pub async fn run_server_with_config(config: Config) -> Result<()> {
    let addr = config.socket_addr();

    tracing::info!(
        host = %config.host,
        port = %config.port,
        "Heimsight API server starting"
    );

    let app = create_router();
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(%addr, "Listening for connections");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Creates the main application router with all routes and middleware.
///
/// This function is public to allow testing the router without starting a full server.
pub fn create_router() -> Router {
    Router::new()
        .merge(routes::health_routes())
        .layer(TraceLayer::new_for_http())
}

/// Waits for a shutdown signal (SIGTERM or SIGINT).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown");
        }
        () = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint_returns_200() {
        let app = create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_json() {
        let app = create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok());

        assert!(content_type.is_some_and(|ct| ct.contains("application/json")));
    }

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_config_socket_addr() {
        let config = Config {
            host: "127.0.0.1".to_string(),
            port: 3000,
        };
        let addr = config.socket_addr();
        assert_eq!(addr.to_string(), "127.0.0.1:3000");
    }
}
