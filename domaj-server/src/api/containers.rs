//! Container management API endpoints

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::db::{Container, UpdateCheck, UpdateSummary};
use crate::AppState;

/// Container with server name for display
#[derive(serde::Serialize)]
pub struct ContainerWithServer {
    #[serde(flatten)]
    pub container: Container,
    pub server_name: String,
}

/// List all containers across all servers
pub async fn list_containers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ContainerWithServer>>, StatusCode> {
    // Get all containers
    let containers: Vec<Container> = sqlx::query_as(
        "SELECT * FROM containers ORDER BY server_id, name",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch containers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get all servers for mapping
    let servers: Vec<crate::db::Server> = sqlx::query_as("SELECT * FROM servers")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let server_map: std::collections::HashMap<i64, String> = servers
        .into_iter()
        .map(|s| (s.id, s.name))
        .collect();

    let result = containers
        .into_iter()
        .map(|c| {
            let server_name = server_map.get(&c.server_id).cloned().unwrap_or_default();
            ContainerWithServer {
                container: c,
                server_name,
            }
        })
        .collect();

    Ok(Json(result))
}

/// Get a specific container by ID
pub async fn get_container(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ContainerWithServer>, StatusCode> {
    let container: Container = sqlx::query_as("SELECT * FROM containers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let server: crate::db::Server = sqlx::query_as("SELECT * FROM servers WHERE id = ?")
        .bind(container.server_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ContainerWithServer {
        container,
        server_name: server.name,
    }))
}

/// Get update checks for a container
pub async fn get_container_updates(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<UpdateCheck>>, StatusCode> {
    let updates: Vec<UpdateCheck> = sqlx::query_as(
        "SELECT * FROM update_checks WHERE container_id = ? ORDER BY checked_at DESC LIMIT 10",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch updates: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(updates))
}

/// List all available updates
pub async fn list_updates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<UpdateSummary>>, StatusCode> {
    // Get all containers
    let containers: Vec<Container> = sqlx::query_as("SELECT * FROM containers")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    // Get all servers
    let servers: Vec<crate::db::Server> = sqlx::query_as("SELECT * FROM servers")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let server_map: std::collections::HashMap<i64, String> = servers
        .into_iter()
        .map(|s| (s.id, s.name))
        .collect();

    // Get all update checks
    let checks: Vec<UpdateCheck> = sqlx::query_as("SELECT * FROM update_checks")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    // Build update summaries
    let mut updates = Vec::new();
    
    for container in containers {
        let container_checks: Vec<&UpdateCheck> = checks
            .iter()
            .filter(|c| c.container_id == container.id)
            .collect();

        let same_tag_check = container_checks
            .iter()
            .find(|c| c.check_type == "same_tag" && c.has_update);

        let same_tag_update = same_tag_check.is_some();
        let same_tag_digest = same_tag_check.and_then(|c| c.remote_digest.clone());

        let latest_check = container_checks
            .iter()
            .find(|c| c.check_type == "latest" && c.has_update);

        let latest_update = latest_check.is_some();
        let latest_tag = latest_check.and_then(|c| c.latest_tag.clone());
        let latest_digest = latest_check.and_then(|c| c.remote_digest.clone());

        let last_checked = container_checks
            .iter()
            .map(|c| c.checked_at)
            .max();

        if same_tag_update || latest_update {
            updates.push(UpdateSummary {
                container_id: container.id,
                container_name: container.name.clone(),
                image: container.image.clone(),
                image_digest: container.image_digest.clone(),
                server_name: server_map.get(&container.server_id).cloned().unwrap_or_default(),
                same_tag_update,
                same_tag_digest,
                latest_update,
                latest_tag,
                latest_digest,
                last_checked,
            });
        }
    }

    Ok(Json(updates))
}

/// Trigger a manual scan
pub async fn trigger_scan(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Run the scan in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::scheduler::run_scan(&state_clone).await {
            tracing::error!("Scan failed: {}", e);
        }
    });

    Ok(Json(serde_json::json!({
        "status": "scan_started",
        "message": "Scan triggered in background"
    })))
}

/// Request body for container update
#[derive(serde::Deserialize)]
pub struct UpdateContainerRequest {
    /// Optional target tag to update to
    pub target_tag: Option<String>,
}

/// Update a container via its agent
pub async fn update_container(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateContainerRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Get container info
    let container: Container = sqlx::query_as("SELECT * FROM containers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch container: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Database error"})))
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Container not found"}))))?;

    // Get the server this container belongs to
    let server: crate::db::Server = sqlx::query_as("SELECT * FROM servers WHERE id = ?")
        .bind(container.server_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Database error"}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Server not found"}))))?;

    tracing::info!(
        "🔄 Requesting update for container {} on {} (target_tag: {:?})",
        container.name,
        server.name,
        body.target_tag
    );

    // Send update request to agent
    let client = reqwest::Client::new();
    let agent_url = format!(
        "{}/containers/{}/update",
        server.endpoint.trim_end_matches('/'),
        container.name
    );

    let response = client
        .post(&agent_url)
        .header("X-API-Key", &state.config.api_secret)
        .json(&serde_json::json!({
            "target_tag": body.target_tag
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to contact agent: {}", e);
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                "error": format!("Failed to contact agent: {}", e)
            })))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.json::<serde_json::Value>().await.unwrap_or_default();
        tracing::error!("Agent returned error: {} - {:?}", status, error_body);
        return Err((StatusCode::BAD_GATEWAY, Json(serde_json::json!({
            "error": error_body.get("error").and_then(|e| e.as_str()).unwrap_or("Agent error"),
            "details": error_body
        }))));
    }

    let result: serde_json::Value = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse agent response: {}", e);
        (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "Invalid agent response"})))
    })?;

    tracing::info!("✅ Container {} updated successfully", container.name);

    Ok(Json(result))
}
