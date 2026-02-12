//! Notification module for Domaj Server
//!
//! Handles sending update reports via email and Telegram.

mod email;
mod telegram;

use anyhow::Result;

use crate::db::{Container, Server, UpdateCheck, UpdateSummary};
use crate::AppState;

/// Send update reports via all configured channels (email + Telegram)
pub async fn send_update_report(state: &AppState) -> Result<()> {
    let updates = get_pending_updates(state).await?;
    
    if updates.is_empty() {
        tracing::info!("No updates to report");
        return Ok(());
    }

    // Send email notification
    if let Err(e) = email::send_email_report(state, &updates).await {
        tracing::error!("Failed to send email notification: {}", e);
    }

    // Send Telegram notification
    if let Err(e) = telegram::send_telegram_report(state, &updates).await {
        tracing::error!("Failed to send Telegram notification: {}", e);
    }

    Ok(())
}

/// Get all containers with pending updates
pub async fn get_pending_updates(state: &AppState) -> Result<Vec<UpdateSummary>> {
    // Get all containers
    let containers: Vec<Container> = sqlx::query_as("SELECT * FROM containers")
        .fetch_all(&state.db)
        .await?;

    // Get all servers
    let servers: Vec<Server> = sqlx::query_as("SELECT * FROM servers")
        .fetch_all(&state.db)
        .await?;

    let server_map: std::collections::HashMap<i64, String> = servers
        .into_iter()
        .map(|s| (s.id, s.name))
        .collect();

    // Get all update checks
    let checks: Vec<UpdateCheck> = sqlx::query_as("SELECT * FROM update_checks")
        .fetch_all(&state.db)
        .await?;

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
                container_name: container.name,
                image: container.image,
                image_digest: container.image_digest,
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

    Ok(updates)
}
