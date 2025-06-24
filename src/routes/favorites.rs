use crate::extractors::current_user::CurrentUser;
use axum::{
    Router,
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

#[derive(Deserialize, Serialize, Debug, Validate)]
pub struct FavoritePayload {
    target_type: String, // This can be "provider", "business", etc.
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
) -> impl IntoResponse {
    let target_type = payload.target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target type" })),
        );
    }

    if payload.target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID" })),
        );
    }

    let result = sqlx::query!(
        "INSERT INTO favorites (user_id, target_type, target_id) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, target_type, target_id) DO NOTHING",
        user_id.parse::<i32>().unwrap(),
        target_type,
        payload.target_id
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Favorite added successfully" })),
        ),
        Err(e) => {
            eprintln!("Error adding favorite: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to add favorite" })),
            )
        }
    }
}

pub async fn get_favorites(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> impl IntoResponse {
    let favorites = sqlx::query!(
        "SELECT target_type, target_id FROM favorites WHERE user_id = $1
        ORDER BY created_at DESC",
        user_id.parse::<i32>().unwrap()
    )
    .fetch_all(&pool)
    .await;

    match favorites {
        Ok(favs) => {
            let result: Vec<FavoritePayload> = favs
                .into_iter()
                .map(|fav| FavoritePayload {
                    target_type: fav.target_type,
                    target_id: fav.target_id,
                })
                .collect();
            (StatusCode::OK, Json(json!({ "favorites": result })))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": "Failed to fetch favorites", "error": e.to_string() })),
        ),
    }
}

pub async fn remove_favorite(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
) -> impl IntoResponse {
    if id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID" })),
        );
    }

    let result = sqlx::query!(
        "DELETE FROM favorites WHERE user_id = $1 AND target_id = $2",
        user_id.parse::<i32>().unwrap(),
        id
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Favorite removed successfully" })),
        ),
        Err(e) => {
            eprintln!("Error removing favorite: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to remove favorite" })),
            )
        }
    }
}
