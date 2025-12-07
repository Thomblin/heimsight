//! API route definitions.
//!
//! This module organizes all HTTP routes for the Heimsight API server.

use crate::state::AppState;
use axum::Router;

mod health;
mod logs;

pub use health::health_routes;

/// Creates log routes with the given application state.
pub fn logs_routes(state: AppState) -> Router {
    logs::logs_routes(state)
}
