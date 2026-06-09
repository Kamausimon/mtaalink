use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::notifications::notify_best_effort;
use crate::utils::image_upload::parse_image_from_multipart;
use crate::utils::storage::{SharedStorage, generate_key};
use crate::utils::ws_state::{WsConnections, push_to_user};
use axum::{
    Extension, Json, Router,
    extract::{Multipart, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn messages_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/sendMessage", post(send_message))
        .route("/getMessages", get(get_messages))
        .route("/markMessagesAsRead", post(mark_messages_as_read))
        .route("/unreadMessagesCount", get(get_unread_messages_count))
        .route("/conversations", get(get_conversations))
        .route("/upload", post(upload_message_attachment))
        .with_state(pool)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NewMessage {
    pub content: String,
    pub target_type: String,
    pub target_id: i32,
    pub receiver_id: i32,
}

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Message {
    pub id: i32,
    pub sender_id: i32,
    pub receiver_id: i32,
    pub content: String,
    pub target_type: String,
    pub target_id: i32,
    pub created_at: chrono::NaiveDateTime,
    pub is_read: bool,
    pub read_at: Option<NaiveDateTime>,
}

pub async fn send_message(
    State(pool): State<PgPool>,
    Extension(ws_conns): Extension<WsConnections>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<NewMessage>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.content.is_empty() {
        return Err(AppError::BadRequest("Message content cannot be empty".to_string()));
    }

    let target_type = payload.target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("Invalid target type".to_string()));
    }

    let mut tx = pool.begin().await?;

    let message = sqlx::query_as::<sqlx::Postgres, Message>(
        "INSERT INTO messages (sender_id, receiver_id, target_type, target_id, content)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, sender_id, receiver_id, target_type, target_id, content, created_at, is_read, read_at",
    )
    .bind(user_id)
    .bind(payload.receiver_id)
    .bind(&target_type)
    .bind(payload.target_id)
    .bind(&payload.content)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO interactions (user_id, target_type, target_id, interaction_type)
         VALUES ($1, $2, $3, 'message')",
        user_id,
        &target_type,
        payload.target_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    notify_best_effort(
        &pool, payload.receiver_id,
        "new_message", "New Message",
        "You have a new message",
        Some("message"), Some(message.id),
    ).await;

    push_to_user(&ws_conns, payload.receiver_id, "new_message", json!({
        "id": message.id,
        "sender_id": message.sender_id,
        "content": message.content,
        "target_type": message.target_type,
        "target_id": message.target_id,
        "created_at": message.created_at.to_string(),
    })).await;

    Ok((StatusCode::CREATED, Json(json!({ "message": message }))))
}

// ── Get messages in a thread ──────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct MessageQuery {
    pub other_user_id: i32,
    pub target_type: String,
    pub target_id: i32,
    page: Option<i32>,
    limit: Option<i32>,
}

pub async fn get_messages(
    State(pool): State<PgPool>,
    Query(params): Query<MessageQuery>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = params.target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("Invalid target type".to_string()));
    }

    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1) * limit;

    let messages = sqlx::query_as::<sqlx::Postgres, Message>(
        "SELECT id, sender_id, receiver_id, content, target_type, target_id, created_at, read_at, is_read
         FROM messages
         WHERE (
             (sender_id = $1 AND receiver_id = $2) OR
             (sender_id = $2 AND receiver_id = $1)
         )
         AND target_type = $3
         AND target_id = $4
         ORDER BY created_at ASC
         LIMIT $5 OFFSET $6",
    )
    .bind(user_id)
    .bind(params.other_user_id)
    .bind(&target_type)
    .bind(params.target_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "messages": messages }))))
}

// ── Mark messages as read ─────────────────────────────────────────────────────

#[derive(Deserialize, Serialize, Debug)]
pub struct MarkReadPayload {
    pub message_ids: Vec<i32>,
}

pub async fn mark_messages_as_read(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<MarkReadPayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.message_ids.is_empty() {
        return Err(AppError::BadRequest("Message ID list cannot be empty".to_string()));
    }

    let now = chrono::Utc::now().naive_utc();

    sqlx::query!(
        "UPDATE messages SET is_read = TRUE, read_at = $1
         WHERE id = ANY($2) AND receiver_id = $3 AND is_read = FALSE",
        now,
        &payload.message_ids,
        user_id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Messages marked as read successfully" }))))
}

// ── Unread message count ──────────────────────────────────────────────────────

pub async fn get_unread_messages_count(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM messages WHERE receiver_id = $1 AND is_read = FALSE",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "unread_count": count }))))
}

// ── Conversations list ────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct ConversationRow {
    pub other_user_id: i32,
    pub other_username: String,
    pub target_type: String,
    pub target_id: i32,
    pub last_message: String,
    pub last_message_at: NaiveDateTime,
    pub unread_count: i64,
}

pub async fn get_conversations(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    // Return one row per unique (other_user, target_type, target_id) thread,
    // with the latest message and unread count. Works for any role.
    let conversations = sqlx::query_as::<sqlx::Postgres, ConversationRow>(
        r#"
        WITH ranked AS (
            SELECT
                CASE WHEN sender_id = $1 THEN receiver_id ELSE sender_id END AS other_user_id,
                target_type,
                target_id,
                content AS last_message,
                created_at AS last_message_at,
                ROW_NUMBER() OVER (
                    PARTITION BY
                        CASE WHEN sender_id = $1 THEN receiver_id ELSE sender_id END,
                        target_type,
                        target_id
                    ORDER BY created_at DESC
                ) AS rn
            FROM messages
            WHERE sender_id = $1 OR receiver_id = $1
        ),
        unread_counts AS (
            SELECT sender_id AS other_user_id, target_type, target_id,
                   COUNT(*) AS unread_count
            FROM messages
            WHERE receiver_id = $1 AND is_read = FALSE
            GROUP BY sender_id, target_type, target_id
        )
        SELECT
            r.other_user_id,
            u.username AS other_username,
            r.target_type,
            r.target_id,
            r.last_message,
            r.last_message_at,
            COALESCE(uc.unread_count, 0) AS unread_count
        FROM ranked r
        JOIN users u ON u.id = r.other_user_id
        LEFT JOIN unread_counts uc
            ON  uc.other_user_id = r.other_user_id
            AND uc.target_type   = r.target_type
            AND uc.target_id     = r.target_id
        WHERE r.rn = 1
        ORDER BY r.last_message_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "conversations": conversations }))))
}

// ── Upload message attachment ─────────────────────────────────────────────────

pub async fn upload_message_attachment(
    State(pool): State<PgPool>,
    Extension(storage): Extension<SharedStorage>,
    CurrentUser { user_id: _ }: CurrentUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let (data, ext, _ct) = parse_image_from_multipart(multipart).await?;
    let key = generate_key("messages/attachments", &ext);
    let url = storage.save(&key, &data).await?;
    Ok((StatusCode::CREATED, Json(json!({ "url": url }))))
}
