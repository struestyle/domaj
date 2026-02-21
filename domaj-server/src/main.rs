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
use sqlx::AnyPool;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::scheduler::Scheduler;

/// Application state shared across all handlers
pub struct AppState {
    pub db: AnyPool,
    pub config: Config,
    pub scheduler: Arc<RwLock<Scheduler>>,
    pub broadcast_tx: broadcast::Sender<String>,
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
    let db_type = if config.database_url.starts_with("postgres") { "PostgreSQL" } else { "SQLite" };
    tracing::info!("💾 Database initialized ({})", db_type);

    // Setup admin account from environment variables if configured
    setup_admin_account(&db, &config).await?;

    // Initialize scheduler
    let scheduler = Arc::new(RwLock::new(Scheduler::new()));

    // Create broadcast channel for WebSocket events
    let (broadcast_tx, _) = broadcast::channel::<String>(100);

    // Create app state
    let state = Arc::new(AppState {
        db: db.clone(),
        config: config.clone(),
        scheduler: scheduler.clone(),
        broadcast_tx,
    });

    // Start the scheduler
    {
        let mut sched = scheduler.write().await;
        sched.start(state.clone()).await?;
    }
    tracing::info!("📅 Scheduler started");

    // Build router
    let api_router = api::router(config.jwt_secret.clone());
    
    // Serve static files from "public" directory, fallback to "public/index.html" for SPA routing
    let static_files_service = ServeDir::new("public")
        .not_found_service(ServeFile::new("public/index.html"));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(static_files_service)
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

/// Setup admin account from environment variables if configured
async fn setup_admin_account(db: &AnyPool, config: &Config) -> anyhow::Result<()> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    // Check if admin credentials are configured
    let (username, password) = match (&config.admin_username, &config.admin_password) {
        (Some(u), Some(p)) => (u, p),
        _ => return Ok(()), // No admin configured, skip
    };

    // Validate credentials
    if username.len() < 3 {
        tracing::warn!("⚠️  ADMIN_USERNAME must be at least 3 characters, skipping admin setup");
        return Ok(());
    }
    if password.len() < 6 {
        tracing::warn!("⚠️  ADMIN_PASSWORD must be at least 6 characters, skipping admin setup");
        return Ok(());
    }

    // Check if admin user already exists
    let existing: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(db)
        .await?;

    if existing.is_some() {
        tracing::info!("👤 Admin account '{}' already exists", username);
        return Ok(());
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    // Create admin user
    sqlx::query(
        "INSERT INTO users (username, password_hash, role) VALUES ($1, $2, 'admin')"
    )
    .bind(username)
    .bind(&password_hash)
    .execute(db)
    .await?;

    tracing::info!("✅ Admin account '{}' created successfully", username);
    Ok(())
}
