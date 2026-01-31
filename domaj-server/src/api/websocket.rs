//! WebSocket handler for real-time updates

use std::sync::Arc;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};

use crate::AppState;

/// WebSocket upgrade handler
pub async fn ws_handler(
    State(_state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(handle_socket)
}

/// Handle individual WebSocket connection
async fn handle_socket(mut socket: WebSocket) {
    tracing::info!("New WebSocket connection");

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

    // Handle incoming messages (for future use: subscriptions, filters)
    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!("Received WS message: {}", text);
                // Echo back for now - will implement subscriptions later
                if socket
                    .send(Message::Text(
                        serde_json::json!({
                            "type": "ack",
                            "received": text
                        })
                        .to_string(),
                    ))
                    .await
                    .is_err()
                {
                    break;
                }
            }
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
}
