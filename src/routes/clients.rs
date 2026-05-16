use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::parse_image_from_multipart;
use crate::utils::storage::{SharedStorage, generate_key};
use axum::{
    Extension, Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn client_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/uploadProfilePicture", post(upload_profile_picture))
        .route("/me/profile", get(get_my_profile).put(update_my_profile))
        .with_state(pool)
}

// ── Profile ───────────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct ClientProfile {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub profile_picture: Option<String>,
    pub phone: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateProfileInput {
    pub phone: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
}

pub async fn get_my_profile(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let profile = sqlx::query_as!(
        ClientProfile,
        r#"SELECT c.id, u.username, u.email,
                  c.profile_picture, c.phone, c.bio, c.location
           FROM clients c
           JOIN users u ON u.id = c.user_id
           WHERE c.user_id = $1"#,
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Client profile not found".to_string()))?;

    Ok((StatusCode::OK, Json(json!({ "profile": profile }))))
}

pub async fn update_my_profile(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<UpdateProfileInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query!(
        r#"UPDATE clients
           SET phone    = COALESCE($1, phone),
               bio      = COALESCE($2, bio),
               location = COALESCE($3, location)
           WHERE user_id = $4"#,
        payload.phone,
        payload.bio,
        payload.location,
        user_id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Profile updated successfully" }))))
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
