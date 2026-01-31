//! Email notification sender
//!
//! Generates and sends update reports via SMTP.

use anyhow::{Context, Result};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::db::{Container, Server, UpdateCheck, UpdateSummary};
use crate::AppState;

/// Send an update report email
pub async fn send_update_report(state: &AppState) -> Result<()> {
    if !state.config.is_smtp_configured() {
        tracing::warn!("SMTP not configured, skipping email notification");
        return Ok(());
    }

    if state.config.notify_emails.is_empty() {
        tracing::warn!("No notification emails configured");
        return Ok(());
    }

    // Fetch update summaries (containers with available updates)
    let updates = get_pending_updates(state).await?;
    
    if updates.is_empty() {
        tracing::info!("No updates to report");
        return Ok(());
    }

    let html_body = generate_html_report(&updates);
    let text_body = generate_text_report(&updates);

    // Build email
    let from: Mailbox = state
        .config
        .smtp_from
        .as_ref()
        .unwrap()
        .parse()
        .context("Invalid SMTP_FROM address")?;

    for recipient in &state.config.notify_emails {
        let to: Mailbox = recipient.parse().context("Invalid recipient email")?;

        let email = Message::builder()
            .from(from.clone())
            .to(to)
            .subject(format!("🐳 Domaj: {} mise(s) à jour disponible(s)", updates.len()))
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(text_body.clone()),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html_body.clone()),
                    ),
            )
            .context("Failed to build email")?;

        // Send email
        let smtp_host = state.config.smtp_host.as_ref().unwrap();
        
        let mailer = if let (Some(user), Some(pass)) = (
            state.config.smtp_user.as_ref(),
            state.config.smtp_password.as_ref(),
        ) {
            let creds = Credentials::new(user.clone(), pass.clone());
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)?
                .port(state.config.smtp_port)
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)?
                .port(state.config.smtp_port)
                .build()
        };

        mailer.send(email).await.context("Failed to send email")?;
        tracing::info!("📧 Update report sent to {}", recipient);
    }

    Ok(())
}

/// Get all containers with pending updates
async fn get_pending_updates(state: &AppState) -> Result<Vec<UpdateSummary>> {
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

/// Generate HTML email body
fn generate_html_report(updates: &[UpdateSummary]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 600px; margin: 0 auto; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; text-align: center; }
        .header h1 { margin: 0; font-size: 24px; }
        .header p { margin: 5px 0 0; opacity: 0.9; }
        .content { padding: 20px; }
        .update-card { border: 1px solid #e1e5e9; border-radius: 6px; padding: 15px; margin-bottom: 15px; }
        .update-card:last-child { margin-bottom: 0; }
        .server-name { font-size: 12px; color: #666; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 5px; }
        .container-name { font-size: 18px; font-weight: 600; color: #1a1a1a; margin-bottom: 5px; }
        .image-name { font-family: monospace; font-size: 14px; color: #555; background: #f0f0f0; padding: 2px 6px; border-radius: 3px; }
        .badges { margin-top: 10px; }
        .badge { display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 12px; font-weight: 500; margin-right: 5px; }
        .badge-same { background: #fff3cd; color: #856404; }
        .badge-latest { background: #d4edda; color: #155724; }
        .footer { background: #f8f9fa; padding: 15px 20px; text-align: center; font-size: 12px; color: #666; border-top: 1px solid #e1e5e9; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🐳 Domaj - Rapport de Mises à Jour</h1>
            <p>"#,
    );
    
    html.push_str(&format!("{} mise(s) à jour disponible(s)", updates.len()));
    html.push_str(r#"</p>
        </div>
        <div class="content">"#);

    for update in updates {
        html.push_str(r#"<div class="update-card">"#);
        html.push_str(&format!(
            r#"<div class="server-name">📦 {}</div>"#,
            update.server_name
        ));
        html.push_str(&format!(
            r#"<div class="container-name">{}</div>"#,
            update.container_name
        ));
        html.push_str(&format!(
            r#"<code class="image-name">{}</code>"#,
            update.image
        ));
        html.push_str(r#"<div class="badges">"#);
        
        if update.same_tag_update {
            html.push_str(r#"<span class="badge badge-same">🔄 Même tag mis à jour</span>"#);
        }
        if update.latest_update {
            let tag = update.latest_tag.as_deref().unwrap_or("latest");
            html.push_str(&format!(
                r#"<span class="badge badge-latest">🆕 {} disponible</span>"#,
                tag
            ));
        }
        
        html.push_str("</div></div>");
    }

    html.push_str(r#"
        </div>
        <div class="footer">
            Généré par Domaj - Docker Mise à Jour<br>
            Ce rapport est envoyé automatiquement selon la fréquence configurée.
        </div>
    </div>
</body>
</html>"#);

    html
}

/// Generate plain text email body
fn generate_text_report(updates: &[UpdateSummary]) -> String {
    let mut text = format!(
        "🐳 Domaj - Rapport de Mises à Jour\n{}\n\n",
        "=".repeat(40)
    );
    
    text.push_str(&format!("{} mise(s) à jour disponible(s)\n\n", updates.len()));

    for update in updates {
        text.push_str(&format!("📦 Serveur: {}\n", update.server_name));
        text.push_str(&format!("   Conteneur: {}\n", update.container_name));
        text.push_str(&format!("   Image: {}\n", update.image));
        
        if update.same_tag_update {
            text.push_str("   🔄 Même tag mis à jour sur le registre\n");
        }
        if update.latest_update {
            let tag = update.latest_tag.as_deref().unwrap_or("latest");
            text.push_str(&format!("   🆕 Tag '{}' disponible\n", tag));
        }
        text.push_str("\n");
    }

    text.push_str(&format!("{}\n", "-".repeat(40)));
    text.push_str("Généré par Domaj - Docker Mise à Jour\n");

    text
}
