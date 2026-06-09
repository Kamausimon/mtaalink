use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::notifications::notify_and_push;
use crate::utils::ws_state::WsConnections;
use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
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
        // Interactions
        .route("/:id/like", post(like_post).delete(unlike_post))
        .route("/:id/comments", get(get_comments).post(add_comment))
        .route("/:id/comments/:comment_id", delete(delete_comment))
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
    Extension(ws_conns): Extension<WsConnections>,
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

    // Notify all users who have favourited this provider/business
    let (target_type, target_id) = match (payload.provider_id, payload.business_id) {
        (Some(pid), _) => ("provider", pid),
        (_, Some(bid)) => ("business", bid),
        _ => return Ok((StatusCode::CREATED, Json(json!({ "post_id": post.id })))),
    };

    let favouriters: Vec<i32> = sqlx::query_scalar!(
        "SELECT user_id FROM favorites WHERE target_type = $1 AND target_id = $2",
        target_type, target_id
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    for uid in favouriters {
        notify_and_push(
            &pool, &ws_conns, uid,
            "new_post", "New Post",
            &format!("A provider you follow posted: {}", payload.title.trim()),
            Some(target_type), Some(target_id),
        ).await;
    }

    Ok((StatusCode::CREATED, Json(json!({ "post_id": post.id }))))
}

#[derive(Deserialize, Serialize)]
pub struct PostQuery {
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
}

#[derive(Debug, sqlx::FromRow)]
struct PostRow {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    image_urls_csv: String,
    pub like_count: i64,
    pub comment_count: Option<i64>,
}

impl PostRow {
    fn to_value(&self) -> serde_json::Value {
        let image_urls: Vec<&str> = if self.image_urls_csv.is_empty() {
            vec![]
        } else {
            self.image_urls_csv.split(',').collect()
        };
        json!({
            "id": self.id,
            "title": self.title,
            "content": self.content,
            "business_id": self.business_id,
            "provider_id": self.provider_id,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "image_urls": image_urls,
            "like_count": self.like_count,
            "comment_count": self.comment_count.unwrap_or(0),
        })
    }
}

const POSTS_WITH_DETAILS_SQL: &str = r#"
    SELECT
        p.id, p.title, p.content, p.business_id, p.provider_id,
        p.created_at, p.updated_at,
        COALESCE(string_agg(DISTINCT a.file_path, ','), '') AS image_urls_csv,
        COUNT(DISTINCT pl.user_id) AS like_count,
        (SELECT COUNT(*) FROM post_comments WHERE post_id = p.id) AS comment_count
    FROM posts p
    LEFT JOIN attachments a ON a.post_id = p.id
    LEFT JOIN post_likes pl ON pl.post_id = p.id
"#;

pub async fn get_all_posts(
    State(pool): State<PgPool>,
    Query(params): Query<PostQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let posts = sqlx::query_as::<_, PostRow>(
        &format!(
            "{} WHERE ($1::int IS NULL OR p.business_id = $1) AND ($2::int IS NULL OR p.provider_id = $2)
             GROUP BY p.id ORDER BY p.created_at DESC",
            POSTS_WITH_DETAILS_SQL
        ),
    )
    .bind(params.business_id)
    .bind(params.provider_id)
    .fetch_all(&pool)
    .await?;

    let values: Vec<serde_json::Value> = posts.iter().map(|p| p.to_value()).collect();
    Ok((StatusCode::OK, Json(json!({ "posts": values }))))
}

