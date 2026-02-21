//! Settings API endpoints
//!
//! Manage configurable settings with env-var locking support.

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
pub struct UpdateSettingRequest {
    pub value: serde_json::Value,
}

/// Known settings and their types
const VALID_SETTINGS: &[&str] = &["auto_rollback", "auto_rollback_delay", "docker_username", "docker_password"];

/// Get all settings with lock status
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch settings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut settings = serde_json::Map::new();
    
    for (key, value) in &rows {
        let (parsed_value, locked) = match key.as_str() {
            "auto_rollback" => {
                let locked = state.config.auto_rollback.is_some();
                let effective_value = if locked {
                    state.config.auto_rollback.unwrap()
                } else {
                    value == "true"
                };
                (serde_json::Value::Bool(effective_value), locked)
            }
            "auto_rollback_delay" => {
                let locked = std::env::var("AUTO_ROLLBACK_DELAY").is_ok();
                let effective_value = if locked {
                    state.config.auto_rollback_delay_secs
                } else {
                    value.parse().unwrap_or(30)
                };
                (serde_json::json!(effective_value), locked)
            }
            "docker_username" => {
                let locked = state.config.docker_username.is_some();
                let effective_value = if locked {
                    state.config.docker_username.clone().unwrap_or_default()
                } else {
                    value.clone()
                };
                (serde_json::Value::String(effective_value), locked)
            }
            "docker_password" => {
                let locked = state.config.docker_password.is_some();
                let effective_value = if locked {
                    state.config.docker_password.clone().unwrap_or_default()
                } else {
                    value.clone()
                };
                // Mask the password
                let display = if effective_value.is_empty() {
                    String::new()
                } else {
                    "••••••••".to_string()
                };
                (serde_json::Value::String(display), locked)
            }
            _ => (serde_json::Value::String(value.clone()), false),
        };
        
        settings.insert(key.clone(), serde_json::json!({
            "value": parsed_value,
            "locked": locked,
        }));
    }

    // Ensure docker_username and docker_password always appear
    if !settings.contains_key("docker_username") {
        let locked = state.config.docker_username.is_some();
        let value = if locked {
            state.config.docker_username.clone().unwrap_or_default()
        } else {
            String::new()
        };
        settings.insert("docker_username".to_string(), serde_json::json!({
            "value": value,
            "locked": locked,
        }));
    }
    if !settings.contains_key("docker_password") {
        let locked = state.config.docker_password.is_some();
        let display = if locked && state.config.docker_password.is_some() {
            "••••••••".to_string()
        } else {
            String::new()
        };
        settings.insert("docker_password".to_string(), serde_json::json!({
            "value": display,
            "locked": locked,
        }));
    }
    
    Ok(Json(serde_json::Value::Object(settings)))
}

/// Update a specific setting
pub async fn update_setting(
    State(state): State<Arc<AppState>>,
    claims: crate::api::auth::Claims,
    Path(key): Path<String>,
    Json(body): Json<UpdateSettingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Validate key
    if !VALID_SETTINGS.contains(&key.as_str()) {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": format!("Unknown setting: {}", key)
        }))));
    }
    
    // Check if locked by env var
    let locked = match key.as_str() {
        "auto_rollback" => state.config.auto_rollback.is_some(),
        "auto_rollback_delay" => std::env::var("AUTO_ROLLBACK_DELAY").is_ok(),
        "docker_username" => state.config.docker_username.is_some(),
        "docker_password" => state.config.docker_password.is_some(),
        _ => false,
    };
    
    if locked {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Ce paramètre est verrouillé par variable d'environnement"
        }))));
    }
    
    // Validate value
    let value_str = match key.as_str() {
        "auto_rollback" => {
            match body.value.as_bool() {
                Some(b) => b.to_string(),
                None => return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": "auto_rollback doit être un booléen"
                })))),
            }
        }
        "auto_rollback_delay" => {
            match body.value.as_u64() {
                Some(n) if n >= 5 && n <= 300 => n.to_string(),
                _ => return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": "auto_rollback_delay doit être un nombre entre 5 et 300"
                })))),
            }
        }
        "docker_username" | "docker_password" => {
            match body.value.as_str() {
                Some(s) => s.to_string(),
                None => return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": format!("{} doit être une chaîne de caractères", key)
                })))),
            }
        }
        _ => body.value.to_string(),
    };
    
    // Upsert setting
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $2"
    )
    .bind(&key)
    .bind(&value_str)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update setting {}: {}", key, e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "error": "Erreur base de données"
        })))
    })?;
    
    let display_value = if key == "docker_password" && !value_str.is_empty() {
        "••••••••".to_string()
    } else {
        value_str.clone()
    };
    
    tracing::info!("Setting '{}' updated to '{}'", key, display_value);
    
    // Audit log
    crate::api::audit::log_action(
        &state.db, &claims.username, "settings_change",
        &format!("{} = {}", key, display_value)
    ).await;
    
    Ok(Json(serde_json::json!({
        "key": key,
        "value": display_value,
    })))
}

/// Helper: get effective auto_rollback value (env var takes priority over DB)
pub async fn get_auto_rollback_enabled(state: &AppState) -> bool {
    if let Some(env_value) = state.config.auto_rollback {
        return env_value;
    }
    
    // Read from DB
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'auto_rollback'"
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);
    
    result.map(|(v,)| v == "true").unwrap_or(true)
}

/// Helper: get effective auto_rollback_delay value
pub async fn get_auto_rollback_delay(state: &AppState) -> u64 {
    if std::env::var("AUTO_ROLLBACK_DELAY").is_ok() {
        return state.config.auto_rollback_delay_secs;
    }
    
    // Read from DB
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'auto_rollback_delay'"
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);
    
    result.map(|(v,)| v.parse().unwrap_or(30)).unwrap_or(30)
}

/// Test Docker Hub credentials by attempting to authenticate
pub async fn test_docker_credentials(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Get effective credentials (env var takes priority)
    let username = if let Some(ref u) = state.config.docker_username {
        u.clone()
    } else {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM settings WHERE key = 'docker_username'"
        )
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
        result.map(|(v,)| v).unwrap_or_default()
    };

    let password = if let Some(ref p) = state.config.docker_password {
        p.clone()
    } else {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM settings WHERE key = 'docker_password'"
        )
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
        result.map(|(v,)| v).unwrap_or_default()
    };

    if username.is_empty() || password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Identifiants manquants"
        }))));
    }

    // Test against Docker Hub API
    let client = reqwest::Client::new();
    let resp = client
        .post("https://hub.docker.com/v2/users/login")
        .json(&serde_json::json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to reach Docker Hub: {}", e);
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                "error": format!("Impossible de contacter Docker Hub: {}", e)
            })))
        })?;

    if resp.status().is_success() {
        tracing::info!("✅ Docker Hub credentials validated for user '{}'", username);
        Ok(Json(serde_json::json!({
            "status": "ok",
            "message": format!("Authentification réussie ({})", username)
        })))
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!("❌ Docker Hub auth failed for '{}': {} - {}", username, status, body);
        Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "Identifiants Docker Hub invalides"
        }))))
    }
}
