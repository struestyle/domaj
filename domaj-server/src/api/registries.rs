//! Registry management API endpoints
//!
//! Provides endpoints for listing detected registries and managing credentials.

use std::collections::HashMap;
use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::config::RegistryCredential;
use crate::db::DbRegistryCredential;
use crate::registry::ImageReference;

/// Information about a detected registry
#[derive(Debug, Serialize)]
pub struct RegistryInfo {
    /// Registry hostname (e.g. "docker.io", "ghcr.io", "hb.example.com")
    pub host: String,
    /// Whether credentials are configured for this registry
    pub has_credentials: bool,
    /// Source of credentials: "env", "db", or null
    pub credential_source: Option<String>,
    /// Credential ID in database (only for source "db")
    pub credential_id: Option<i64>,
    /// Number of containers using images from this registry
    pub container_count: i64,
    /// Access status: "accessible", "auth_failed", "unreachable", "checking"
    pub status: String,
    /// Optional error message
    pub error: Option<String>,
}

/// Get all credentials merged from env + DB (env takes priority)
pub async fn get_all_credentials(state: &AppState) -> Vec<RegistryCredential> {
    // Start with env credentials
    let mut credentials: Vec<RegistryCredential> = state.config.registry_credentials.clone();
    let env_hosts: Vec<String> = credentials.iter().map(|c| c.host.clone()).collect();

    // Add DB credentials that don't conflict with env ones
    let db_creds: Vec<DbRegistryCredential> = sqlx::query_as(
        &format!("SELECT {} FROM registry_credentials", crate::db::SELECT_REGISTRY_CREDS)
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for db_cred in db_creds {
        if !env_hosts.contains(&db_cred.host) {
            credentials.push(RegistryCredential {
                host: db_cred.host,
                username: db_cred.username,
                password: db_cred.password,
            });
        }
    }

    credentials
}

/// List all detected registries with their access status
pub async fn list_registries(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<RegistryInfo>>, (StatusCode, Json<serde_json::Value>)> {
    // Get all unique registries from container images
    let containers: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT image FROM containers"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    // Count containers per registry
    let mut registry_counts: HashMap<String, i64> = HashMap::new();
    for (image,) in &containers {
        let image_ref = ImageReference::parse(image);
        *registry_counts.entry(image_ref.registry).or_insert(0) += 1;
    }

    // Load DB credentials
    let db_creds: Vec<DbRegistryCredential> = sqlx::query_as(
        &format!("SELECT {} FROM registry_credentials", crate::db::SELECT_REGISTRY_CREDS)
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Build registry info list with access checks
    let mut registries = Vec::new();
    
    for (host, count) in &registry_counts {
        // Check env credentials
        let env_cred = state.config.registry_credentials
            .iter()
            .find(|c| &c.host == host);
        
        // Check DB credentials
        let db_cred = db_creds.iter().find(|c| &c.host == host);

        let (has_credentials, credential_source, credential_id) = if env_cred.is_some() {
            (true, Some("env".to_string()), None)
        } else if let Some(dc) = db_cred {
            (true, Some("db".to_string()), Some(dc.id))
        } else {
            (false, None, None)
        };

        // Use the effective credential for access check
        let effective_cred = env_cred.cloned().or_else(|| {
            db_cred.map(|dc| RegistryCredential {
                host: dc.host.clone(),
                username: dc.username.clone(),
                password: dc.password.clone(),
            })
        });
        
        let (status, error) = check_registry_access(host, effective_cred).await;
        
        registries.push(RegistryInfo {
            host: host.clone(),
            has_credentials,
            credential_source,
            credential_id,
            container_count: *count,
            status,
            error,
        });
    }

    // Also show DB credentials for registries not yet detected in containers
    for db_cred in &db_creds {
        if !registry_counts.contains_key(&db_cred.host) {
            let cred = RegistryCredential {
                host: db_cred.host.clone(),
                username: db_cred.username.clone(),
                password: db_cred.password.clone(),
            };
            let (status, error) = check_registry_access(&db_cred.host, Some(cred)).await;
            registries.push(RegistryInfo {
                host: db_cred.host.clone(),
                has_credentials: true,
                credential_source: Some("db".to_string()),
                credential_id: Some(db_cred.id),
                container_count: 0,
                status,
                error,
            });
        }
    }

    // Also show env credentials for registries not yet detected in containers
    for env_cred in &state.config.registry_credentials {
        if !registry_counts.contains_key(&env_cred.host) {
            let (status, error) = check_registry_access(&env_cred.host, Some(env_cred.clone())).await;
            registries.push(RegistryInfo {
                host: env_cred.host.clone(),
                has_credentials: true,
                credential_source: Some("env".to_string()),
                credential_id: None,
                container_count: 0,
                status,
                error,
            });
        }
    }

    // Sort: registries with credentials first, then alphabetically
    registries.sort_by(|a, b| {
        b.has_credentials.cmp(&a.has_credentials)
            .then(a.host.cmp(&b.host))
    });

    Ok(Json(registries))
}

/// Input for creating/updating a registry credential
#[derive(Debug, Deserialize)]
pub struct CredentialInput {
    pub host: String,
    pub username: String,
    pub password: String,
}

/// Create a new registry credential
pub async fn create_credential(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CredentialInput>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    // Check if host conflicts with env credential
    if state.config.registry_credentials.iter().any(|c| c.host == input.host) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "Ce registre est configuré via les variables d'environnement et ne peut pas être ajouté ici"})),
        ));
    }

    sqlx::query(
        "INSERT INTO registry_credentials (host, username, password) VALUES ($1, $2, $3)"
    )
    .bind(&input.host)
    .bind(&input.username)
    .bind(&input.password)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "Un credential existe déjà pour ce registre"})),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"status": "created"})),
    ))
}

/// Update an existing DB registry credential
pub async fn update_credential(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(input): Json<CredentialInput>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let result = sqlx::query(
        "UPDATE registry_credentials SET host = $1, username = $2, password = $3 WHERE id = $4"
    )
    .bind(&input.host)
    .bind(&input.username)
    .bind(&input.password)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Credential non trouvé"})),
        ));
    }

    Ok(Json(serde_json::json!({"status": "updated"})))
}

/// Delete a DB registry credential
pub async fn delete_credential(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let result = sqlx::query("DELETE FROM registry_credentials WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Credential non trouvé"})),
        ));
    }

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// Check if we can access a registry
async fn check_registry_access(
    host: &str,
    credentials: Option<crate::config::RegistryCredential>,
) -> (String, Option<String>) {
    
    // Try to check access using a simple v2 API call
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
