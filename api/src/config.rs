//! Server configuration module.
//!
//! Handles loading configuration from environment variables with sensible defaults.

use anyhow::Result;
use std::net::SocketAddr;

/// Server configuration.
///
/// Configuration values can be set via environment variables:
/// - `HEIMSIGHT_HOST`: The host address to bind to (default: "0.0.0.0")
/// - `HEIMSIGHT_PORT`: The port to listen on (default: 8080)
#[derive(Debug, Clone)]
pub struct Config {
    /// The host address to bind to.
    pub host: String,
    /// The port to listen on.
    pub port: u16,
}

impl Config {
    /// Creates a new configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `HEIMSIGHT_PORT` is set but cannot be parsed as a valid port number
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("HEIMSIGHT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = std::env::var("HEIMSIGHT_PORT")
            .ok()
            .map(|p| p.parse::<u16>())
            .transpose()?
            .unwrap_or(8080);

        Ok(Self { host, port })
    }

    /// Returns the socket address for binding.
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}
