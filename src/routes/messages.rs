use axum::{
    Router,
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

use crate::extractors::current_user::CurrentUser;

pub fn messages_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/sendMessage", post(send_message))
        .route("/getMessages", get(get_messages))
        .route("/markMessagesAsRead", post(mark_messages_as_read))
        .route("/unreadMessagesCount", get(get_unread_messages_count))
        .with_state(pool)
}

#[derive(Deserialize, Serialize, Debug, Validate)]
pub struct NewMessage {
    pub content: String,
    pub target_type: String, // This can be "provider", "business", etc.
    pub target_id: i32,
    pub receiver_id: i32, // ID of the user receiving the message
}

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Message {
    pub id: i32,
    pub sender_id: i32,
    pub receiver_id: i32,
    pub content: String,
    pub target_type: String, // "provider", "business", etc.
    pub target_id: i32,
    pub created_at: chrono::NaiveDateTime, // Use chrono::DateTime if you want to handle dates more robustly
    pub is_read: bool,                     // Indicates if the message has been read
    pub read_at: Option<NaiveDateTime>,    // Timestamp when the message was read
}

// send message
pub async fn send_message(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<NewMessage>,
) -> impl IntoResponse {
    println!("Received payload: {:?}", payload);
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": format!("Invalid message data: {}", e) })),
        );
    }

    if payload.content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Message content cannot be empty" })),
        );
    }

    let target_type = payload.target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target type" })),
        );
    }

    let result = sqlx::query_as::<sqlx::Postgres, Message>(
    "INSERT INTO messages (sender_id, receiver_id, target_type, target_id, content)
     VALUES ($1, $2, $3, $4, $5)
     RETURNING id, sender_id, receiver_id, target_type, target_id, content, created_at, is_read, read_at"
)
.bind(user_id.parse::<i32>().unwrap())
.bind(payload.receiver_id)
.bind(&target_type)
.bind(payload.target_id)
.bind(payload.content)
.fetch_one(&pool)
.await;

    if result.is_ok(){
        let interaction = sqlx::query!(
            "INSERT INTO interactions (user_id, target_type, target_id, interaction_type)
             VALUES ($1, $2, $3, 'message')",
            user_id.parse::<i32>().unwrap(),
            &target_type,
            payload.target_id
        ).execute(&pool).await;

        if interaction.is_err() {
            eprintln!("Failed to log interaction: {}", interaction.unwrap_err());
        }
    }

   

    match result {
        Ok(message) => (StatusCode::CREATED, Json(json!({ "message": message }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to send message: {}", e) })),
        ),
    }

}

// get messages
#[derive(Deserialize, Validate, Serialize, Debug)]
pub struct MessageQuery {
    pub target_type: String, // This can be "provider", "business", etc.
    pub target_id: i32,
    pub with_user: i32, // ID of the user to filter messages with
    page: Option<i32>,  // Optional pagination parameter
    limit: Option<i32>, // Optional limit for pagination
}

pub async fn get_messages(
    State(pool): State<PgPool>,
    Query(params): Query<MessageQuery>,
    CurrentUser { user_id }: CurrentUser,
) -> impl IntoResponse {
    println!("Received query parameters: {:?}", params);
    println!(
        "Current user ID: {}, with user: {}",
        user_id, params.with_user
    );
    let user_id = user_id.parse::<i32>().unwrap();
    let target_id = params.target_id;
    let target_type = params.target_type.to_lowercase();
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let offset = (page - 1) * limit;
    if !["provider", "business"].contains(&target_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target type" })),
        );
    }

    if params.target_id <= 0 || params.with_user <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID or user ID" })),
        );
    }

    let receiver_id = match get_target_id(&pool, user_id, &target_type).await {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "message": "You are not allowed to view these messages" })),
            );
        }
    };

    println!(
        "Fetching messages for target type: {}, target ID: {}, user ID: {}",
        target_type, params.target_id, user_id
    );

    let result = sqlx::query_as::<sqlx::Postgres, Message>(
        "SELECT id, sender_id, receiver_id, content, target_type, target_id, created_at,read_at, is_read
         FROM messages
         WHERE (target_type = $1 AND target_id = $2 AND((sender_id = $3 AND receiver_id = $4) OR (sender_id = $4 AND receiver_id = $3)))
         OR (sender_id = $4 AND receiver_id = $3)
         ORDER BY created_at DESC
         LIMIT $5 OFFSET $6"
    )
    .bind(target_type)
    .bind(target_id)
     .bind(receiver_id)
     .bind(params.with_user)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(&pool)
    .await;

    match result {
        Ok(messages) => (StatusCode::OK, Json(json!({ "messages": messages }))),
        Err(e) => {
            eprintln!("Error fetching messages: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to fetch messages" })),
            )
        }
    }
}

