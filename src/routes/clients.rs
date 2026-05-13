use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::parse_image_from_multipart;
use crate::utils::storage::{SharedStorage, generate_key};
use axum::{
    Extension, Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
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
    Extension(storage): Extension<SharedStorage>,
    CurrentUser { user_id }: CurrentUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let (data, ext, _content_type) = parse_image_from_multipart(multipart).await?;
    let key = generate_key("clients/profile_pictures", &ext);
    let url = storage.save(&key, &data).await?;

    let result = sqlx::query!(
        "UPDATE clients SET profile_picture = $1 WHERE user_id = $2",
        url,
        user_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        let _ = storage.delete(&key).await;
        return Err(AppError::Database(e));
    }

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Profile picture uploaded successfully", "url": url })),
    ))
}
