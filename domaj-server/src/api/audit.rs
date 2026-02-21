//! Audit log module
//!
//! Provides a helper to record actions and an API endpoint to retrieve logs.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::db::AuditLog;
use crate::AppState;

/// Record an action in the audit log.
pub async fn log_action(
    pool: &sqlx::AnyPool,
    username: &str,
    action: &str,
    details: &str,
) {
    let result = sqlx::query(
        "INSERT INTO audit_logs (username, action, details) VALUES ($1, $2, $3)",
    )
    .bind(username)
    .bind(action)
    .bind(details)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::error!("Failed to write audit log: {}", e);
    }
}

/// Query parameters for listing audit logs
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List audit logs (newest first), with pagination.
pub async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuditLogQuery>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let logs: Vec<AuditLog> = sqlx::query_as(&format!(
        "SELECT {} FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        crate::db::SELECT_AUDIT_LOGS
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch audit logs: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs")
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count audit logs: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "logs": logs,
        "total": total.0,
        "limit": limit,
        "offset": offset,
    })))
}
