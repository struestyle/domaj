//! Scheduler module for periodic scans
//!
//! Handles cron-based scheduling of container update checks.

use std::sync::Arc;
use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::db::{CheckType, Container, Server};
use crate::registry::{get_registry_client, ImageReference};
use crate::AppState;

/// Scheduler wrapper
pub struct Scheduler {
    inner: Option<JobScheduler>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { inner: None }
    }

    /// Start the scheduler with the configured interval
    pub async fn start(&mut self, state: Arc<AppState>) -> Result<()> {
        let sched = JobScheduler::new().await?;
        
        let cron_expr = &state.config.scan_interval;
        let state_clone = state.clone();
        
        let job = Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
            let state = state_clone.clone();
            Box::pin(async move {
                tracing::info!("🔍 Starting scheduled scan...");
                if let Err(e) = run_scan(&state).await {
                    tracing::error!("Scheduled scan failed: {}", e);
                }
            })
        })?;

        sched.add(job).await?;
        sched.start().await?;
        
        self.inner = Some(sched);
        
        tracing::info!("📅 Scheduler started with cron expression: {}", cron_expr);
        Ok(())
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a full scan of all servers and containers
pub async fn run_scan(state: &AppState) -> Result<()> {
    tracing::info!("Starting full scan...");
    
    // Get all servers
    let servers: Vec<Server> = sqlx::query_as("SELECT * FROM servers")
        .fetch_all(&state.db)
        .await?;

    tracing::info!("Found {} servers to scan", servers.len());

    let mut total_updates = 0;

    for server in &servers {
        tracing::info!("Scanning server: {}", server.name);
        
        // Sync containers from agent first
        if let Err(e) = sync_server_containers(state, server).await {
            tracing::warn!("Failed to sync server {}: {}", server.name, e);
            continue;
        }

        // Get containers for this server
        let containers: Vec<Container> = sqlx::query_as(
            "SELECT * FROM containers WHERE server_id = ?",
        )
        .bind(server.id)
        .fetch_all(&state.db)
        .await?;

        for container in &containers {
            match check_container_updates(state, container).await {
                Ok(has_update) => {
                    if has_update {
                        total_updates += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to check updates for {}/{}: {}",
                        server.name,
                        container.name,
                        e
                    );
                }
            }
        }
    }

    tracing::info!("Scan complete. Found {} containers with updates", total_updates);

    // Send notification if there are updates
    if total_updates > 0 {
        if let Err(e) = crate::notifier::send_update_report(state).await {
            tracing::error!("Failed to send notification: {}", e);
        }
    }

    Ok(())
}

/// Sync containers from a server's agent
async fn sync_server_containers(state: &AppState, server: &Server) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let agent_url = format!("{}/containers", server.endpoint.trim_end_matches('/'));
    
    let response = client
        .get(&agent_url)
        .header("X-API-Key", &state.config.api_secret)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Agent returned status: {}", response.status());
    }

    #[derive(serde::Deserialize)]
    struct AgentContainer {
        id: String,
        name: String,
        image: String,
        image_digest: Option<String>,
        architecture: Option<String>,
        status: String,
    }

    let agent_containers: Vec<AgentContainer> = response.json().await?;

    // Update database
    sqlx::query("DELETE FROM containers WHERE server_id = ?")
        .bind(server.id)
        .execute(&state.db)
        .await?;

    for c in &agent_containers {
        sqlx::query(
            "INSERT INTO containers (server_id, container_id, name, image, image_digest, architecture, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(server.id)
        .bind(&c.id)
        .bind(&c.name)
        .bind(&c.image)
        .bind(&c.image_digest)
        .bind(&c.architecture)
        .bind(&c.status)
        .execute(&state.db)
        .await?;
    }

    // Update last_seen
    sqlx::query("UPDATE servers SET last_seen = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(server.id)
        .execute(&state.db)
        .await?;

    Ok(())
}

/// Check for updates for a single container
async fn check_container_updates(state: &AppState, container: &Container) -> Result<bool> {
    let image_ref = ImageReference::parse(&container.image);
    let client = get_registry_client(&image_ref.registry);
    
    let mut has_any_update = false;

    // Check 1: Same tag comparison
    match client.get_digest_dyn(&image_ref.repository, &image_ref.tag).await {
        Ok(remote_digest) => {
            let local_digest = container.image_digest.clone().unwrap_or_default();
            let has_update = !local_digest.is_empty() && local_digest != remote_digest;
            
            sqlx::query(
                "INSERT INTO update_checks (container_id, check_type, local_digest, remote_digest, has_update) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(container.id)
            .bind(CheckType::SameTag.to_string())
            .bind(&local_digest)
            .bind(&remote_digest)
            .bind(has_update)
            .execute(&state.db)
            .await?;

            if has_update {
                tracing::info!(
                    "🔄 Update available for {} (same tag): {} -> {}",
                    container.name,
                    &local_digest[..12.min(local_digest.len())],
                    &remote_digest[..12.min(remote_digest.len())]
                );
                has_any_update = true;
            }
        }
        Err(e) => {
            tracing::debug!("Failed to get digest for {}: {}", container.image, e);
        }
    }

    // Check 2: Latest tag comparison (if not already using latest)
    if image_ref.tag != "latest" {
        match client.get_digest_dyn(&image_ref.repository, "latest").await {
            Ok(latest_digest) => {
                let local_digest = container.image_digest.clone().unwrap_or_default();
                let has_update = !local_digest.is_empty() && local_digest != latest_digest;
                
                sqlx::query(
                    "INSERT INTO update_checks (container_id, check_type, local_digest, remote_digest, has_update, latest_tag) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(container.id)
                .bind(CheckType::Latest.to_string())
                .bind(&local_digest)
                .bind(&latest_digest)
                .bind(has_update)
                .bind("latest")
                .execute(&state.db)
                .await?;

                if has_update {
                    tracing::info!(
                        "🆕 Newer 'latest' available for {} (current: {})",
                        container.name,
                        image_ref.tag
                    );
                    has_any_update = true;
                }
            }
            Err(e) => {
                tracing::debug!("Failed to get latest digest for {}: {}", container.image, e);
            }
        }
    }

    // Update last_checked
    sqlx::query("UPDATE containers SET last_checked = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(container.id)
        .execute(&state.db)
        .await?;

    Ok(has_any_update)
}
