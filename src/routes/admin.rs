use crate::errors::{AppError, AppResult};
use crate::extractors::administrator::require_admin;
use bigdecimal::BigDecimal;
use axum::{
    Json, Router,
    extract::{Path, State},
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
        .route("/payouts", get(list_pending_payouts))
        .route("/payouts/:id/approve", post(approve_payout))
        .route("/payouts/:id/reject", post(reject_payout))
        .route("/disputes", get(list_disputes))
        .route("/disputes/:id/resolve", post(resolve_dispute))
        .route("/dashboard", get(platform_dashboard))
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

// ── Payout management ─────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct PayoutRequestRow {
    pub id: i32,
    pub wallet_id: i32,
    pub amount: BigDecimal,
    pub phone_number: String,
    pub status: String,
    pub notes: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<i32>,
}

pub async fn list_pending_payouts(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let payouts = sqlx::query_as::<_, PayoutRequestRow>(
        r#"SELECT pr.id, pr.wallet_id, pr.amount, pr.phone_number, pr.status, pr.notes,
                  w.target_type, w.target_id
           FROM payout_requests pr
           JOIN wallets w ON pr.wallet_id = w.id
           WHERE pr.status = 'pending'
           ORDER BY pr.created_at ASC"#,
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "pending_payouts": payouts }))))
}

#[derive(Deserialize)]
pub struct PayoutDecision {
    pub notes: Option<String>,
}

pub async fn approve_payout(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<PayoutDecision>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let updated = sqlx::query!(
        r#"UPDATE payout_requests
           SET status = 'approved', notes = $1, updated_at = NOW()
           WHERE id = $2 AND status = 'pending'"#,
        payload.notes,
        id
    )
    .execute(&pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Payout request not found or already processed".to_string(),
        ));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Payout approved" }))))
}

pub async fn reject_payout(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<PayoutDecision>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    // Fetch the payout to get wallet_id and amount for the refund
    let payout = sqlx::query!(
        "SELECT wallet_id, amount FROM payout_requests WHERE id = $1 AND status = 'pending'",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Payout request not found or already processed".to_string()))?;

    let mut tx = pool.begin().await?;

    // Refund the balance
    sqlx::query!(
        r#"UPDATE wallets
           SET balance = balance + $1, total_paid_out = total_paid_out - $1, updated_at = NOW()
           WHERE id = $2"#,
        payout.amount,
        payout.wallet_id
    )
    .execute(&mut *tx)
    .await?;

    // Insert a credit transaction for the refund
    sqlx::query!(
        r#"INSERT INTO wallet_transactions (wallet_id, txn_type, amount, description)
           VALUES ($1, 'credit', $2, 'Payout rejected — balance refunded')"#,
        payout.wallet_id,
        payout.amount
    )
    .execute(&mut *tx)
    .await?;

    // Mark payout as rejected
    sqlx::query!(
        "UPDATE payout_requests SET status = 'rejected', notes = $1, updated_at = NOW() WHERE id = $2",
        payload.notes,
        id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Payout rejected and balance refunded" }))))
}

// ── Platform dashboard ────────────────────────────────────────────────────────

pub async fn platform_dashboard(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let (users, bookings, revenue, payouts) = tokio::try_join!(
        // User counts by role
        sqlx::query!(
            r#"SELECT
                COUNT(*) FILTER (WHERE c.id IS NOT NULL) AS clients,
                COUNT(*) FILTER (WHERE p.id IS NOT NULL) AS providers,
                COUNT(*) FILTER (WHERE b.id IS NOT NULL) AS businesses,
                COUNT(*)                                 AS total
               FROM users u
               LEFT JOIN clients   c ON c.user_id = u.id
               LEFT JOIN providers p ON p.user_id = u.id
               LEFT JOIN businesses b ON b.user_id = u.id"#
        )
        .fetch_one(&pool),

        // Booking counts by status
        sqlx::query!(
            r#"SELECT
                COUNT(*) AS total,
                COUNT(*) FILTER (WHERE status = 'pending')   AS pending,
                COUNT(*) FILTER (WHERE status = 'confirmed') AS confirmed,
                COUNT(*) FILTER (WHERE status = 'completed') AS completed,
                COUNT(*) FILTER (WHERE status = 'cancelled') AS cancelled
               FROM bookings"#
        )
        .fetch_one(&pool),

        // Revenue totals
        sqlx::query!(
            r#"SELECT
                COALESCE(SUM(amount) FILTER (WHERE status = 'completed'), 0)::float8 AS total_revenue,
                COALESCE(SUM(amount) FILTER (WHERE status = 'pending'),   0)::float8 AS pending_revenue
               FROM payments"#
        )
        .fetch_one(&pool),

        // Payout totals
        sqlx::query!(
            r#"SELECT
                COALESCE(SUM(amount) FILTER (WHERE status = 'pending'),  0)::float8 AS pending_payouts,
                COALESCE(SUM(amount) FILTER (WHERE status = 'approved'), 0)::float8 AS approved_payouts
               FROM payout_requests"#
        )
        .fetch_one(&pool),
    )?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "users": {
                "total":     users.total,
                "clients":   users.clients,
                "providers": users.providers,
                "businesses":users.businesses,
            },
            "bookings": {
                "total":     bookings.total,
                "pending":   bookings.pending,
                "confirmed": bookings.confirmed,
                "completed": bookings.completed,
                "cancelled": bookings.cancelled,
            },
            "revenue": {
                "total_collected": revenue.total_revenue,
                "pending_payments":revenue.pending_revenue,
            },
            "payouts": {
                "pending_amount":  payouts.pending_payouts,
                "approved_amount": payouts.approved_payouts,
            },
        })),
    ))
}

