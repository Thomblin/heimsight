//! API route definitions.
//!
//! This module organizes all HTTP routes for the Heimsight API server.

mod health;
mod logs;

pub use health::health_routes;
pub use logs::logs_routes;
