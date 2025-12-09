//! Retention configuration for data expiration policies.
//!
//! This module defines structures for configuring data retention (TTL) policies
//! for different data types (logs, metrics, traces).

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Represents different types of observability data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    /// Log entries
    Logs,
    /// Metrics (counters, gauges, histograms)
    Metrics,
    /// Distributed traces (spans)
    Traces,
}

/// Retention policy for a specific data type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// The data type this policy applies to.
    pub data_type: DataType,
    /// Time-to-live (TTL) duration in days.
    pub ttl_days: u32,
}

impl RetentionPolicy {
    /// Creates a new retention policy.
    ///
    /// # Arguments
    ///
    /// * `data_type` - The type of data this policy applies to
    /// * `ttl_days` - Number of days to retain the data
    ///
    /// # Examples
    ///
    /// ```
    /// use shared::config::{DataType, RetentionPolicy};
    ///
    /// let policy = RetentionPolicy::new(DataType::Logs, 30);
    /// assert_eq!(policy.ttl_days, 30);
    /// ```
    #[must_use]
    pub fn new(data_type: DataType, ttl_days: u32) -> Self {
        Self {
            data_type,
            ttl_days,
        }
    }

    /// Returns the TTL as a `Duration`.
    ///
    /// # Examples
    ///
    /// ```
    /// use shared::config::{DataType, RetentionPolicy};
    ///
    /// let policy = RetentionPolicy::new(DataType::Logs, 30);
    /// let duration = policy.as_duration();
    /// assert_eq!(duration.as_secs(), 30 * 24 * 60 * 60);
    /// ```
    #[must_use]
    pub fn as_duration(&self) -> Duration {
        Duration::from_secs(u64::from(self.ttl_days) * 24 * 60 * 60)
    }

    /// Validates the retention policy.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - TTL is zero
    /// - TTL exceeds maximum allowed (3650 days / 10 years)
    pub fn validate(&self) -> Result<(), String> {
        if self.ttl_days == 0 {
            return Err("TTL must be greater than zero".to_string());
        }
        if self.ttl_days > 3650 {
            return Err("TTL cannot exceed 3650 days (10 years)".to_string());
        }
        Ok(())
    }
}

/// Complete retention configuration for all data types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Retention policy for logs.
    pub logs: RetentionPolicy,
    /// Retention policy for metrics.
    pub metrics: RetentionPolicy,
    /// Retention policy for traces.
    pub traces: RetentionPolicy,
}

impl RetentionConfig {
    /// Creates a new retention configuration.
    ///
    /// # Arguments
    ///
    /// * `logs_ttl_days` - TTL for logs in days
    /// * `metrics_ttl_days` - TTL for metrics in days
    /// * `traces_ttl_days` - TTL for traces in days
    ///
    /// # Examples
    ///
    /// ```
    /// use shared::config::RetentionConfig;
    ///
    /// let config = RetentionConfig::new(30, 90, 30);
    /// assert_eq!(config.logs.ttl_days, 30);
    /// assert_eq!(config.metrics.ttl_days, 90);
    /// assert_eq!(config.traces.ttl_days, 30);
    /// ```
    #[must_use]
    pub fn new(logs_ttl_days: u32, metrics_ttl_days: u32, traces_ttl_days: u32) -> Self {
        Self {
            logs: RetentionPolicy::new(DataType::Logs, logs_ttl_days),
            metrics: RetentionPolicy::new(DataType::Metrics, metrics_ttl_days),
            traces: RetentionPolicy::new(DataType::Traces, traces_ttl_days),
        }
    }

    /// Validates all retention policies.
    ///
    /// # Errors
    ///
    /// Returns an error if any policy is invalid.
    pub fn validate(&self) -> Result<(), String> {
        self.logs.validate()?;
        self.metrics.validate()?;
        self.traces.validate()?;
        Ok(())
    }

    /// Gets the retention policy for a specific data type.
    ///
    /// # Examples
    ///
    /// ```
    /// use shared::config::{DataType, RetentionConfig};
    ///
    /// let config = RetentionConfig::default();
    /// let policy = config.get_policy(DataType::Logs);
    /// assert_eq!(policy.ttl_days, 30);
    /// ```
    #[must_use]
    pub fn get_policy(&self, data_type: DataType) -> &RetentionPolicy {
        match data_type {
            DataType::Logs => &self.logs,
            DataType::Metrics => &self.metrics,
            DataType::Traces => &self.traces,
        }
    }

