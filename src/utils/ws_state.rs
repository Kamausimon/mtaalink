use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Shared map of user_id → broadcast sender.
/// Each connected WebSocket subscribes to the sender for their user_id.
pub type WsConnections = Arc<RwLock<HashMap<i32, broadcast::Sender<String>>>>;

pub fn new_ws_connections() -> WsConnections {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Push a typed event to a connected user.
/// Silently does nothing when the user is not connected.
pub async fn push_to_user(
    connections: &WsConnections,
    user_id: i32,
    event: &str,
    data: serde_json::Value,
) {
    let msg = match serde_json::to_string(&json!({ "event": event, "data": data })) {
        Ok(s) => s,
        Err(_) => return,
    };
    let map = connections.read().await;
    if let Some(sender) = map.get(&user_id) {
        let _ = sender.send(msg); // SendError is fine — no subscribers
    }
}
