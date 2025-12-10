//! Data aggregation configuration for long-term storage optimization.
//!
//! This module defines configuration for downsampling observability data
//! to reduce storage costs while maintaining queryability over longer time periods.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Aggregation time interval for downsampling data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationInterval {
    /// 1 minute aggregation (raw data → 1min)
    OneMinute,
    /// 5 minute aggregation
    FiveMinutes,
    /// 1 hour aggregation
    OneHour,
    /// 1 day aggregation
    OneDay,
}

impl AggregationInterval {
    /// Returns the duration of this interval.
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        match self {
            Self::OneMinute => Duration::from_secs(60),
            Self::FiveMinutes => Duration::from_secs(300),
            Self::OneHour => Duration::from_secs(3600),
            Self::OneDay => Duration::from_secs(86400),
        }
    }

    /// Returns the interval in seconds.
    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.as_duration().as_secs()
    }

    /// Returns a human-readable string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::OneMinute => "1 minute",
            Self::FiveMinutes => "5 minutes",
            Self::OneHour => "1 hour",
            Self::OneDay => "1 day",
        }
    }
}

impl std::fmt::Display for AggregationInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Aggregation policy for a specific data type and interval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregationPolicy {
    /// The aggregation interval
    pub interval: AggregationInterval,
    /// Retention period in days for this aggregation level
    pub retention_days: u32,
    /// Whether this aggregation level is enabled
    pub enabled: bool,
}

impl AggregationPolicy {
    /// Creates a new aggregation policy.
    #[must_use]
    pub const fn new(interval: AggregationInterval, retention_days: u32, enabled: bool) -> Self {
        Self {
            interval,
            retention_days,
            enabled,
        }
    }

    /// Validates the aggregation policy.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Retention is zero
    /// - Retention exceeds maximum (3650 days / 10 years)
    pub fn validate(&self) -> Result<(), String> {
        if self.retention_days == 0 {
            return Err("Aggregation retention must be greater than zero".to_string());
        }
        if self.retention_days > 3650 {
            return Err("Aggregation retention cannot exceed 3650 days (10 years)".to_string());
        }
        Ok(())
    }
}

/// Complete aggregation configuration for all observability data.
///
/// Defines multi-tier aggregation strategies for:
/// - **Metrics**: Raw data → 1-minute → 5-minute → 1-hour → 1-day aggregates
/// - **Logs**: Raw logs → 1-hour counts → 1-day counts (by level, service, message pattern)
/// - **Traces/Spans**: Raw spans → 1-hour stats → 1-day stats (latency, throughput, errors)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// Whether aggregation is enabled globally
    pub enabled: bool,

    // Metric aggregation policies
    /// 1-minute aggregation policy (metrics only)
    pub one_minute: AggregationPolicy,
    /// 5-minute aggregation policy (metrics only)
    pub five_minutes: AggregationPolicy,

    // Universal aggregation policies (metrics, logs, traces)
    /// 1-hour aggregation policy
    pub one_hour: AggregationPolicy,
    /// 1-day aggregation policy
    pub one_day: AggregationPolicy,
}

