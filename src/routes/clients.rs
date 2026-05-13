use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::save_image_to_fs;
use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::post,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::fs;

pub fn client_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/uploadProfilePicture", post(upload_profile_picture))
        .with_state(pool)
}

pub async fn upload_profile_picture(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let dir = "uploads/clients/profile_pictures";
    let file_name = save_image_to_fs(multipart, dir)
        .await
        .map_err(AppError::Internal)?;

    let result = sqlx::query!(
        "UPDATE clients SET profile_picture = $1 WHERE user_id = $2",
        file_name,
        user_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        let _ = fs::remove_file(format!("{}/{}", dir, file_name)).await;
        return Err(AppError::Database(e));
    }

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Profile picture uploaded successfully", "file_name": file_name })),
    ))
}
