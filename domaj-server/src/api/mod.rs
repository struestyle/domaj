//! API module for Domaj Server
//!
//! RESTful API endpoints for managing servers, containers, and updates.

pub mod auth;
mod containers;
pub mod rate_limit;
mod servers;
mod websocket;

use std::sync::Arc;
use axum::{
    middleware,
    routing::{get, post, delete, put},
    Router,
};

use crate::AppState;

/// Build the API router with all endpoints
pub fn router(jwt_secret: String) -> Router<Arc<AppState>> {
    // Public auth routes (no authentication required)
    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login));
    
    // Protected routes (require JWT authentication)
    let protected_routes = Router::new()
        // Auth - get current user
        .route("/auth/me", get(auth::me))
        
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
        .route("/containers/:id/update", post(containers::update_container))
        
        // Updates overview
        .route("/updates", get(containers::list_updates))
        
        // Actions
        .route("/scan", post(containers::trigger_scan))
        
        // System status
        .route("/status", get(status))
        
        // WebSocket for real-time updates
        .route("/ws", get(websocket::ws_handler))
        .layer(middleware::from_fn_with_state(jwt_secret, auth::auth_middleware));
    
    // Combine public and protected routes
    auth_routes.merge(protected_routes)
}

/// System status endpoint
async fn status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