impl AggregationConfig {
    /// Creates a new aggregation configuration with recommended defaults.
    ///
    /// Default strategy:
    /// - **Metrics**:
    ///   - 1-minute aggregates: 30 days retention
    ///   - 5-minute aggregates: 90 days retention
    ///   - 1-hour aggregates: 365 days retention
    ///   - 1-day aggregates: 730 days (2 years) retention
    /// - **Logs**:
    ///   - 1-hour counts: 365 days retention (by level, service, message pattern)
    ///   - 1-day counts: 730 days retention
    /// - **Traces/Spans**:
    ///   - 1-hour stats: 365 days retention (latency percentiles, throughput, errors)
    ///   - 1-day stats: 730 days retention
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: false, // Disabled by default
            one_minute: AggregationPolicy::new(AggregationInterval::OneMinute, 30, true),
            five_minutes: AggregationPolicy::new(AggregationInterval::FiveMinutes, 90, true),
            one_hour: AggregationPolicy::new(AggregationInterval::OneHour, 365, true),
            one_day: AggregationPolicy::new(AggregationInterval::OneDay, 730, true),
        }
    }

    /// Validates all aggregation policies.
    ///
    /// # Errors
    ///
    /// Returns an error if any policy is invalid.
    pub fn validate(&self) -> Result<(), String> {
        self.one_minute.validate()?;
        self.five_minutes.validate()?;
        self.one_hour.validate()?;
        self.one_day.validate()?;
        Ok(())
    }

    /// Gets the policy for a specific interval.
    #[must_use]
    pub const fn get_policy(&self, interval: AggregationInterval) -> &AggregationPolicy {
        match interval {
            AggregationInterval::OneMinute => &self.one_minute,
            AggregationInterval::FiveMinutes => &self.five_minutes,
            AggregationInterval::OneHour => &self.one_hour,
            AggregationInterval::OneDay => &self.one_day,
        }
    }

    /// Updates the policy for a specific interval.
    pub fn update_policy(
        &mut self,
        interval: AggregationInterval,
        retention_days: u32,
        enabled: bool,
    ) {
        let policy = match interval {
            AggregationInterval::OneMinute => &mut self.one_minute,
            AggregationInterval::FiveMinutes => &mut self.five_minutes,
            AggregationInterval::OneHour => &mut self.one_hour,
            AggregationInterval::OneDay => &mut self.one_day,
        };
        policy.retention_days = retention_days;
        policy.enabled = enabled;
    }
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregation_interval_duration() {
        assert_eq!(AggregationInterval::OneMinute.as_secs(), 60);
        assert_eq!(AggregationInterval::FiveMinutes.as_secs(), 300);
        assert_eq!(AggregationInterval::OneHour.as_secs(), 3600);
        assert_eq!(AggregationInterval::OneDay.as_secs(), 86400);
    }

    #[test]
    fn test_aggregation_interval_display() {
        assert_eq!(AggregationInterval::OneMinute.to_string(), "1 minute");
        assert_eq!(AggregationInterval::FiveMinutes.to_string(), "5 minutes");
        assert_eq!(AggregationInterval::OneHour.to_string(), "1 hour");
        assert_eq!(AggregationInterval::OneDay.to_string(), "1 day");
    }

    #[test]
    fn test_aggregation_policy_new() {
        let policy = AggregationPolicy::new(AggregationInterval::OneHour, 365, true);
        assert_eq!(policy.interval, AggregationInterval::OneHour);
        assert_eq!(policy.retention_days, 365);
        assert!(policy.enabled);
    }

    #[test]
    fn test_aggregation_policy_validate_valid() {
        let policy = AggregationPolicy::new(AggregationInterval::OneHour, 365, true);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_aggregation_policy_validate_zero_retention() {
        let policy = AggregationPolicy::new(AggregationInterval::OneHour, 0, true);
        let result = policy.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("greater than zero"));
    }

    #[test]
    fn test_aggregation_policy_validate_exceeds_max() {
        let policy = AggregationPolicy::new(AggregationInterval::OneHour, 3651, true);
        let result = policy.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("10 years"));
    }

    #[test]
    fn test_aggregation_config_default() {
        let config = AggregationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.one_minute.retention_days, 30);
        assert_eq!(config.five_minutes.retention_days, 90);
        assert_eq!(config.one_hour.retention_days, 365);
        assert_eq!(config.one_day.retention_days, 730);
    }

    #[test]
    fn test_aggregation_config_validate_valid() {
        let config = AggregationConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_aggregation_config_validate_invalid_policy() {
        let mut config = AggregationConfig::default();
        config.one_hour.retention_days = 0;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregation_config_get_policy() {
        let config = AggregationConfig::default();
        assert_eq!(
            config
                .get_policy(AggregationInterval::OneHour)
                .retention_days,
            365
        );
        assert_eq!(
            config
                .get_policy(AggregationInterval::OneDay)
                .retention_days,
            730
        );
    }

    #[test]
    fn test_aggregation_config_update_policy() {
        let mut config = AggregationConfig::default();
        config.update_policy(AggregationInterval::OneHour, 500, false);

        assert_eq!(config.one_hour.retention_days, 500);
        assert!(!config.one_hour.enabled);
    }

    #[test]
    fn test_aggregation_policy_serialization() {
        let policy = AggregationPolicy::new(AggregationInterval::OneHour, 365, true);
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: AggregationPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_aggregation_config_serialization() {
        let config = AggregationConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AggregationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
