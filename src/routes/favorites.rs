use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

#[derive(Deserialize, Serialize, Debug)]
pub struct FavoritePayload {
    target_type: String,
    target_id: i32,
}

pub fn favorites_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/addFavorite", post(add_favorite))
        .route("/getFavorites", get(get_favorites))
        .route("/removeFavorite/:id", post(remove_favorite))
        .with_state(pool)
}

pub async fn add_favorite(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<FavoritePayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = payload.target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("Invalid target type".to_string()));
    }
    if payload.target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID".to_string()));
    }

    sqlx::query!(
        "INSERT INTO favorites (user_id, target_type, target_id) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, target_type, target_id) DO NOTHING",
        user_id,
        target_type,
        payload.target_id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Favorite added successfully" }))))
}

pub async fn get_favorites(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let favs = sqlx::query!(
        "SELECT target_type, target_id FROM favorites WHERE user_id = $1 ORDER BY created_at DESC",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    let result: Vec<FavoritePayload> = favs
        .into_iter()
        .map(|f| FavoritePayload { target_type: f.target_type, target_id: f.target_id })
        .collect();

    Ok((StatusCode::OK, Json(json!({ "favorites": result }))))
}

pub async fn remove_favorite(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID".to_string()));
    }

    sqlx::query!(
        "DELETE FROM favorites WHERE user_id = $1 AND target_id = $2",
        user_id,
        id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Favorite removed successfully" }))))
}
