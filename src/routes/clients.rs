use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::save_image_to_fs;
use axum::{
    Router,
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
};
use serde_json::json;
use sqlx::PgPool;

pub fn client_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/uploadProfilePicture", post(upload_profile_picture))
        .with_state(pool)
}

pub async fn upload_profile_picture(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    multipart: Multipart,
) -> impl IntoResponse {
    // Define the upload directory
    let upload_dir = "uploads/clients/profile_pictures";

    // Save the image to the filesystem
    match save_image_to_fs(multipart, upload_dir).await {
        Ok(file_name) => {
            // Update the user's profile picture in the database
            let _ = sqlx::query!(
                "UPDATE clients SET profile_picture = $1 WHERE id = $2",
                file_name,
                user_id.parse::<i32>().unwrap()
            )
            .execute(&pool)
            .await;

            (
                StatusCode::OK,
                Json(
                    json!({ "message": "Profile picture uploaded successfully", "file_name": file_name }),
                ),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to upload profile picture: {}", e) })),
        ),
    }
}
