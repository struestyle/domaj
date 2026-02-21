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
        &format!("SELECT {} FROM containers ORDER BY server_id, name", crate::db::SELECT_CONTAINERS),
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch containers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get all servers for mapping
    let servers: Vec<crate::db::Server> = sqlx::query_as(&format!("SELECT {} FROM servers", crate::db::SELECT_SERVERS))
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
    let container: Container = sqlx::query_as(&format!("SELECT {} FROM containers WHERE id = $1", crate::db::SELECT_CONTAINERS))
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let server: crate::db::Server = sqlx::query_as(&format!("SELECT {} FROM servers WHERE id = $1", crate::db::SELECT_SERVERS))
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
        &format!("SELECT {} FROM update_checks WHERE container_id = $1 ORDER BY checked_at DESC LIMIT 10", crate::db::SELECT_UPDATE_CHECKS),
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
    let containers: Vec<Container> = sqlx::query_as(&format!("SELECT {} FROM containers", crate::db::SELECT_CONTAINERS))
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    // Get all servers
    let servers: Vec<crate::db::Server> = sqlx::query_as(&format!("SELECT {} FROM servers", crate::db::SELECT_SERVERS))
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let server_map: std::collections::HashMap<i64, String> = servers
        .into_iter()
        .map(|s| (s.id, s.name))
        .collect();

    // Get all update checks
    let checks: Vec<UpdateCheck> = sqlx::query_as(&format!("SELECT {} FROM update_checks", crate::db::SELECT_UPDATE_CHECKS))
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
            .find(|c| c.check_type == "same_tag" && c.has_update != 0);

        let same_tag_update = same_tag_check.is_some();
        let same_tag_digest = same_tag_check.map(|c| c.remote_digest.clone()).unwrap_or_default();

        let latest_check = container_checks
            .iter()
            .find(|c| c.check_type == "latest" && c.has_update != 0);

        let latest_update = latest_check.is_some();
        let latest_tag = latest_check.map(|c| c.latest_tag.clone()).unwrap_or_default();
        let latest_digest = latest_check.map(|c| c.remote_digest.clone()).unwrap_or_default();

        let last_checked = container_checks
            .iter()
            .map(|c| c.checked_at.clone())
            .max()
            .unwrap_or_default();

        // Get version gap from any check for this container (same value for both check types)
        let versions_behind = container_checks
            .iter()
            .map(|c| c.version_gap)
            .max()
            .unwrap_or(-1);

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
                versions_behind,
                last_checked,
            });
        }
    }

    Ok(Json(updates))
}

