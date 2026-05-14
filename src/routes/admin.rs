use crate::errors::{AppError, AppResult};
use crate::extractors::administrator::require_admin;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn admin_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/categories", get(get_categories))
        .route("/create_category", post(create_category))
        .route("/create_parent_category", post(create_parent_category))
        .route("/delete_category", post(delete_category))
        .route("/users", get(get_users))
        .route("/delete_user", post(delete_user))
        .route("/userAnalytics", get(get_user_analytics))
        .route("/flagContent", post(flag_content))
        .route("/resolveFlag", post(resolve_flag))
        .route("/moderateReviews", get(moderate_reviews))
        .layer(axum::middleware::from_fn_with_state(pool.clone(), require_admin))
        .with_state(pool)
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CategoryWithParent {
    pub id: i32,
    pub category_name: String,
    pub parent_id: Option<i32>,
    pub parent_name: Option<String>,
}

pub async fn get_categories(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let categories = sqlx::query_as!(
        CategoryWithParent,
        r#"SELECT c.id, c.name AS category_name, c.parent_id, p.name AS parent_name
           FROM categories c LEFT JOIN categories p ON c.parent_id = p.id"#
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "categories": categories }))))
}

#[derive(Deserialize, Serialize, Validate)]
pub struct NewCategory {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub parent_id: Option<i32>,
}

pub async fn create_category(
    State(pool): State<PgPool>,
    Json(payload): Json<NewCategory>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let row = sqlx::query!(
        "INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id",
        payload.name,
        payload.parent_id,
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "message": "Category created successfully", "id": row.id }))))
}

#[derive(Deserialize, Serialize, Validate, Debug, sqlx::FromRow)]
pub struct NewParentCategory {
    subcategory_name: String,
    parent_category_name: String,
}

pub async fn create_parent_category(
    State(pool): State<PgPool>,
    Json(payload): Json<NewParentCategory>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut tx = pool.begin().await?;

    let existing_parent = sqlx::query_scalar!(
        "SELECT id FROM categories WHERE name = $1 AND parent_id IS NULL",
        payload.parent_category_name
    )
    .fetch_optional(&mut *tx)
    .await?;

    let parent_id = if let Some(id) = existing_parent {
        id
    } else {
        sqlx::query!(
            "INSERT INTO categories (name, parent_id) VALUES ($1, NULL) RETURNING id",
            payload.parent_category_name
        )
        .fetch_one(&mut *tx)
        .await?
        .id
    };

    let subcategory = sqlx::query!(
        "INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id",
        payload.subcategory_name,
        parent_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Parent category and subcategory created successfully",
            "subcategory_id": subcategory.id
        })),
    ))
}

#[derive(Deserialize, Debug)]
pub struct DeleteCategoryParams {
    pub category_id: i32,
}

pub async fn delete_category(
    State(pool): State<PgPool>,
    Json(payload): Json<DeleteCategoryParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query!("DELETE FROM categories WHERE id = $1", payload.category_id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Category deleted successfully" }))))
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub role: Option<String>,
}

pub async fn get_users(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let users = sqlx::query_as!(
        User,
        "SELECT id, username, email, role FROM users ORDER BY id DESC"
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "users": users }))))
}

#[derive(Deserialize, Debug)]
pub struct DeleteUserParams {
    pub user_id: i32,
}

pub async fn delete_user(
    State(pool): State<PgPool>,
    Json(payload): Json<DeleteUserParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query!("DELETE FROM users WHERE id = $1", payload.user_id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "User deleted successfully" }))))
}

// ── Analytics ─────────────────────────────────────────────────────────────────

