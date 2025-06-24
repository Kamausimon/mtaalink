

use axum::{
    body::Body,
    extract::{State},
    response::{IntoResponse, Response},
    http::{StatusCode, Request},
    middleware::Next,
    Json,
};
use sqlx::PgPool;
use crate::extractors::current_user::CurrentUser;
use serde_json::json;

pub async fn require_admin(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    request: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Check if the user is an admin in the database
    let admin_check = sqlx::query!("SELECT is_super_admin FROM admins WHERE id = $1", user_id.parse::<i32>().unwrap())
        .fetch_optional(&pool)
        .await;

    match admin_check {
        Ok(Some(row)) if row.is_super_admin.unwrap_or(false) => {
            Ok(next.run(request).await)
        }
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "message": "Admin access required" })),
        )),
    }
}