// ── Disputes ──────────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct DisputeRow {
    pub booking_id: i32,
    pub client_id: i32,
    pub client_username: String,
    pub target_type: String,
    pub target_id: i32,
    pub provider_name: Option<String>,
    pub service_description: Option<String>,
    pub scheduled_time: chrono::NaiveDateTime,
    pub dispute_reason: Option<String>,
    pub admin_resolution: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn list_disputes(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let disputes = sqlx::query_as!(
        DisputeRow,
        r#"SELECT
               b.id                 AS booking_id,
               b.client_id,
               u.username           AS client_username,
               b.target_type,
               b.target_id,
               COALESCE(p.service_name, biz.business_name) AS provider_name,
               b.service_description,
               b.scheduled_time,
               b.dispute_reason,
               b.admin_resolution,
               b.created_at
           FROM bookings b
           JOIN users u ON u.id = b.client_id
           LEFT JOIN providers   p   ON b.target_type = 'provider' AND b.target_id = p.id
           LEFT JOIN businesses  biz ON b.target_type = 'business' AND b.target_id = biz.id
           WHERE b.status = 'disputed'
           ORDER BY b.created_at DESC"#
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "disputes": disputes }))))
}

#[derive(Deserialize, Debug)]
pub struct ResolveDisputePayload {
    pub resolution: String,
    pub note: Option<String>,
}

pub async fn resolve_dispute(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<ResolveDisputePayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let resolution = payload.resolution.to_lowercase();
    if resolution != "completed" && resolution != "cancelled" {
        return Err(AppError::BadRequest(
            "Resolution must be 'completed' or 'cancelled'".to_string(),
        ));
    }

    let updated = sqlx::query!(
        r#"UPDATE bookings
           SET status = $1, admin_resolution = $2, updated_at = NOW()
           WHERE id = $3 AND status = 'disputed'"#,
        resolution,
        payload.note.as_deref(),
        id
    )
    .execute(&pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Disputed booking not found or already resolved".to_string(),
        ));
    }

    Ok((StatusCode::OK, Json(json!({ "message": format!("Booking marked as {resolution}") }))))
}
