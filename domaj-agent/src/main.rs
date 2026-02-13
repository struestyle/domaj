//! Domaj Agent - Docker Container Monitor
//!
//! Lightweight agent that runs on each server and exposes
//! Docker container information via a REST API.

mod docker;

use std::sync::Arc;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::docker::DockerClient;

/// Registry credential for private registries
#[derive(Debug, Clone)]
pub struct RegistryCredential {
    pub host: String,
    pub username: String,
    pub password: String,
}

/// Agent configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub api_key: String,
    pub agent_id: String,
    pub registry_credentials: Vec<RegistryCredential>,
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

        // Get or generate unique agent ID
        let agent_id = get_or_create_agent_id()?;

        // Parse registry credentials (REGISTRY_1_HOST, REGISTRY_1_USER, REGISTRY_1_PASSWORD, ...)
        let mut registry_credentials = Vec::new();
        for i in 1..=10 {
            let host = std::env::var(format!("REGISTRY_{}_HOST", i));
            let user = std::env::var(format!("REGISTRY_{}_USER", i));
            let pass = std::env::var(format!("REGISTRY_{}_PASSWORD", i));
            
            if let (Ok(host), Ok(username), Ok(password)) = (host, user, pass) {
                if !host.is_empty() {
                    tracing::info!("🔐 Loaded credentials for registry: {}", host);
                    registry_credentials.push(RegistryCredential { host, username, password });
                }
            }
        }

        Ok(Self { port, api_key, agent_id, registry_credentials })
    }

    /// Find credentials for a given image reference
    pub fn find_credentials_for_image(&self, image: &str) -> Option<&RegistryCredential> {
        // Extract the registry hostname from the image
        let image_part = image.split(':').next().unwrap_or(image);
        if let Some(first_segment) = image_part.split('/').next() {
            if first_segment.contains('.') || first_segment.contains(':') {
                return self.registry_credentials.iter().find(|c| c.host == first_segment);
            }
        }
        None
    }
}

/// Get existing agent ID from file or create a new one
fn get_or_create_agent_id() -> anyhow::Result<String> {
    let id_path = std::path::Path::new("/data/agent_id");
    
    // Try to read existing ID
    if id_path.exists() {
        if let Ok(id) = std::fs::read_to_string(id_path) {
            let id = id.trim().to_string();
            if !id.is_empty() {
                return Ok(id);
            }
        }
    }
    
    // Create data directory if needed
    if let Some(parent) = id_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    // Generate new UUID
    let new_id = Uuid::new_v4().to_string();
    
    // Save to file
    std::fs::write(id_path, &new_id)?;
    tracing::info!("🆔 Generated new agent ID: {}", new_id);
    
    Ok(new_id)
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
    tracing::info!("🆔 Agent ID: {}", config.agent_id);

    // Initialize Docker client
    let docker = DockerClient::new().await?;
    tracing::info!("🐳 Connected to Docker daemon");

    // Create app state
    let state = Arc::new(AppState { config: config.clone(), docker });

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/info", get(info))
        .route("/containers", get(list_containers))
        .route("/containers/:id", get(get_container))
        .route("/containers/:name/update", post(update_container))
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

/// Agent info endpoint - returns unique agent ID
async fn info(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    check_api_key(&headers, &state.config.api_key)?;
    Ok(Json(serde_json::json!({
        "agent_id": state.config.agent_id,
        "version": env!("CARGO_PKG_VERSION"),
    })))
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
/// Request body for container update
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// Optional target tag to update to
    pub target_tag: Option<String>,
    /// Optional full target image (for rollback). Takes priority over target_tag.
    pub target_image: Option<String>,
}

/// Update a container to a new image
async fn update_container(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<UpdateRequest>,
) -> Result<Json<docker::UpdateResult>, (StatusCode, Json<serde_json::Value>)> {
    check_api_key(&headers, &state.config.api_key)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;
    
    tracing::info!("🔄 Received update request for container '{}' (target_tag: {:?}, target_image: {:?})", name, body.target_tag, body.target_image);
    
    // First get the container to determine its image
    let container = state.docker.get_container(&name).await.map_err(|e| {
        tracing::error!("Failed to get container {}: {}", name, e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "error": format!("Failed to get container: {}", e)
        })))
    })?;
    
    let container = container.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "error": format!("Container '{}' not found", name)
        })))
    })?;
    
    // Determine the image to pull:
    // 1. target_image (full image ref, for rollback) takes priority
    // 2. target_tag (just a tag, for normal updates)
    // 3. fallback: re-pull current image
    let pull_image = if let Some(ref img) = body.target_image {
        img.clone()
    } else if let Some(ref tag) = body.target_tag {
        let base = container.image.split(':').next().unwrap_or(&container.image);
        format!("{}:{}", base, tag)
    } else {
        container.image.clone()
    };
    
    // Find credentials for the registry
    let credentials = state.config.find_credentials_for_image(&pull_image);
    if credentials.is_some() {
        tracing::info!("🔐 Using private registry credentials for {}", pull_image);
    }
    
    let result = state.docker
        .update_container(&name, Some(&pull_image), credentials)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update container {}: {}", name, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to update container: {}", e)
            })))
        })?;
    
    if result.success {
        Ok(Json(result))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "error": result.message,
            "old_image": result.old_image,
            "new_image": result.new_image
        }))))
    }
}
