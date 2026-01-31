//! Domaj Agent - Docker Container Monitor
//!
//! Lightweight agent that runs on each server and exposes
//! Docker container information via a REST API.

mod docker;

use std::sync::Arc;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::docker::DockerClient;

/// Agent configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub api_key: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .unwrap_or(3001);

        let api_key = std::env::var("API_KEY").unwrap_or_else(|_| {
            tracing::warn!("⚠️  API_KEY not set, agent will accept any requests");
            String::new()
        });

        Ok(Self { port, api_key })
    }
}

/// Application state
pub struct AppState {
    pub config: Config,
    pub docker: DockerClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "domaj_agent=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting Domaj Agent v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::from_env()?;

    // Initialize Docker client
    let docker = DockerClient::new().await?;
    tracing::info!("🐳 Connected to Docker daemon");

    // Create app state
    let state = Arc::new(AppState { config: config.clone(), docker });

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/containers", get(list_containers))
        .route("/containers/{id}", get(get_container))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🌐 Agent listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Middleware to check API key
fn check_api_key(headers: &HeaderMap, expected: &str) -> Result<(), StatusCode> {
    if expected.is_empty() {
        return Ok(());
    }
    
    let provided = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    if provided == expected {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// List all containers
async fn list_containers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<docker::ContainerInfo>>, StatusCode> {
    check_api_key(&headers, &state.config.api_key)?;
    
    let containers = state.docker.list_containers().await.map_err(|e| {
        tracing::error!("Failed to list containers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(containers))
}

/// Get a specific container by ID
async fn get_container(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<docker::ContainerInfo>, StatusCode> {
    check_api_key(&headers, &state.config.api_key)?;
    
    let container = state.docker.get_container(&id).await.map_err(|e| {
        tracing::error!("Failed to get container {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match container {
        Some(c) => Ok(Json(c)),
        None => Err(StatusCode::NOT_FOUND),
    }
}
