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

/// Update a container via its agent (async with job tracking)
pub async fn update_container(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateContainerRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
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

    // Create job in database (store current image as previous_image for rollback)
    let job_id: i64 = sqlx::query_scalar(
        "INSERT INTO update_jobs (container_id, container_name, server_name, image, target_tag, previous_image, job_type, status) VALUES (?, ?, ?, ?, ?, ?, 'update', 'running') RETURNING id"
    )
    .bind(container.id)
    .bind(&container.name)
    .bind(&server.name)
    .bind(&container.image)
    .bind(&body.target_tag)
    .bind(&container.image)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create update job: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to create job"})))
    })?;

    // Broadcast job started
    let _ = state.broadcast_tx.send(serde_json::json!({
        "type": "job_started",
        "job": {
            "id": job_id,
            "container_name": container.name,
            "server_name": server.name,
            "image": container.image,
            "target_tag": body.target_tag,
            "status": "running"
        }
    }).to_string());

    // Spawn background task
    let state_clone = state.clone();
    let container_id = container.id;
    let container_name = container.name.clone();
    let server_endpoint = server.endpoint.clone();
    let target_tag = body.target_tag.clone();
    let api_secret = state.config.api_secret.clone();

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let agent_url = format!(
            "{}/containers/{}/update",
            server_endpoint.trim_end_matches('/'),
            container_name
        );

        let result = client
            .post(&agent_url)
            .header("X-API-Key", &api_secret)
            .json(&serde_json::json!({
                "target_tag": target_tag
            }))
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                tracing::info!("✅ Container {} updated successfully", container_name);
                // Mark job as success
                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'success', completed_at = CURRENT_TIMESTAMP WHERE id = ?"
                )
                .bind(job_id)
                .execute(&state_clone.db)
                .await;

                // Clear update_checks so the container disappears from the updates list
                let _ = sqlx::query(
                    "DELETE FROM update_checks WHERE container_id = ?"
                )
                .bind(container_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_completed",
                    "job": {
                        "id": job_id,
                        "container_name": container_name,
                        "status": "success"
                    }
                }).to_string());
            }
            Ok(response) => {
                let status = response.status();
                let error_body = response.json::<serde_json::Value>().await.unwrap_or_default();
                let error_msg = error_body.get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Agent error")
                    .to_string();
                tracing::error!("Agent returned error for {}: {} - {}", container_name, status, error_msg);

                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'failed', error_message = ?, completed_at = CURRENT_TIMESTAMP WHERE id = ?"
                )
                .bind(&error_msg)
                .bind(job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": job_id,
                        "container_name": container_name,
                        "status": "failed",
                        "error": error_msg
                    }
                }).to_string());
            }
            Err(e) => {
                let error_msg = format!("Failed to contact agent: {}", e);
                tracing::error!("{}", error_msg);

                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'failed', error_message = ?, completed_at = CURRENT_TIMESTAMP WHERE id = ?"
                )
                .bind(&error_msg)
                .bind(job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": job_id,
                        "container_name": container_name,
                        "status": "failed",
                        "error": error_msg
                    }
                }).to_string());
            }
        }
    });

    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({
        "status": "accepted",
        "job_id": job_id,
        "message": "Update job started"
    }))))
}

/// List all update jobs
pub async fn list_update_jobs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::db::UpdateJob>>, StatusCode> {
    let jobs: Vec<crate::db::UpdateJob> = sqlx::query_as(
        "SELECT * FROM update_jobs ORDER BY started_at DESC LIMIT 50"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch update jobs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(jobs))
}

/// Rollback a container to its previous image
pub async fn rollback_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<i64>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    // Get the original job
    let original_job: crate::db::UpdateJob = sqlx::query_as(
        "SELECT * FROM update_jobs WHERE id = ?"
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Database error"}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Job non trouvé"}))))?;

    let previous_image = original_job.previous_image.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Pas d'image précédente enregistrée pour ce job"})))
    })?;

    // Get the server for this container
    let server: crate::db::Server = sqlx::query_as(
        "SELECT s.* FROM servers s JOIN containers c ON c.server_id = s.id WHERE c.id = ?"
    )
    .bind(original_job.container_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Database error"}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Serveur non trouvé"}))))?;

    tracing::info!(
        "⏪ Rollback requested for container {} on {} -> {}",
        original_job.container_name,
        server.name,
        previous_image
    );

    // Create rollback job
    let rollback_job_id: i64 = sqlx::query_scalar(
        "INSERT INTO update_jobs (container_id, container_name, server_name, image, previous_image, job_type, status) VALUES (?, ?, ?, ?, ?, 'rollback', 'running') RETURNING id"
    )
    .bind(original_job.container_id)
    .bind(&original_job.container_name)
    .bind(&server.name)
    .bind(&previous_image)
    .bind(&original_job.image)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create rollback job: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to create job"})))
    })?;

    // Broadcast job started
    let _ = state.broadcast_tx.send(serde_json::json!({
        "type": "job_started",
        "job": {
            "id": rollback_job_id,
            "container_name": original_job.container_name,
            "server_name": server.name,
            "image": previous_image,
            "job_type": "rollback",
            "status": "running"
        }
    }).to_string());

    // Spawn background task
    let state_clone = state.clone();
    let container_name = original_job.container_name.clone();
    let container_id = original_job.container_id;
    let server_endpoint = server.endpoint.clone();
    let api_secret = state.config.api_secret.clone();
    let target_image = previous_image.clone();

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let agent_url = format!(
            "{}/containers/{}/update",
            server_endpoint.trim_end_matches('/'),
            container_name
        );

        let result = client
            .post(&agent_url)
            .header("X-API-Key", &api_secret)
            .json(&serde_json::json!({
                "target_image": target_image
            }))
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                tracing::info!("✅ Rollback of {} completed successfully", container_name);
                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'success', completed_at = CURRENT_TIMESTAMP WHERE id = ?"
                )
                .bind(rollback_job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_completed",
                    "job": {
                        "id": rollback_job_id,
                        "container_name": container_name,
                        "status": "success"
                    }
                }).to_string());
            }
            Ok(response) => {
                let error_body = response.json::<serde_json::Value>().await.unwrap_or_default();
                let error_msg = error_body.get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Agent error")
                    .to_string();
                tracing::error!("Rollback failed for {}: {}", container_name, error_msg);

                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'failed', error_message = ?, completed_at = CURRENT_TIMESTAMP WHERE id = ?"
                )
                .bind(&error_msg)
                .bind(rollback_job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": rollback_job_id,
                        "container_name": container_name,
                        "status": "failed",
                        "error": error_msg
                    }
                }).to_string());
            }
            Err(e) => {
                let error_msg = format!("Failed to contact agent: {}", e);
                tracing::error!("{}", error_msg);

                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'failed', error_message = ?, completed_at = CURRENT_TIMESTAMP WHERE id = ?"
                )
                .bind(&error_msg)
                .bind(rollback_job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": rollback_job_id,
                        "container_name": container_name,
                        "status": "failed",
                        "error": error_msg
                    }
                }).to_string());
            }
        }
    });

    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({
        "status": "accepted",
        "job_id": rollback_job_id,
        "message": "Rollback job started"
    }))))
}
