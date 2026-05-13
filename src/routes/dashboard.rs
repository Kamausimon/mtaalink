use crate::extractors::current_user::CurrentUser;
use axum::Json;
use serde_json::json;

pub async fn dashboard(CurrentUser { user_id }: CurrentUser) -> Json<serde_json::Value> {
    Json(json!({
        "message": "Welcome to your dashboard!",
        "user_id": user_id,
        "status": "success"
    }))
}