pub async fn get_user_analytics(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let clients = sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE role = 'client'")
        .fetch_one(&pool).await?;
    let providers = sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE role = 'provider'")
        .fetch_one(&pool).await?;
    let businesses = sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE role = 'business'")
        .fetch_one(&pool).await?;

    let pending = sqlx::query_scalar!("SELECT COUNT(*) FROM bookings WHERE status = 'pending'")
        .fetch_one(&pool).await?;
    let confirmed = sqlx::query_scalar!("SELECT COUNT(*) FROM bookings WHERE status = 'confirmed'")
        .fetch_one(&pool).await?;
    let completed = sqlx::query_scalar!("SELECT COUNT(*) FROM bookings WHERE status = 'completed'")
        .fetch_one(&pool).await?;
    let cancelled = sqlx::query_scalar!("SELECT COUNT(*) FROM bookings WHERE status = 'cancelled'")
        .fetch_one(&pool).await?;

    // New signups per day over the last 7 days
    let signups = sqlx::query!(
        r#"SELECT DATE(created_at) AS day, COUNT(*) AS count
           FROM users
           WHERE created_at >= NOW() - INTERVAL '7 days'
           GROUP BY DATE(created_at)
           ORDER BY day DESC"#
    )
    .fetch_all(&pool)
    .await?;

    let signups_by_day: Vec<_> = signups
        .iter()
        .map(|r| json!({ "day": r.day, "count": r.count }))
        .collect();

    Ok((
        StatusCode::OK,
        Json(json!({
            "users": {
                "clients": clients,
                "providers": providers,
                "businesses": businesses,
                "total": clients.unwrap_or(0) + providers.unwrap_or(0) + businesses.unwrap_or(0)
            },
            "bookings": {
                "pending": pending,
                "confirmed": confirmed,
                "completed": completed,
                "cancelled": cancelled
            },
            "signups_last_7_days": signups_by_day
        })),
    ))
}

// ── Content moderation ────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Debug)]
pub struct FlagContentPayload {
    pub target_type: String,
    pub target_id: i32,
    pub reason: String,
}

pub async fn flag_content(
    State(pool): State<PgPool>,
    Json(payload): Json<FlagContentPayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.reason.trim().is_empty() {
        return Err(AppError::BadRequest("Reason cannot be empty".to_string()));
    }
    if payload.target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID".to_string()));
    }

    let record = sqlx::query!(
        "INSERT INTO content_flags (target_type, target_id, reason) VALUES ($1, $2, $3) RETURNING id",
        payload.target_type,
        payload.target_id,
        payload.reason.trim()
    )
    .fetch_one(&pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "message": "Content flagged successfully", "flag_id": record.id })),
    ))
}

#[derive(serde::Deserialize, Debug)]
pub struct ResolveFlagPayload {
    pub flag_id: i32,
}

pub async fn resolve_flag(
    State(pool): State<PgPool>,
    Json(payload): Json<ResolveFlagPayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let updated = sqlx::query!(
        "UPDATE content_flags SET resolved = TRUE WHERE id = $1 AND resolved = FALSE",
        payload.flag_id
    )
    .execute(&pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("Flag not found or already resolved".to_string()));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Flag resolved successfully" }))))
}

#[derive(serde::Serialize, sqlx::FromRow, Debug)]
pub struct FlaggedReview {
    pub review_id: i32,
    pub reviewer_id: i32,
    pub target_type: String,
    pub target_id: i32,
    pub rating: i32,
    pub comment: Option<String>,
    pub flag_count: Option<i64>,
}

pub async fn moderate_reviews(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let reviews = sqlx::query_as!(
        FlaggedReview,
        r#"SELECT
               r.id AS review_id,
               r.reviewer_id,
               r.target_type,
               r.target_id,
               r.rating,
               r.comment,
               COUNT(cf.id) AS flag_count
           FROM reviews r
           LEFT JOIN content_flags cf
               ON cf.target_type = 'review' AND cf.target_id = r.id AND cf.resolved = FALSE
           GROUP BY r.id
           HAVING COUNT(cf.id) > 0
           ORDER BY flag_count DESC, r.created_at DESC"#
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "flagged_reviews": reviews }))))
}
