//! Server configuration module.
//!
//! Handles loading configuration from environment variables with sensible defaults.

use anyhow::Result;
use std::net::SocketAddr;

/// Server configuration.
///
/// Configuration values can be set via environment variables:
/// - `HEIMSIGHT_HOST`: The host address to bind to (default: "0.0.0.0")
/// - `HEIMSIGHT_PORT`: The HTTP port to listen on (default: 8080)
/// - `HEIMSIGHT_GRPC_PORT`: The gRPC port to listen on (default: 4317)
#[derive(Debug, Clone)]
pub struct Config {
    /// The host address to bind to.
    pub host: String,
    /// The HTTP port to listen on.
    pub port: u16,
    /// The gRPC port to listen on.
    pub grpc_port: u16,
}

impl Config {
    /// Creates a new configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `HEIMSIGHT_PORT` is set but cannot be parsed as a valid port number
    /// - `HEIMSIGHT_GRPC_PORT` is set but cannot be parsed as a valid port number
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("HEIMSIGHT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = std::env::var("HEIMSIGHT_PORT")
            .ok()
            .map(|p| p.parse::<u16>())
            .transpose()?
            .unwrap_or(8080);

        let grpc_port = std::env::var("HEIMSIGHT_GRPC_PORT")
            .ok()
            .map(|p| p.parse::<u16>())
            .transpose()?
            .unwrap_or(4317);

        Ok(Self {
            host,
            port,
            grpc_port,
        })
    }

    /// Returns the HTTP socket address for binding.
    ///
    /// # Panics
    ///
    /// Panics if the host and port combination cannot be parsed as a valid socket address.
    #[must_use]
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid socket address from config")
    }

    /// Returns the gRPC socket address for binding.
    ///
    /// # Panics
    ///
    /// Panics if the host and gRPC port combination cannot be parsed as a valid socket address.
    #[must_use]
    pub fn grpc_socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.grpc_port)
            .parse()
            .expect("Invalid gRPC socket address from config")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            grpc_port: 4317,
        }
    }
}
