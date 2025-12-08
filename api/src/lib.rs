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
pub mod db;
pub mod grpc;
mod routes;
mod state;

pub use config::Config;
pub use state::AppState;

use anyhow::Result;
use axum::Router;
use shared::otlp::proto;
use tokio::net::TcpListener;
use tonic::transport::Server;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

/// Runs the Heimsight API server.
///
/// This function initializes the server with configuration from environment variables
/// and starts listening for incoming connections. It handles graceful shutdown on
/// SIGTERM/SIGINT signals.
///
/// The server will attempt to connect to ClickHouse using the database configuration.
/// If the connection succeeds, it will use persistent ClickHouse-backed stores.
/// If the connection fails, it will fall back to in-memory stores with a warning.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration cannot be loaded from environment
/// - The server fails to bind to the configured address
/// - A fatal error occurs during operation
pub async fn run_server() -> Result<()> {
    let config = Config::from_env()?;
    
    // Try to initialize database connection
    let state = match db::DatabaseConfig::from_env() {
        Ok(db_config) => {
            let database = db::Database::new(&db_config);
            match database.ping().await {
                Ok(()) => {
                    tracing::info!("Successfully connected to ClickHouse database");
                    let client = database.client();
                    AppState::with_clickhouse_store(client)
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to connect to ClickHouse, falling back to in-memory storage. \
                        Data will not persist across restarts."
                    );
                    AppState::with_in_memory_store()
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to load database configuration, using in-memory storage. \
                Data will not persist across restarts."
            );
            AppState::with_in_memory_store()
        }
    };
    
    run_server_with_config_and_state(config, state).await
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
    let state = AppState::with_in_memory_store();
    run_server_with_config_and_state(config, state).await
}

/// Runs the Heimsight API server with the provided configuration and state.
///
/// This is useful for testing or when you want to provide configuration programmatically.
///
/// # Errors
///
/// Returns an error if:
/// - The server fails to bind to the configured address
/// - A fatal error occurs during operation
pub async fn run_server_with_config_and_state(config: Config, state: AppState) -> Result<()> {
    let http_addr = config.socket_addr();
    let grpc_addr = config.grpc_socket_addr();

    tracing::info!(
        host = %config.host,
        http_port = %config.port,
        grpc_port = %config.grpc_port,
        "Heimsight API server starting"
    );

    // Create HTTP server
    let app = create_router(state.clone());
    let listener = TcpListener::bind(http_addr).await?;

    tracing::info!(%http_addr, "HTTP server listening");

    // Create gRPC services
    let logs_service = proto::collector::logs::v1::logs_service_server::LogsServiceServer::new(
        grpc::LogsServiceImpl::new(state.clone()),
    );
    let metrics_service =
        proto::collector::metrics::v1::metrics_service_server::MetricsServiceServer::new(
            grpc::MetricsServiceImpl::new(state.clone()),
        );
    let traces_service = proto::collector::trace::v1::trace_service_server::TraceServiceServer::new(
        grpc::TracesServiceImpl::new(state),
    );

    // Build gRPC server
    let grpc_server = Server::builder()
        .add_service(logs_service)
        .add_service(metrics_service)
        .add_service(traces_service)
        .serve_with_shutdown(grpc_addr, shutdown_signal());

    tracing::info!(%grpc_addr, "gRPC server listening");

    // Run both servers concurrently
    let http_server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    tokio::try_join!(
        async move {
            http_server
                .await
                .map_err(|e| anyhow::anyhow!("HTTP server error: {e}"))
        },
        async move {
            grpc_server
                .await
                .map_err(|e| anyhow::anyhow!("gRPC server error: {e}"))
        }
    )?;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Maximum request body size (10 MB).
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Creates the main application router with all routes and middleware.
///
/// This function is public to allow testing the router without starting a full server.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(routes::health_routes())
        .merge(routes::logs_routes(state.clone()))
        .merge(routes::query_routes(state.clone()))
        .merge(routes::metrics_routes(state.clone()))
        .merge(routes::traces_routes(state.clone()))
        .merge(routes::otlp_routes(state))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
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

    fn create_test_router() -> Router {
        create_router(AppState::with_in_memory_store())
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_200() {
        let app = create_test_router();

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
        let app = create_test_router();

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
        assert_eq!(config.grpc_port, 4317);
    }

    #[test]
    fn test_config_socket_addr() {
        let config = Config {
            host: "127.0.0.1".to_string(),
            port: 3000,
            grpc_port: 4317,
        };
        let addr = config.socket_addr();
        assert_eq!(addr.to_string(), "127.0.0.1:3000");
    }

    #[test]
    fn test_config_grpc_socket_addr() {
        let config = Config {
            host: "127.0.0.1".to_string(),
            port: 3000,
            grpc_port: 9090,
        };
        let addr = config.grpc_socket_addr();
        assert_eq!(addr.to_string(), "127.0.0.1:9090");
    }
}
