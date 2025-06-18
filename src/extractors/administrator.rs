use axum::{
    extract::{State, Json},
    response::IntoResponse,
    http::StatusCode,
};
use sql::PgPool;
use crate::extractors::current_user::CurrentUser;

pub async fn require_admin(
    State(pool): State<PgPool>,
    CurrentUser{user_id} : CurrentUser,
)-> Result<(), impl IntoResponse> {
    let admin = sqlx::query!(
        "SELECT id FROM admins WHERE id = $1",
        user_id
    ).fetch_optional(&pool)
    .await;

    match admin {
        Ok(Some(_))=> Ok(()),
        Ok(None) => Err((StatusCode::FORBIDDEN,  Json(json!({
            "error": "You must be an administrator to access this resource."
        })))),
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "error": "An internal server error occurred."
            }))))
        }
    }
}