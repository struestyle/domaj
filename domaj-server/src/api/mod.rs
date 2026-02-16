//! API module for Domaj Server
//!
//! RESTful API endpoints for managing servers, containers, and updates.

pub mod auth;
mod containers;
pub mod registries;
mod servers;
pub mod settings;
mod websocket;

use std::sync::Arc;
use axum::{
    middleware,
    routing::{get, post, delete, put},
    Router,
};
use tower_governor::{
    governor::GovernorConfigBuilder,
    GovernorLayer,
};

use crate::AppState;

/// Build the API router with all endpoints
pub fn router(jwt_secret: String) -> Router<Arc<AppState>> {
    // Rate limiting config: 5 requests per 60 seconds per IP
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(60)
            .burst_size(5)
            .finish()
            .expect("Invalid governor config")
    );
    
    let governor_limiter = governor_conf.limiter().clone();
    
    // Spawn cleanup task for rate limiter
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            governor_limiter.retain_recent();
        }
    });
    
    // Public auth routes with rate limiting (no JWT required)
    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .layer(GovernorLayer {
            config: governor_conf,
        });
    
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
        .route("/servers/:id/health", get(servers::check_server_health))
        
        // Container management
        .route("/containers", get(containers::list_containers))
        .route("/containers/:id", get(containers::get_container))
        .route("/containers/:id/updates", get(containers::get_container_updates))
        .route("/containers/:id/update", post(containers::update_container))
        
        // Updates overview
        .route("/updates", get(containers::list_updates))
        
        // Update jobs
        .route("/update-jobs", get(containers::list_update_jobs))
        .route("/update-jobs/:id/rollback", post(containers::rollback_job))
        
        // Actions
        .route("/scan", post(containers::trigger_scan))
        
        // System status
        .route("/status", get(status))
        
        // Registries
        .route("/registries", get(registries::list_registries))
        .route("/registries/credentials", post(registries::create_credential))
        .route("/registries/credentials/:id", put(registries::update_credential))
        .route("/registries/credentials/:id", delete(registries::delete_credential))
        
        // Settings
        .route("/settings", get(settings::get_settings))
        .route("/settings/:key", put(settings::update_setting))
        .layer(middleware::from_fn_with_state(jwt_secret, auth::auth_middleware));
    
    // WebSocket route (handles its own auth via query param)
    let ws_routes = Router::new()
        .route("/ws", get(websocket::ws_handler));
    
    // Combine public, protected, and WS routes
    auth_routes.merge(protected_routes).merge(ws_routes)
}

/// System status endpoint
async fn status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}


