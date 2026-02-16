//! WebSocket handler for real-time updates

use std::sync::Arc;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
pub struct WsParams {
    token: Option<String>,
}

/// WebSocket upgrade handler with token-based auth via query parameter
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> Response {
    // Validate JWT from query parameter
    let token = match params.token {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, "Missing token").into_response(),
    };

    match super::auth::validate_jwt(&token, &state.config.jwt_secret) {
        Ok(_) => ws.on_upgrade(move |socket| handle_socket(socket, state)),
        Err(_) => (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    }
}

/// Handle individual WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("New WebSocket connection");

    // Subscribe to broadcast channel
    let mut rx = state.broadcast_tx.subscribe();

    // Send initial connection confirmation
    if socket
        .send(Message::Text(
            serde_json::json!({
                "type": "connected",
                "message": "Connected to Domaj server"
            })
            .to_string(),
        ))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            // Forward broadcast events to WebSocket client
            Ok(msg) = rx.recv() => {
                if socket.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
            // Handle incoming messages from client
            Some(msg) = socket.next() => {
                match msg {
                    Ok(Message::Ping(data)) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }
}
