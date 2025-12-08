//! OTLP gRPC receiver services.
//!
//! Implements OpenTelemetry Protocol gRPC services for ingesting logs, metrics, and traces.
//! These services follow the OTLP specification and work with standard OpenTelemetry SDK exporters.
//!
//! # Services
//!
//! - `LogsService` - Receives logs via gRPC
//! - `MetricsService` - Receives metrics via gRPC
//! - `TracesService` - Receives traces via gRPC

mod services;

pub use services::{LogsServiceImpl, MetricsServiceImpl, TracesServiceImpl};