/// Trigger a manual scan
pub async fn trigger_scan(
    State(state): State<Arc<AppState>>,
    claims: crate::api::auth::Claims,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check if any servers are configured
    let server_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM servers")
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count servers: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if server_count.0 == 0 {
        return Ok(Json(serde_json::json!({
            "status": "no_servers",
            "message": "Aucun serveur configuré"
        })));
    }

    // Audit log
    crate::api::audit::log_action(&state.db, &claims.username, "scan", "Scan manuel lancé").await;

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
    claims: crate::api::auth::Claims,
    Path(id): Path<i64>,
    Json(body): Json<UpdateContainerRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    // Get container info
    let container: Container = sqlx::query_as(&format!("SELECT {} FROM containers WHERE id = $1", crate::db::SELECT_CONTAINERS))
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch container: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Database error"})))
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Container not found"}))))?;

    // Get the server this container belongs to
    let server: crate::db::Server = sqlx::query_as(&format!("SELECT {} FROM servers WHERE id = $1", crate::db::SELECT_SERVERS))
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

    // Audit log
    crate::api::audit::log_action(
        &state.db, &claims.username, "update",
        &format!("{} sur {} (tag: {:?})", container.name, server.name, body.target_tag)
    ).await;

    // Create job in database (store current image as previous_image for rollback)
    let job_id: i64 = sqlx::query_scalar(
        "INSERT INTO update_jobs (container_id, container_name, server_name, image, target_tag, previous_image, job_type, status) VALUES ($1, $2, $3, $4, $5, $6, 'update', 'running') RETURNING id"
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
            "container_id": container.id,
            "container_name": container.name,
            "server_name": server.name,
            "image": container.image,
            "target_tag": body.target_tag,
            "job_type": "update",
            "status": "running"
        }
    }).to_string());

    // Spawn background task
    let state_clone = state.clone();
    let container_id = container.id;
    let container_name = container.name.clone();
    let server_name = server.name.clone();
    let server_endpoint = server.endpoint.clone();
    let target_tag = body.target_tag.clone();
    let api_secret = state.config.api_secret.clone();
    let previous_image = container.image.clone();
    let current_image = container.image.clone();

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
                tracing::info!("✅ Container {} updated successfully, starting health check...", container_name);
                
                // Clear update_checks so the container disappears from the updates list
                let _ = sqlx::query(
                    "DELETE FROM update_checks WHERE container_id = $1"
                )
                .bind(container_id)
                .execute(&state_clone.db)
                .await;

                // Check if auto-rollback is enabled
                let auto_rollback_enabled = crate::api::settings::get_auto_rollback_enabled(&state_clone).await;
                let rollback_delay = crate::api::settings::get_auto_rollback_delay(&state_clone).await;
                
                if auto_rollback_enabled {
                    tracing::info!("🔍 Health check scheduled for {} in {}s", container_name, rollback_delay);
                    tokio::time::sleep(std::time::Duration::from_secs(rollback_delay)).await;
                    
                    // Query the agent for container status
                    let check_url = format!(
                        "{}/containers/{}",
                        server_endpoint.trim_end_matches('/'),
                        container_name
                    );
                    
                    let check_result = client
                        .get(&check_url)
                        .header("X-API-Key", &api_secret)
                        .send()
                        .await;
                    
                    let container_healthy = match check_result {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(info) = resp.json::<serde_json::Value>().await {
                                let status = info.get("status")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("unknown");
                                tracing::info!("🔍 Container {} status after update: {}", container_name, status);
                                status == "running"
                            } else {
                                tracing::warn!("⚠️ Could not parse container status for {}", container_name);
                                true // Assume healthy if we can't parse
                            }
                        }
                        _ => {
                            tracing::warn!("⚠️ Could not reach agent to check {} status", container_name);
                            true // Assume healthy if agent is unreachable
                        }
                    };
                    
                    if container_healthy {
                        // Container is running fine, mark as success
                        let _ = sqlx::query(
                            "UPDATE update_jobs SET status = 'success', completed_at = CURRENT_TIMESTAMP WHERE id = $1"
                        )
                        .bind(job_id)
                        .execute(&state_clone.db)
                        .await;

                        let _ = state_clone.broadcast_tx.send(serde_json::json!({
                            "type": "job_completed",
                            "job": {
                                "id": job_id,
                                "container_id": container_id,
                                "container_name": container_name,
                                "job_type": "update",
                                "status": "success"
                            }
                        }).to_string());
                    } else {
                        // Container crashed! Trigger auto-rollback
                        tracing::warn!("⚠️ Container {} is NOT running after update, triggering auto-rollback!", container_name);
                        
                        let _ = sqlx::query(
                            "UPDATE update_jobs SET status = 'failed', error_message = 'Container exited after update - auto-rollback triggered', completed_at = CURRENT_TIMESTAMP WHERE id = $1"
                        )
                        .bind(job_id)
                        .execute(&state_clone.db)
                        .await;
                        
                        let _ = state_clone.broadcast_tx.send(serde_json::json!({
                            "type": "job_failed",
                            "job": {
                                "id": job_id,
                                "container_id": container_id,
                                "container_name": container_name,
                                "job_type": "update",
                                "status": "failed",
                                "error": "Container exited after update - auto-rollback triggered"
                            }
                        }).to_string());
                        
                        // Create auto-rollback job
                        let rollback_job_id: Result<i64, _> = sqlx::query_scalar(
                            "INSERT INTO update_jobs (container_id, container_name, server_name, image, previous_image, job_type, status) VALUES ($1, $2, $3, $4, $5, 'auto_rollback', 'running') RETURNING id"
                        )
                        .bind(container_id)
                        .bind(&container_name)
                        .bind(&server_name)
                        .bind(&previous_image)
                        .bind(&current_image)
                        .fetch_one(&state_clone.db)
                        .await;
                        
                        if let Ok(rb_job_id) = rollback_job_id {
                            let _ = state_clone.broadcast_tx.send(serde_json::json!({
                                "type": "job_started",
                                "job": {
                                    "id": rb_job_id,
                                    "container_id": container_id,
                                    "container_name": container_name,
                                    "server_name": server_name,
                                    "image": previous_image,
                                    "job_type": "auto_rollback",
                                    "status": "running"
                                }
                            }).to_string());
                            
                            // Execute rollback
                            let rollback_url = format!(
                                "{}/containers/{}/update",
                                server_endpoint.trim_end_matches('/'),
                                container_name
                            );
                            
                            let rb_result = client
                                .post(&rollback_url)
                                .header("X-API-Key", &api_secret)
                                .json(&serde_json::json!({
                                    "target_image": previous_image
                                }))
                                .send()
                                .await;
                            
                            match rb_result {
                                Ok(resp) if resp.status().is_success() => {
                                    tracing::info!("✅ Auto-rollback of {} completed successfully", container_name);
                                    let _ = sqlx::query(
                                        "UPDATE update_jobs SET status = 'success', completed_at = CURRENT_TIMESTAMP WHERE id = $1"
                                    )
                                    .bind(rb_job_id)
                                    .execute(&state_clone.db)
                                    .await;
                                    
                                    let _ = state_clone.broadcast_tx.send(serde_json::json!({
                                        "type": "job_completed",
                                        "job": {
                                            "id": rb_job_id,
                                            "container_id": container_id,
                                            "container_name": container_name,
                                            "status": "success",
                                            "job_type": "auto_rollback"
                                        }
                                    }).to_string());
                                }
                                _ => {
                                    tracing::error!("❌ Auto-rollback of {} FAILED!", container_name);
                                    let _ = sqlx::query(
                                        "UPDATE update_jobs SET status = 'failed', error_message = 'Auto-rollback failed', completed_at = CURRENT_TIMESTAMP WHERE id = $1"
                                    )
                                    .bind(rb_job_id)
                                    .execute(&state_clone.db)
                                    .await;
                                    
                                    let _ = state_clone.broadcast_tx.send(serde_json::json!({
                                        "type": "job_failed",
                                        "job": {
                                            "id": rb_job_id,
                                            "container_id": container_id,
                                            "container_name": container_name,
                                            "status": "failed",
                                            "job_type": "auto_rollback",
                                            "error": "Auto-rollback failed"
                                        }
                                    }).to_string());
                                }
                            }
                        }
                    }
                } else {
                    // Auto-rollback disabled, just mark success immediately
                    let _ = sqlx::query(
                        "UPDATE update_jobs SET status = 'success', completed_at = CURRENT_TIMESTAMP WHERE id = $1"
                    )
                    .bind(job_id)
                    .execute(&state_clone.db)
                    .await;

                    let _ = state_clone.broadcast_tx.send(serde_json::json!({
                        "type": "job_completed",
                        "job": {
                            "id": job_id,
                            "container_id": container_id,
                            "container_name": container_name,
                            "job_type": "update",
                            "status": "success"
                        }
                    }).to_string());
                }
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
                    "UPDATE update_jobs SET status = 'failed', error_message = $1, completed_at = CURRENT_TIMESTAMP WHERE id = $2"
                )
                .bind(&error_msg)
                .bind(job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": job_id,
                        "container_id": container_id,
                        "container_name": container_name,
                        "job_type": "update",
                        "status": "failed",
                        "error": error_msg
                    }
                }).to_string());
            }
            Err(e) => {
                let error_msg = format!("Failed to contact agent: {}", e);
                tracing::error!("{}", error_msg);

                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'failed', error_message = $1, completed_at = CURRENT_TIMESTAMP WHERE id = $2"
                )
                .bind(&error_msg)
                .bind(job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": job_id,
                        "container_id": container_id,
                        "container_name": container_name,
                        "job_type": "update",
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
        &format!("SELECT {} FROM update_jobs ORDER BY started_at DESC LIMIT 50", crate::db::SELECT_UPDATE_JOBS)
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
    claims: crate::api::auth::Claims,
    Path(job_id): Path<i64>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    // Get the original job
    let original_job: crate::db::UpdateJob = sqlx::query_as(
        &format!("SELECT {} FROM update_jobs WHERE id = $1", crate::db::SELECT_UPDATE_JOBS)
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Database error"}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Job non trouvé"}))))?;

    let previous_image = original_job.previous_image.clone();
    if previous_image.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Pas d'image précédente enregistrée pour ce job"}))));
    }

    // Get the server for this container
    let server: crate::db::Server = sqlx::query_as(
        &format!("SELECT s.id, s.name, s.endpoint, s.api_key, COALESCE(s.agent_id, '') AS agent_id, COALESCE(s.last_seen, '') AS last_seen, CAST(s.created_at AS TEXT) AS created_at FROM servers s JOIN containers c ON c.server_id = s.id WHERE c.id = $1")
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

    // Audit log
    crate::api::audit::log_action(
        &state.db, &claims.username, "rollback",
        &format!("{} sur {} → {}", original_job.container_name, server.name, previous_image)
    ).await;

    // Create rollback job
    let rollback_job_id: i64 = sqlx::query_scalar(
        "INSERT INTO update_jobs (container_id, container_name, server_name, image, previous_image, job_type, status) VALUES ($1, $2, $3, $4, $5, 'rollback', 'running') RETURNING id"
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
            "container_id": original_job.container_id,
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
                    "UPDATE update_jobs SET status = 'success', completed_at = CURRENT_TIMESTAMP WHERE id = $1"
                )
                .bind(rollback_job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_completed",
                    "job": {
                        "id": rollback_job_id,
                        "container_id": container_id,
                        "container_name": container_name,
                        "job_type": "rollback",
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
                    "UPDATE update_jobs SET status = 'failed', error_message = $1, completed_at = CURRENT_TIMESTAMP WHERE id = $2"
                )
                .bind(&error_msg)
                .bind(rollback_job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": rollback_job_id,
                        "container_id": container_id,
                        "container_name": container_name,
                        "job_type": "rollback",
                        "status": "failed",
                        "error": error_msg
                    }
                }).to_string());
            }
            Err(e) => {
                let error_msg = format!("Failed to contact agent: {}", e);
                tracing::error!("{}", error_msg);

                let _ = sqlx::query(
                    "UPDATE update_jobs SET status = 'failed', error_message = $1, completed_at = CURRENT_TIMESTAMP WHERE id = $2"
                )
                .bind(&error_msg)
                .bind(rollback_job_id)
                .execute(&state_clone.db)
                .await;

                let _ = state_clone.broadcast_tx.send(serde_json::json!({
                    "type": "job_failed",
                    "job": {
                        "id": rollback_job_id,
                        "container_id": container_id,
                        "container_name": container_name,
                        "job_type": "rollback",
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
