use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn notification_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(list_notifications))
        .route("/unread-count", get(unread_count))
        .route("/read-all", post(mark_all_read))
        .route("/:id/read", post(mark_one_read))
        .route("/:id", delete(delete_notification))
        .with_state(pool)
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct NotifQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub unread_only: Option<bool>,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct NotificationRow {
    pub id: i32,
    pub notif_type: String,
    pub title: String,
    pub body: String,
    pub target_type: Option<String>,
    pub target_id: Option<i32>,
    pub is_read: bool,
    pub created_at: NaiveDateTime,
}

// ── GET /notifications ────────────────────────────────────────────────────────

pub async fn list_notifications(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<NotifQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let unread_only = params.unread_only.unwrap_or(false);

    let notifications = if unread_only {
        sqlx::query_as::<_, NotificationRow>(
            r#"SELECT id, notif_type, title, body, target_type, target_id, is_read, created_at
               FROM notifications
               WHERE user_id = $1 AND is_read = false
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as::<_, NotificationRow>(
            r#"SELECT id, notif_type, title, body, target_type, target_id, is_read, created_at
               FROM notifications
               WHERE user_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&pool)
        .await?
    };

    Ok((
        StatusCode::OK,
        Json(json!({
            "notifications": notifications,
            "page": page,
            "per_page": per_page,
        })),
    ))
}

// ── GET /notifications/unread-count ──────────────────────────────────────────

pub async fn unread_count(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
        user_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    Ok((StatusCode::OK, Json(json!({ "unread_count": count }))))
}

// ── POST /notifications/:id/read ─────────────────────────────────────────────

pub async fn mark_one_read(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let updated = sqlx::query!(
        "UPDATE notifications SET is_read = true WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(&pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("Notification not found".to_string()));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Marked as read" }))))
}

// ── POST /notifications/read-all ─────────────────────────────────────────────

pub async fn mark_all_read(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let updated = sqlx::query!(
        "UPDATE notifications SET is_read = true WHERE user_id = $1 AND is_read = false",
        user_id
    )
    .execute(&pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "All notifications marked as read", "updated": updated.rows_affected() })),
    ))
}

// ── DELETE /notifications/:id ─────────────────────────────────────────────────

pub async fn delete_notification(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let deleted = sqlx::query!(
        "DELETE FROM notifications WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(&pool)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("Notification not found".to_string()));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Notification deleted" }))))
}
