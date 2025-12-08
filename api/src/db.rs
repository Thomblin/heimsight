// ! Database connection module for `ClickHouse`.
//!
//! This module provides connection pooling and configuration for `ClickHouse` database.
//! It supports creating client instances from environment variables and provides
//! a convenient way to manage database connections throughout the application.

use anyhow::{Context, Result};
use clickhouse::Client;
use std::sync::Arc;

/// Database configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// `ClickHouse` database URL (e.g., <http://localhost:8123>)
    pub url: String,
    /// Database name to use
    pub database: String,
    /// Username for authentication
    pub user: String,
    /// Password for authentication
    pub password: String,
}

impl DatabaseConfig {
    /// Load database configuration from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `HEIMSIGHT_DB_URL`: Database URL (default: <http://localhost:8123>)
    /// - `HEIMSIGHT_DB_NAME`: Database name (default: "heimsight")
    /// - `HEIMSIGHT_DB_USER`: Database user (default: "heimsight")
    /// - `HEIMSIGHT_DB_PASSWORD`: Database password (default: "`heimsight_dev`")
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables cannot be read.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            url: std::env::var("HEIMSIGHT_DB_URL")
                .unwrap_or_else(|_| "http://localhost:8123".to_string()),
            database: std::env::var("HEIMSIGHT_DB_NAME")
                .unwrap_or_else(|_| "heimsight".to_string()),
            user: std::env::var("HEIMSIGHT_DB_USER").unwrap_or_else(|_| "heimsight".to_string()),
            password: std::env::var("HEIMSIGHT_DB_PASSWORD")
                .unwrap_or_else(|_| "heimsight_dev".to_string()),
        })
    }
}

/// Database client wrapper providing connection pooling.
#[derive(Clone)]
pub struct Database {
    client: Arc<Client>,
}

impl Database {
    /// Create a new database client from configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Database configuration
    ///
    /// # Returns
    ///
    /// A new Database instance with configured client.
    ///
    /// # Examples
    ///
    /// ```
    /// # use api::db::{Database, DatabaseConfig};
    /// # fn example() -> anyhow::Result<()> {
    /// let config = DatabaseConfig::from_env()?;
    /// let db = Database::new(&config);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(config: &DatabaseConfig) -> Self {
        let client = Client::default()
            .with_url(&config.url)
            .with_database(&config.database)
            .with_user(&config.user)
            .with_password(&config.password);

        Self {
            client: Arc::new(client),
        }
    }

    /// Get a reference to the underlying `ClickHouse` client.
    ///
    /// # Returns
    ///
    /// An Arc-wrapped `ClickHouse` client.
    #[must_use]
    pub fn client(&self) -> Arc<Client> {
        Arc::clone(&self.client)
    }

    /// Test database connectivity by executing a simple query.
    ///
    /// # Returns
    ///
    /// Ok(()) if the connection is successful, or an error describing the failure.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be reached or the query fails.
    pub async fn ping(&self) -> Result<()> {
        self.client
            .query("SELECT 1")
            .fetch_one::<u8>()
            .await
            .context("Failed to ping database")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_from_env_with_defaults() {
        // Clear any existing env vars
        std::env::remove_var("HEIMSIGHT_DB_URL");
        std::env::remove_var("HEIMSIGHT_DB_NAME");
        std::env::remove_var("HEIMSIGHT_DB_USER");
        std::env::remove_var("HEIMSIGHT_DB_PASSWORD");

        let config = DatabaseConfig::from_env().expect("Failed to load config");

        assert_eq!(config.url, "http://localhost:8123");
        assert_eq!(config.database, "heimsight");
        assert_eq!(config.user, "heimsight");
        assert_eq!(config.password, "heimsight_dev");
    }

    #[test]
    fn test_database_config_with_custom_values() {
        // Create config directly to avoid env var conflicts with other tests
        let config = DatabaseConfig {
            url: "http://custom:8123".to_string(),
            database: "test_db".to_string(),
            user: "test_user".to_string(),
            password: "test_pass".to_string(),
        };

        assert_eq!(config.url, "http://custom:8123");
        assert_eq!(config.database, "test_db");
        assert_eq!(config.user, "test_user");
        assert_eq!(config.password, "test_pass");
    }

    #[test]
    fn test_database_creation() {
        let config = DatabaseConfig {
            url: "http://localhost:8123".to_string(),
            database: "heimsight".to_string(),
            user: "heimsight".to_string(),
            password: "heimsight_dev".to_string(),
        };

        let _db = Database::new(&config);
        // If we get here without panicking, the database was created successfully
    }

    #[tokio::test]
    async fn test_database_ping() {
        // This test requires a running ClickHouse instance
        let config = DatabaseConfig::from_env().expect("Failed to load config");
        let db = Database::new(&config);

        let result = db.ping().await;
        assert!(
            result.is_ok(),
            "Database ping failed. Make sure ClickHouse is running via docker-compose"
        );
    }
}
