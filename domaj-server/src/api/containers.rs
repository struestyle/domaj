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

        let same_tag_update = container_checks
            .iter()
            .any(|c| c.check_type == "same_tag" && c.has_update);

        let latest_check = container_checks
            .iter()
            .find(|c| c.check_type == "latest" && c.has_update);

        let latest_update = latest_check.is_some();
        let latest_tag = latest_check.and_then(|c| c.latest_tag.clone());

        let last_checked = container_checks
            .iter()
            .map(|c| c.checked_at)
            .max();

        if same_tag_update || latest_update {
            updates.push(UpdateSummary {
                container_id: container.id,
                container_name: container.name,
                image: container.image,
                server_name: server_map.get(&container.server_id).cloned().unwrap_or_default(),
                same_tag_update,
                latest_update,
                latest_tag,
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