    /// Updates the retention policy for a specific data type.
    ///
    /// # Arguments
    ///
    /// * `data_type` - The data type to update
    /// * `ttl_days` - New TTL in days
    ///
    /// # Examples
    ///
    /// ```
    /// use shared::config::{DataType, RetentionConfig};
    ///
    /// let mut config = RetentionConfig::default();
    /// config.update_policy(DataType::Logs, 60);
    /// assert_eq!(config.logs.ttl_days, 60);
    /// ```
    pub fn update_policy(&mut self, data_type: DataType, ttl_days: u32) {
        match data_type {
            DataType::Logs => self.logs.ttl_days = ttl_days,
            DataType::Metrics => self.metrics.ttl_days = ttl_days,
            DataType::Traces => self.traces.ttl_days = ttl_days,
        }
    }
}

impl Default for RetentionConfig {
    /// Returns default retention configuration:
    /// - Logs: 30 days
    /// - Metrics: 90 days
    /// - Traces: 30 days
    fn default() -> Self {
        Self::new(30, 90, 30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_policy_new() {
        let policy = RetentionPolicy::new(DataType::Logs, 30);
        assert_eq!(policy.data_type, DataType::Logs);
        assert_eq!(policy.ttl_days, 30);
    }

    #[test]
    fn test_retention_policy_as_duration() {
        let policy = RetentionPolicy::new(DataType::Logs, 30);
        let duration = policy.as_duration();
        assert_eq!(duration.as_secs(), 30 * 24 * 60 * 60);
    }

    #[test]
    fn test_retention_policy_validate_valid() {
        let policy = RetentionPolicy::new(DataType::Logs, 30);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_retention_policy_validate_zero_ttl() {
        let policy = RetentionPolicy::new(DataType::Logs, 0);
        let result = policy.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "TTL must be greater than zero");
    }

    #[test]
    fn test_retention_policy_validate_exceeds_max() {
        let policy = RetentionPolicy::new(DataType::Logs, 3651);
        let result = policy.validate();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "TTL cannot exceed 3650 days (10 years)"
        );
    }

    #[test]
    fn test_retention_config_new() {
        let config = RetentionConfig::new(30, 90, 30);
        assert_eq!(config.logs.ttl_days, 30);
        assert_eq!(config.metrics.ttl_days, 90);
        assert_eq!(config.traces.ttl_days, 30);
    }

    #[test]
    fn test_retention_config_default() {
        let config = RetentionConfig::default();
        assert_eq!(config.logs.ttl_days, 30);
        assert_eq!(config.metrics.ttl_days, 90);
        assert_eq!(config.traces.ttl_days, 30);
    }

    #[test]
    fn test_retention_config_validate_valid() {
        let config = RetentionConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_retention_config_validate_invalid_logs() {
        let config = RetentionConfig::new(0, 90, 30);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("TTL must be greater than zero"));
    }

    #[test]
    fn test_retention_config_get_policy() {
        let config = RetentionConfig::default();
        let logs_policy = config.get_policy(DataType::Logs);
        assert_eq!(logs_policy.ttl_days, 30);

        let metrics_policy = config.get_policy(DataType::Metrics);
        assert_eq!(metrics_policy.ttl_days, 90);

        let traces_policy = config.get_policy(DataType::Traces);
        assert_eq!(traces_policy.ttl_days, 30);
    }

    #[test]
    fn test_retention_config_update_policy() {
        let mut config = RetentionConfig::default();
        config.update_policy(DataType::Logs, 60);
        assert_eq!(config.logs.ttl_days, 60);

        config.update_policy(DataType::Metrics, 180);
        assert_eq!(config.metrics.ttl_days, 180);

        config.update_policy(DataType::Traces, 45);
        assert_eq!(config.traces.ttl_days, 45);
    }

    #[test]
    fn test_retention_policy_serialization() {
        let policy = RetentionPolicy::new(DataType::Logs, 30);
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: RetentionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_retention_config_serialization() {
        let config = RetentionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RetentionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_data_type_serialization() {
        let data_type = DataType::Logs;
        let json = serde_json::to_string(&data_type).unwrap();
        assert_eq!(json, "\"logs\"");

        let deserialized: DataType = serde_json::from_str(&json).unwrap();
        assert_eq!(data_type, deserialized);
    }
}