pub async fn get_post_by_id(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let post = sqlx::query_as::<_, PostRow>(
        &format!("{} WHERE p.id = $1 GROUP BY p.id", POSTS_WITH_DETAILS_SQL),
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    Ok((StatusCode::OK, Json(json!({ "post": post.to_value() }))))
}

pub async fn get_posts_by_provider_id(
    State(pool): State<PgPool>,
    Path(provider_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let posts = sqlx::query_as::<_, PostRow>(
        &format!("{} WHERE p.provider_id = $1 GROUP BY p.id ORDER BY p.created_at DESC", POSTS_WITH_DETAILS_SQL),
    )
    .bind(provider_id)
    .fetch_all(&pool)
    .await?;

    let values: Vec<serde_json::Value> = posts.iter().map(|p| p.to_value()).collect();
    Ok((StatusCode::OK, Json(json!({ "posts": values }))))
}

pub async fn get_posts_by_business_id(
    State(pool): State<PgPool>,
    Path(business_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let posts = sqlx::query_as::<_, PostRow>(
        &format!("{} WHERE p.business_id = $1 GROUP BY p.id ORDER BY p.created_at DESC", POSTS_WITH_DETAILS_SQL),
    )
    .bind(business_id)
    .fetch_all(&pool)
    .await?;

    let values: Vec<serde_json::Value> = posts.iter().map(|p| p.to_value()).collect();
    Ok((StatusCode::OK, Json(json!({ "posts": values }))))
}

pub async fn delete_post(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let post = sqlx::query!(
        "SELECT provider_id, business_id FROM posts WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let owned = match (post.provider_id, post.business_id) {
        (Some(pid), _) => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2", pid, user_id
        ).fetch_optional(&pool).await?.is_some(),
        (_, Some(bid)) => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2", bid, user_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };

    if !owned {
        return Err(AppError::Forbidden("You do not have permission to delete this post".to_string()));
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
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<UpdatePost>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.attachments.len() > 5 {
        return Err(AppError::BadRequest("Too many attachments. Maximum is 5.".to_string()));
    }

    let post = sqlx::query!(
        "SELECT provider_id, business_id FROM posts WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let owned = match (post.provider_id, post.business_id) {
        (Some(pid), _) => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2", pid, user_id
        ).fetch_optional(&pool).await?.is_some(),
        (_, Some(bid)) => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2", bid, user_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };

    if !owned {
        return Err(AppError::Forbidden("You do not have permission to update this post".to_string()));
    }

    let target_type = if post.provider_id.is_some() { "provider" } else { "business" };
    let target_id = post.provider_id.or(post.business_id).unwrap_or(0);

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
        let file_name = path.split('/').last().unwrap_or("image");
        sqlx::query!(
            r#"INSERT INTO attachments (post_id, file_name, file_path, file_type, target_type, target_id, uploaded_by)
               VALUES ($1, $2, $3, 'image', $4, $5, $6)"#,
            id,
            file_name,
            path,
            target_type,
            target_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Post and attachments updated successfully" }))))
}

// ── Likes ─────────────────────────────────────────────────────────────────────

pub async fn like_post(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(post_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query_scalar!("SELECT id FROM posts WHERE id = $1", post_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    sqlx::query!(
        "INSERT INTO post_likes (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        user_id, post_id
    )
    .execute(&pool)
    .await?;

    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM post_likes WHERE post_id = $1", post_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    Ok((StatusCode::OK, Json(json!({ "message": "Post liked", "likes": count }))))
}

pub async fn unlike_post(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(post_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query!(
        "DELETE FROM post_likes WHERE user_id = $1 AND post_id = $2",
        user_id, post_id
    )
    .execute(&pool)
    .await?;

    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM post_likes WHERE post_id = $1", post_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    Ok((StatusCode::OK, Json(json!({ "message": "Post unliked", "likes": count }))))
}

// ── Comments ──────────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct CommentRow {
    pub id: i32,
    pub user_id: i32,
    pub username: String,
    pub comment: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Deserialize, Debug)]
pub struct CommentInput {
    pub comment: String,
}

pub async fn get_comments(
    State(pool): State<PgPool>,
    Path(post_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let comments = sqlx::query_as::<_, CommentRow>(
        r#"SELECT pc.id, pc.user_id, u.username, pc.comment, pc.created_at
           FROM post_comments pc
           JOIN users u ON u.id = pc.user_id
           WHERE pc.post_id = $1
           ORDER BY pc.created_at ASC"#,
    )
    .bind(post_id)
    .fetch_all(&pool)
    .await?;

    let like_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM post_likes WHERE post_id = $1", post_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    Ok((StatusCode::OK, Json(json!({
        "comments": comments,
        "likes": like_count,
    }))))
}

pub async fn add_comment(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(post_id): Path<i32>,
    Json(payload): Json<CommentInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.comment.trim().is_empty() {
        return Err(AppError::BadRequest("Comment cannot be empty".to_string()));
    }

    sqlx::query_scalar!("SELECT id FROM posts WHERE id = $1", post_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let comment = sqlx::query!(
        "INSERT INTO post_comments (post_id, user_id, comment) VALUES ($1, $2, $3) RETURNING id",
        post_id, user_id, payload.comment.trim()
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "message": "Comment added", "comment_id": comment.id }))))
}

pub async fn delete_comment(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path((post_id, comment_id)): Path<(i32, i32)>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let deleted = sqlx::query!(
        "DELETE FROM post_comments WHERE id = $1 AND post_id = $2 AND user_id = $3",
        comment_id, post_id, user_id
    )
    .execute(&pool)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("Comment not found or not yours".to_string()));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Comment deleted" }))))
}
