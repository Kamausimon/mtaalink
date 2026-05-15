use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::notifications::notify_best_effort;
use axum::{
    Json, Router,
    extract::{Query, State},
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

    Ok((StatusCode::CREATED, Json(json!({ "message": message }))))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MessageQuery {
    pub target_type: String,
    pub target_id: i32,
    pub with_user: i32,
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
    if params.target_id <= 0 || params.with_user <= 0 {
        return Err(AppError::BadRequest("Invalid target ID or user ID".to_string()));
    }

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page - 1) * limit;

    let receiver_id = get_target_id(&pool, user_id, &target_type)
        .await
        .map_err(|_| AppError::Forbidden("You are not allowed to view these messages".to_string()))?;

    let messages = sqlx::query_as::<sqlx::Postgres, Message>(
        "SELECT id, sender_id, receiver_id, content, target_type, target_id, created_at, read_at, is_read
         FROM messages
         WHERE (target_type = $1 AND target_id = $2 AND ((sender_id = $3 AND receiver_id = $4) OR (sender_id = $4 AND receiver_id = $3)))
         OR (sender_id = $4 AND receiver_id = $3)
         ORDER BY created_at DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(target_type)
    .bind(params.target_id)
    .bind(receiver_id)
    .bind(params.with_user)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "messages": messages }))))
}

async fn get_target_id(pool: &PgPool, user_id: i32, target_type: &str) -> Result<i32, sqlx::Error> {
    match target_type {
        "provider" => {
            let r = sqlx::query!("SELECT id FROM providers WHERE user_id = $1", user_id)
                .fetch_one(pool)
                .await?;
            Ok(r.id)
        }
        "business" => {
            let r = sqlx::query!("SELECT id FROM businesses WHERE user_id = $1", user_id)
                .fetch_one(pool)
                .await?;
            Ok(r.id)
        }
        _ => Err(sqlx::Error::RowNotFound),
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MarkReadPayload {
    target_type: String,
    target_id: i32,
    sender_id: i32,
    message_ids: Vec<i32>,
}

pub async fn mark_messages_as_read(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<MarkReadPayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.message_ids.is_empty() {
        return Err(AppError::BadRequest("Message ID list cannot be empty".to_string()));
    }

    let target_type = payload.target_type.to_lowercase();
    if target_type.is_empty() || !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("Invalid target type".to_string()));
    }
    if payload.target_id <= 0 || payload.sender_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID or sender ID".to_string()));
    }

    let receiver_id = get_target_id(&pool, user_id, &target_type)
        .await
        .map_err(|_| AppError::Forbidden("You are not allowed to view these messages".to_string()))?;

    let now = chrono::Utc::now().naive_utc();

    sqlx::query!(
        "UPDATE messages SET is_read = TRUE, read_at = $1
         WHERE id = ANY($2) AND receiver_id = $3 AND is_read = FALSE",
        now,
        &payload.message_ids,
        receiver_id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Messages marked as read successfully" }))))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UnreadMessagesCount {
    pub target_type: String,
}

pub async fn get_unread_messages_count(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<UnreadMessagesCount>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = params.target_type.to_lowercase();
    let receiver_id = get_target_id(&pool, user_id, &target_type)
        .await
        .map_err(|_| AppError::Forbidden("You are not allowed to view these messages".to_string()))?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM messages WHERE receiver_id = $1 AND is_read = FALSE",
        receiver_id
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "unread_count": count }))))
}

#[derive(Serialize, sqlx::FromRow, Debug, Deserialize)]
pub struct ConversationResponse {
    pub participant_id: Option<i32>,
    pub participant_name: Option<String>,
    pub last_message: Option<String>,
    pub last_message_time: Option<NaiveDateTime>,
    pub unread_count: Option<i64>,
    pub user_type: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ConversationsQuery {
    pub target_type: String,
}

pub async fn get_conversations(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<ConversationsQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = params.target_type.to_lowercase();
    let entity_id = get_target_id(&pool, user_id, &target_type)
        .await
        .map_err(|_| AppError::Forbidden("You are not allowed to view these conversations".to_string()))?;

    let conversations = sqlx::query_as!(
        ConversationResponse,
        r#"
        WITH conversation_partners AS (
            SELECT DISTINCT sender_id AS participant_id FROM messages
            WHERE receiver_id = $1 AND target_type = $2
            UNION
            SELECT DISTINCT receiver_id AS participant_id FROM messages
            WHERE sender_id = $1 AND target_type = $2
        ),
        last_messages AS (
            SELECT
                CASE WHEN sender_id = $1 THEN receiver_id ELSE sender_id END AS participant_id,
                content AS last_message,
                created_at AS last_message_time,
                ROW_NUMBER() OVER (
                    PARTITION BY CASE WHEN sender_id = $1 THEN receiver_id ELSE sender_id END
                    ORDER BY created_at DESC
                ) AS rn
            FROM messages
            WHERE (sender_id = $1 AND target_type = $2) OR (receiver_id = $1 AND target_type = $2)
        ),
        unread_counts AS (
            SELECT sender_id AS participant_id, COUNT(*) AS unread_count
            FROM messages
            WHERE receiver_id = $1 AND is_read = FALSE AND target_type = $2
            GROUP BY sender_id
        )
        SELECT
            COALESCE(cp.participant_id, 0) AS participant_id,
            COALESCE(u.username, '') AS participant_name,
            lm.last_message,
            lm.last_message_time,
            COALESCE(uc.unread_count, 0) AS unread_count,
            COALESCE((SELECT role FROM users WHERE id = cp.participant_id), 'unknown') AS user_type
        FROM conversation_partners cp
        LEFT JOIN users u ON cp.participant_id = u.id
        LEFT JOIN last_messages lm ON cp.participant_id = lm.participant_id AND lm.rn = 1
        LEFT JOIN unread_counts uc ON cp.participant_id = uc.participant_id
        ORDER BY lm.last_message_time DESC NULLS LAST
        "#,
        entity_id,
        target_type
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "conversations": conversations }))))
}
