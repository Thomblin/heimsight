//! Heimsight API Server Binary
//!
//! Entry point for the Heimsight observability platform API server.

#![deny(unsafe_code)]

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    api::run_server().await
}
