//! API module for Domaj Server
//!
//! RESTful API endpoints for managing servers, containers, and updates.

mod containers;
mod servers;
mod websocket;

use std::sync::Arc;
use axum::{
    routing::{get, post, delete, put},
    Router,
};

use crate::AppState;

/// Build the API router with all endpoints
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Server management
        .route("/servers", get(servers::list_servers))
        .route("/servers", post(servers::create_server))
        .route("/servers/:id", get(servers::get_server))
        .route("/servers/:id", put(servers::update_server))
        .route("/servers/:id", delete(servers::delete_server))
        .route("/servers/:id/containers", get(servers::get_server_containers))
        .route("/servers/:id/sync", post(servers::sync_server))
        
        // Container management
        .route("/containers", get(containers::list_containers))
        .route("/containers/:id", get(containers::get_container))
        .route("/containers/:id/updates", get(containers::get_container_updates))
        
        // Updates overview
        .route("/updates", get(containers::list_updates))
        
        // Actions
        .route("/scan", post(containers::trigger_scan))
        
        // System status
        .route("/status", get(status))
        
        // WebSocket for real-time updates
        .route("/ws", get(websocket::ws_handler))
}

/// System status endpoint
async fn status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
