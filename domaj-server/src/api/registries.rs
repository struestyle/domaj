//! Registry management API endpoints
//!
//! Provides endpoints for listing detected registries and checking access.

use std::collections::HashMap;
use std::sync::Arc;
use axum::{extract::State, Json};
use serde::Serialize;

use crate::AppState;
use crate::registry::ImageReference;

/// Information about a detected registry
#[derive(Debug, Serialize)]
pub struct RegistryInfo {
    /// Registry hostname (e.g. "docker.io", "ghcr.io", "hb.example.com")
    pub host: String,
    /// Whether credentials are configured for this registry
    pub has_credentials: bool,
    /// Number of containers using images from this registry
    pub container_count: i64,
    /// Access status: "accessible", "auth_failed", "unreachable", "checking"
    pub status: String,
    /// Optional error message
    pub error: Option<String>,
}

/// List all detected registries with their access status
pub async fn list_registries(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<RegistryInfo>>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Get all unique registries from container images
    let containers: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT image FROM containers"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    // Count containers per registry
    let mut registry_counts: HashMap<String, i64> = HashMap::new();
    for (image,) in &containers {
        let image_ref = ImageReference::parse(image);
        *registry_counts.entry(image_ref.registry).or_insert(0) += 1;
    }

    // Build registry info list with access checks
    let mut registries = Vec::new();
    
    for (host, count) in &registry_counts {
        let credentials = state.config.registry_credentials
            .iter()
            .find(|c| &c.host == host);
        
        let has_credentials = credentials.is_some();
        
        // Check access by trying to list tags or get a manifest
        let (status, error) = check_registry_access(host, credentials.cloned()).await;
        
        registries.push(RegistryInfo {
            host: host.clone(),
            has_credentials,
            container_count: *count,
            status,
            error,
        });
    }

    // Sort: registries with credentials first, then alphabetically
    registries.sort_by(|a, b| {
        b.has_credentials.cmp(&a.has_credentials)
            .then(a.host.cmp(&b.host))
    });

    Ok(Json(registries))
}

/// Check if we can access a registry
async fn check_registry_access(
    host: &str,
    credentials: Option<crate::config::RegistryCredential>,
) -> (String, Option<String>) {
    
    // Try to check access using a simple v2 API call
    // We use a known test: try to get the catalog or a simple endpoint
    let test_result = check_v2_api(host, credentials.as_ref()).await;
    
    match test_result {
        Ok(_) => ("accessible".to_string(), None),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("401") || error_msg.contains("UNAUTHORIZED") || error_msg.contains("Unauthorized") {
                ("auth_failed".to_string(), Some(error_msg))
            } else {
                ("unreachable".to_string(), Some(error_msg))
            }
        }
    }
}

/// Check access to the registry's v2 API
async fn check_v2_api(
    host: &str,
    credentials: Option<&crate::config::RegistryCredential>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    
    let url = format!("https://{}/v2/", host);
    let resp = client.get(&url).send().await?;
    
    if resp.status().is_success() {
        return Ok(());
    }
    
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        // Need auth - try with credentials
        let www_auth = resp
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        
        if let Some(cred) = credentials {
            // Parse WWW-Authenticate and get token
            if let Some((realm, service, scope)) = parse_www_authenticate(&www_auth) {
                let mut token_url = format!("{}?service={}", realm, service);
                if !scope.is_empty() {
                    token_url = format!("{}&scope={}", token_url, scope);
                }
                
                let token_resp = client
                    .get(&token_url)
                    .header("Authorization", cred.basic_auth())
                    .send()
                    .await?;
                
                if token_resp.status().is_success() {
                    return Ok(());
                } else {
                    let status = token_resp.status();
                    let body = token_resp.text().await.unwrap_or_default();
                    anyhow::bail!("401 Unauthorized: {}", body);
                }
            }
        }
        
        anyhow::bail!("401 Unauthorized: credentials required");
    }
    
    let status = resp.status();
    anyhow::bail!("Registry returned {}", status);
}

/// Parse WWW-Authenticate header
fn parse_www_authenticate(header: &str) -> Option<(String, String, String)> {
    let header = header.strip_prefix("Bearer ")?;
    
    let mut realm = None;
    let mut service = None;
    let mut scope = None;
    
    for part in header.split(',') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("realm=") {
            realm = Some(val.trim_matches('"').to_string());
        } else if let Some(val) = part.strip_prefix("service=") {
            service = Some(val.trim_matches('"').to_string());
        } else if let Some(val) = part.strip_prefix("scope=") {
            scope = Some(val.trim_matches('"').to_string());
        }
    }
    
    Some((realm?, service.unwrap_or_default(), scope.unwrap_or_default()))
}
