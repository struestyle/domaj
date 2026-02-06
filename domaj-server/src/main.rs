//! Domaj Server - Docker Maintenance System
//!
//! Central server for monitoring Docker containers across multiple servers
//! and detecting available updates.

mod api;
mod config;
mod db;
mod notifier;
mod registry;
mod scheduler;

use std::sync::Arc;
use axum::Router;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::scheduler::Scheduler;

use crate::api::rate_limit::RateLimiter;

/// Application state shared across all handlers
pub struct AppState {
    pub db: SqlitePool,
    pub config: Config,
    pub scheduler: Arc<RwLock<Scheduler>>,
    pub rate_limiter: Arc<RateLimiter>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "domaj_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting Domaj Server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("📧 Email notifications: {}", if config.smtp_host.is_some() { "enabled" } else { "disabled" });
    tracing::info!("⏰ Scan interval: {}", config.scan_interval);

    // Initialize database
    let db = db::init_db(&config.database_url).await?;
    tracing::info!("💾 Database initialized");

    // Initialize scheduler
    let scheduler = Arc::new(RwLock::new(Scheduler::new()));

    // Initialize rate limiter
    let rate_limiter = api::rate_limit::create_rate_limiter();

    // Create app state
    let state = Arc::new(AppState {
        db: db.clone(),
        config: config.clone(),
        scheduler: scheduler.clone(),
        rate_limiter,
    });

    // Start the scheduler
    {
        let mut sched = scheduler.write().await;
        sched.start(state.clone()).await?;
    }
    tracing::info!("📅 Scheduler started");

    // Build router
    let app = Router::new()
        .nest("/api", api::router(config.jwt_secret.clone()))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🌐 Server listening on http://{}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