async fn get_target_id(pool: &PgPool, user_id: i32, target_type: &str) -> Result<i32, sqlx::Error> {
    match target_type {
        "provider" => {
            let record = sqlx::query!("SELECT id FROM providers WHERE user_id = $1", user_id)
                .fetch_one(pool)
                .await?;
            Ok(record.id)
        }
        "business" => {
            let record = sqlx::query!("SELECT id FROM businesses WHERE user_id = $1", user_id)
                .fetch_one(pool)
                .await?;
            Ok(record.id)
        }
        _ => Err(sqlx::Error::RowNotFound), // You can return a custom error here
    }
}

#[derive(Deserialize, Serialize, Debug, sqlx::FromRow)]
pub struct MarkReadPayload {
    target_type: String, // "provider", "business", etc.
    target_id: i32,
    sender_id: i32,        // ID of the user who sent the message
    message_ids: Vec<i32>, // ID of the message to mark as read
}

pub async fn mark_messages_as_read(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<MarkReadPayload>,
) -> impl IntoResponse {
    if payload.message_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Message ID list cannot be empty" })),
        );
    }

    if payload.target_type.is_empty()
        || !["provider", "business"].contains(&payload.target_type.to_lowercase().as_str())
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target type" })),
        );
    }

    if payload.target_id <= 0 || payload.sender_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID or sender ID" })),
        );
    }

    let now = chrono::Utc::now().naive_utc();

    println!(
        "Marking as read: ids={:?}, receiver_id={}",
        payload.message_ids, user_id
    );

    println!("payload messages ids: {:?}", payload.message_ids);
    println!("user_id: {}", user_id);

    let user_id = user_id.parse::<i32>().unwrap();
    let target_type = payload.target_type.to_lowercase();
   
    let receiver_id = match get_target_id(&pool, user_id, &target_type).await {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "message": "You are not allowed to view these messages" })),
            );
        }
    };


    let result = sqlx::query!(
        "UPDATE messages
     SET is_read = TRUE, read_at = $1
     WHERE id = ANY($2) AND receiver_id = $3 AND is_read = FALSE",
        now,
        &payload.message_ids,
        receiver_id
    )
    .execute(&pool)
    .await;

    match result {
        Ok(res) => {
            println!("Rows affected: {}", res.rows_affected());
            (
                StatusCode::OK,
                Json(json!({ "message": "Messages marked as read successfully" })),
            )
        }
        Err(e) => {
            eprintln!("Error marking messages as read: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to mark messages as read" })),
            )
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UnreadMessagesCount {
    pub target_type: String, // "provider", "business", etc.
}

pub async fn get_unread_messages_count(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<UnreadMessagesCount>,
    
) -> impl IntoResponse {
   
    let target_type = params.target_type.to_lowercase();
    let user_id = user_id.parse::<i32>().unwrap();
    println!("Current user ID: {}", user_id);
    let receiver_id = match get_target_id(&pool, user_id, &target_type).await {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "message": "You are not allowed to view these messages" })),
            );
        }
    }; 
    println!("Receiver ID: {}", receiver_id);
    
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM messages WHERE receiver_id = $1 AND is_read = FALSE",
        receiver_id
    )
    .fetch_one(&pool)
    .await;

    match count {
        Ok(result) => (
            StatusCode::OK,
            Json(json!({ "unread_count": result.count })),
        ),
        Err(e) => {
            eprintln!("Error fetching unread messages count: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to fetch unread messages count" })),
            )
        }
    }
}
