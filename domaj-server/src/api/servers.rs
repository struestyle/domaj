//! Server management API endpoints

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::db::{Container, CreateServer, Server};
use crate::AppState;

/// List all registered servers
pub async fn list_servers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Server>>, StatusCode> {
    let servers = sqlx::query_as::<_, Server>("SELECT * FROM servers ORDER BY name")
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch servers: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(servers))
}

/// Get a specific server by ID
pub async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Server>, StatusCode> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(server))
}

/// Create a new server
pub async fn create_server(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateServer>,
) -> Result<(StatusCode, Json<Server>), StatusCode> {
    // Generate a unique API key for this server
    let api_key = Uuid::new_v4().to_string();

    let result = sqlx::query(
        "INSERT INTO servers (name, endpoint, api_key) VALUES (?, ?, ?)",
    )
    .bind(&input.name)
    .bind(&input.endpoint)
    .bind(&api_key)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(result.last_insert_rowid())
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch created server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Created server: {} ({})", server.name, server.endpoint);
    Ok((StatusCode::CREATED, Json(server)))
}

/// Delete a server
pub async fn delete_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM servers WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!("Deleted server with id: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

/// Get containers for a specific server
pub async fn get_server_containers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Container>>, StatusCode> {
    // Verify server exists
    let _server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let containers = sqlx::query_as::<_, Container>(
        "SELECT * FROM containers WHERE server_id = ? ORDER BY name",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch containers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(containers))
}

/// Sync containers from a server (fetch from agent)
pub async fn sync_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Container>>, StatusCode> {
    // Get server details
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Fetch containers from agent
    let client = reqwest::Client::new();
    let agent_url = format!("{}/containers", server.endpoint.trim_end_matches('/'));
    
    let response = client
        .get(&agent_url)
        .header("X-API-Key", &server.api_key)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to agent at {}: {}", agent_url, e);
            StatusCode::BAD_GATEWAY
        })?;

    if !response.status().is_success() {
        tracing::error!("Agent returned error: {}", response.status());
        return Err(StatusCode::BAD_GATEWAY);
    }

    #[derive(serde::Deserialize)]
    struct AgentContainer {
        id: String,
        name: String,
        image: String,
        image_digest: Option<String>,
        status: String,
    }

    let agent_containers: Vec<AgentContainer> = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse agent response: {}", e);
        StatusCode::BAD_GATEWAY
    })?;

    // Update database with fetched containers
    // First, remove old containers for this server
    sqlx::query("DELETE FROM containers WHERE server_id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Insert new containers
    for c in &agent_containers {
        sqlx::query(
            "INSERT INTO containers (server_id, container_id, name, image, image_digest, status) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&c.id)
        .bind(&c.name)
        .bind(&c.image)
        .bind(&c.image_digest)
        .bind(&c.status)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    // Update server last_seen
    sqlx::query("UPDATE servers SET last_seen = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .ok();

    // Return updated containers
    get_server_containers(State(state), Path(id)).await
}
