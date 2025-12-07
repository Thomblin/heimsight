//! API route definitions.
//!
//! This module organizes all HTTP routes for the Heimsight API server.

use crate::state::AppState;
use axum::Router;

mod health;
mod logs;
mod metrics;
mod query;
mod traces;

pub use health::health_routes;

/// Creates log routes with the given application state.
pub fn logs_routes(state: AppState) -> Router {
    logs::logs_routes(state)
}

/// Creates query routes with the given application state.
pub fn query_routes(state: AppState) -> Router {
    query::query_routes(state)
}

/// Creates metrics routes with the given application state.
pub fn metrics_routes(state: AppState) -> Router {
    metrics::metrics_routes(state)
}

/// Creates traces routes with the given application state.
pub fn traces_routes(state: AppState) -> Router {
    traces::traces_routes(state)
}
