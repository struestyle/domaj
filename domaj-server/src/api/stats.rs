//! Statistics API endpoint

use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::db::{Container, UpdateCheck};
use crate::AppState;

/// Statistics response
#[derive(Serialize)]
pub struct StatsResponse {
    pub total_servers: i64,
    pub total_containers: i64,
    pub total_updates: i64,
    pub updates_by_type: UpdatesByType,
    pub updates_by_server: Vec<ServerUpdateCount>,
}

#[derive(Serialize)]
pub struct UpdatesByType {
    pub same_tag: i64,
    pub latest: i64,
}

#[derive(Serialize)]
pub struct ServerUpdateCount {
    pub server_name: String,
    pub count: i64,
}

/// Get aggregated statistics
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, StatusCode> {
    // Count servers
    let total_servers: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM servers")
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Count containers
    let total_containers: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM containers")
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get all containers and checks to compute updates
    let containers: Vec<Container> = sqlx::query_as("SELECT * FROM containers")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let checks: Vec<UpdateCheck> = sqlx::query_as("SELECT * FROM update_checks")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let servers: Vec<crate::db::Server> = sqlx::query_as("SELECT * FROM servers")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let server_map: std::collections::HashMap<i64, String> = servers
        .iter()
        .map(|s| (s.id, s.name.clone()))
        .collect();

    // Count updates by type and by server
    let mut same_tag_count: i64 = 0;
    let mut latest_count: i64 = 0;
    let mut server_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for container in &containers {
        let container_checks: Vec<&UpdateCheck> = checks
            .iter()
            .filter(|c| c.container_id == container.id)
            .collect();

        let has_same_tag = container_checks
            .iter()
            .any(|c| c.check_type == "same_tag" && c.has_update);

        let has_latest = container_checks
            .iter()
            .any(|c| c.check_type == "latest" && c.has_update);

        if has_same_tag {
            same_tag_count += 1;
        }
        if has_latest {
            latest_count += 1;
        }

        if has_same_tag || has_latest {
            let server_name = server_map
                .get(&container.server_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            *server_counts.entry(server_name).or_insert(0) += 1;
        }
    }

    let total_updates = same_tag_count + latest_count;

    let updates_by_server: Vec<ServerUpdateCount> = server_counts
        .into_iter()
        .map(|(server_name, count)| ServerUpdateCount { server_name, count })
        .collect();

    Ok(Json(StatsResponse {
        total_servers: total_servers.0,
        total_containers: total_containers.0,
        total_updates,
        updates_by_type: UpdatesByType {
            same_tag: same_tag_count,
            latest: latest_count,
        },
        updates_by_server,
    }))
}
