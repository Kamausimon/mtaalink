use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn posts_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createPosts", post(create_posts))
        .route("/getAllPosts", get(get_all_posts))
        .route("/getPost/:id", get(get_post_by_id))
        .route("/provider/:id/posts", get(get_posts_by_provider_id))
        .route("/business/:id/posts", get(get_posts_by_business_id))
        .route("/deletePost/:id", post(delete_post))
        .route("/updatePost/:id", post(update_post_and_attachments))
        .with_state(pool)
}

#[derive(Debug, Serialize, Deserialize, Validate, sqlx::FromRow)]
pub struct CreatePost {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(min = 1, max = 1000))]
    pub content: String,
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
}

pub async fn create_posts(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreatePost>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let role = sqlx::query_scalar!("SELECT role FROM users WHERE id = $1", user_id)
        .fetch_optional(&pool)
        .await?
        .flatten()
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if role == "client" {
        return Err(AppError::Forbidden("Clients are not authorized to create posts".to_string()));
    }

    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    if let Some(business_id) = payload.business_id {
        let exists = sqlx::query_scalar!("SELECT id FROM businesses WHERE id = $1", business_id)
            .fetch_optional(&pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::BadRequest("Business does not exist".to_string()));
        }
    }

    if let Some(provider_id) = payload.provider_id {
        let exists = sqlx::query_scalar!("SELECT id FROM providers WHERE id = $1", provider_id)
            .fetch_optional(&pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::BadRequest("Provider does not exist".to_string()));
        }
    }

    let now = Utc::now();
    let post = sqlx::query!(
        r#"INSERT INTO posts (title, content, business_id, provider_id, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6) RETURNING id"#,
        payload.title,
        payload.content,
        payload.business_id,
        payload.provider_id,
        now,
        now
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "post_id": post.id }))))
}

#[derive(Deserialize, Serialize)]
pub struct PostQuery {
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn get_all_posts(
    State(pool): State<PgPool>,
    Query(params): Query<PostQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut query = String::from("SELECT * FROM posts");
    let mut conditions = vec![];

    if let Some(business_id) = params.business_id {
        conditions.push(format!("business_id = {}", business_id));
    }
    if let Some(provider_id) = params.provider_id {
        conditions.push(format!("provider_id = {}", provider_id));
    }
    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    let posts = sqlx::query_as::<_, Post>(&query)
        .fetch_all(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "posts": posts }))))
}

pub async fn get_post_by_id(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    Ok((StatusCode::OK, Json(json!({ "post": post }))))
}

pub async fn get_posts_by_provider_id(
    State(pool): State<PgPool>,
    Path(provider_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE provider_id = $1")
        .bind(provider_id)
        .fetch_all(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "posts": posts }))))
}

pub async fn get_posts_by_business_id(
    State(pool): State<PgPool>,
    Path(business_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE business_id = $1")
        .bind(business_id)
        .fetch_all(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "posts": posts }))))
}

pub async fn delete_post(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id: _ }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let exists = sqlx::query_scalar!("SELECT id FROM posts WHERE id = $1", id)
        .fetch_optional(&pool)
        .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Post not found".to_string()));
    }

    sqlx::query!("DELETE FROM posts WHERE id = $1", id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Post deleted successfully" }))))
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct UpdatePost {
    pub title: Option<String>,
    pub content: Option<String>,
    pub attachments: Vec<String>,
}

pub async fn update_post_and_attachments(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id: _ }: CurrentUser,
    Json(payload): Json<UpdatePost>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.attachments.len() > 5 {
        return Err(AppError::BadRequest("Too many attachments. Maximum is 5.".to_string()));
    }

    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE posts SET title = COALESCE($1, title), content = COALESCE($2, content) WHERE id = $3",
        payload.title,
        payload.content,
        id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM attachments WHERE post_id = $1", id)
        .execute(&mut *tx)
        .await?;

    for path in &payload.attachments {
        sqlx::query!(
            "INSERT INTO attachments (post_id, file_path, file_type) VALUES ($1, $2, 'image')",
            id,
            path
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Post and attachments updated successfully" }))))
}
