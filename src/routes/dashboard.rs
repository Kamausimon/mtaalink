use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::get,
};
use serde_json::json;
use sqlx::PgPool;

pub fn dashboard_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .with_state(pool)
}

pub async fn dashboard(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let user = sqlx::query!(
        "SELECT username, email, role FROM users WHERE id = $1", user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let unread_notifications: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
        user_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let upcoming_bookings: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM bookings
           WHERE client_id = $1 AND status = 'confirmed' AND scheduled_time > NOW()"#,
        user_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    let mut resp = json!({
        "user_id": user_id,
        "username": user.username,
        "email": user.email,
        "role": user.role,
        "unread_notifications": unread_notifications,
        "upcoming_bookings": upcoming_bookings,
    });

    match user.role.as_deref() {
        Some("provider") => {
            if let Some(p) = sqlx::query!("SELECT id FROM providers WHERE user_id = $1", user_id)
                .fetch_optional(&pool)
                .await?
            {
                let pending: i64 = sqlx::query_scalar!(
                    r#"SELECT COUNT(*) FROM bookings
                       WHERE target_type = 'provider' AND target_id = $1 AND status = 'pending'"#,
                    p.id
                )
                .fetch_one(&pool)
                .await?
                .unwrap_or(0);

                let wallet = sqlx::query!(
                    "SELECT balance, total_earned FROM wallets WHERE target_type = 'provider' AND target_id = $1",
                    p.id
                )
                .fetch_optional(&pool)
                .await?;

                resp["provider_id"] = json!(p.id);
                resp["pending_bookings"] = json!(pending);
                if let Some(w) = wallet {
                    resp["balance"] = json!(w.balance);
                    resp["total_earned"] = json!(w.total_earned);
                }
            }
        }
        Some("business") => {
            if let Some(b) = sqlx::query!("SELECT id FROM businesses WHERE user_id = $1", user_id)
                .fetch_optional(&pool)
                .await?
            {
                let pending: i64 = sqlx::query_scalar!(
                    r#"SELECT COUNT(*) FROM bookings
                       WHERE target_type = 'business' AND target_id = $1 AND status = 'pending'"#,
                    b.id
                )
                .fetch_one(&pool)
                .await?
                .unwrap_or(0);

                let wallet = sqlx::query!(
                    "SELECT balance, total_earned FROM wallets WHERE target_type = 'business' AND target_id = $1",
                    b.id
                )
                .fetch_optional(&pool)
                .await?;

                resp["business_id"] = json!(b.id);
                resp["pending_bookings"] = json!(pending);
                if let Some(w) = wallet {
                    resp["balance"] = json!(w.balance);
                    resp["total_earned"] = json!(w.total_earned);
                }
            }
        }
        _ => {}
    }

    Ok((StatusCode::OK, Json(resp)))
}
