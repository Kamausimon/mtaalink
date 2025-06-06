use crate::extractors::current_user::CurrentUser;
use axum::{Json, response::IntoResponse};
use serde_json::json;

pub async fn dashboard(CurrentUser { user_id }: CurrentUser) -> impl IntoResponse {
    Json(json!({
        "message": "Welcome to your dashboard!",
        "user_id": user_id,
        "status": "success"
    }))
}
