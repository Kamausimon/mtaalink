use crate::utils::jwt::decode_jwt;
use crate::utils::ws_state::WsConnections;
use axum::{
    Extension, Router,
    extract::{
        Query,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::broadcast;

pub fn ws_routes() -> Router {
    Router::new().route("/", get(ws_handler))
}

// ── Query param extractor for the JWT ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct WsTokenQuery {
    pub token: String,
}

// ── Upgrade handler ───────────────────────────────────────────────────────────

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(ws_connections): Extension<WsConnections>,
    Query(params): Query<WsTokenQuery>,
) -> impl IntoResponse {
    let user_id = match decode_jwt(&params.token) {
        Ok(claims) => match claims.sub.parse::<i32>() {
            Ok(id) => id,
            Err(_) => {
                return (StatusCode::UNAUTHORIZED, "Invalid token subject").into_response()
            }
        },
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, ws_connections, user_id))
}

// ── Socket handler ────────────────────────────────────────────────────────────

async fn handle_socket(socket: WebSocket, connections: WsConnections, user_id: i32) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Subscribe to (or create) the broadcast channel for this user
    let rx: broadcast::Receiver<String> = {
        let mut map = connections.write().await;
        let tx = map.entry(user_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(256);
            tx
        });
        tx.subscribe()
    };

    tracing::info!("WebSocket connected: user_id={}", user_id);

    // Send a connected confirmation
    let hello = serde_json::to_string(&json!({ "event": "connected", "user_id": user_id }))
        .unwrap_or_default();
    if ws_sender.send(Message::Text(hello.into())).await.is_err() {
        return;
    }

    // Task: forward broadcast messages → WebSocket client
    let mut send_task = tokio::spawn(async move {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Client is too slow; skip missed messages and continue
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Task: read from WebSocket (detect close / pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {} // ignore text/binary/ping/pong from client
            }
        }
    });

    // When either task ends, abort the other (connection is gone)
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    tracing::info!("WebSocket disconnected: user_id={}", user_id);
}
