//! Telegram notification sender
//!
//! Sends update reports via the Telegram Bot API using reqwest.

use anyhow::{Context, Result};

use crate::db::UpdateSummary;
use crate::AppState;

/// Send an update report via Telegram
pub async fn send_telegram_report(state: &AppState, updates: &[UpdateSummary]) -> Result<()> {
    if !state.config.is_telegram_configured() {
        tracing::debug!("Telegram not configured, skipping notification");
        return Ok(());
    }

    let token = state.config.telegram_bot_token.as_ref().unwrap();
    let message = generate_telegram_message(updates);

    let client = reqwest::Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);

    for chat_id in &state.config.telegram_chat_ids {
        let response = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": message,
                "parse_mode": "MarkdownV2",
                "disable_web_page_preview": true
            }))
            .send()
            .await
            .context("Failed to send Telegram message")?;

        if response.status().is_success() {
            tracing::info!("📱 Telegram report sent to chat {}", chat_id);
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Telegram API error for chat {}: {} - {}", chat_id, status, body);
        }
    }

    Ok(())
}

/// Generate a Telegram-formatted message (MarkdownV2)
fn generate_telegram_message(updates: &[UpdateSummary]) -> String {
    let mut msg = String::new();

    msg.push_str(&format!(
        "🐳 *Domaj \\- Rapport de Mises à Jour*\n_{} mise\\(s\\) à jour disponible\\(s\\)_\n\n",
        updates.len()
    ));

    for update in updates {
        msg.push_str(&format!(
            "📦 *{}* sur _{}_\n`{}`\n",
            escape_markdown(&update.container_name),
            escape_markdown(&update.server_name),
            escape_markdown(&update.image),
        ));

        if update.same_tag_update {
            msg.push_str("  🔄 Même tag mis à jour\n");
        }
        if update.latest_update {
            let tag = if update.latest_tag.is_empty() { "latest" } else { &update.latest_tag };
            msg.push_str(&format!(
                "  🆕 Tag `{}` disponible\n",
                escape_markdown(tag)
            ));
        }
        msg.push('\n');
    }

    msg
}

/// Escape special characters for Telegram MarkdownV2
fn escape_markdown(text: &str) -> String {
    let special_chars = ['_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!'];
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if special_chars.contains(&c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}
