use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn analytics_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/:target_type/:target_id", get(get_analytics))
        .with_state(pool)
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct AnalyticsParams {
    /// Number of past days to include (default 30, max 365).
    pub days: Option<i64>,
}

// ── Row types for sqlx::FromRow ───────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct BookingStats {
    total_bookings: i64,
    pending: i64,
    confirmed: i64,
    completed: i64,
    cancelled: i64,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct DailyStat {
    pub date: NaiveDate,
    pub count: i64,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct DailyRevenue {
    pub date: NaiveDate,
    pub amount: f64,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct TopService {
    pub service_name: Option<String>,
    pub booking_count: i64,
    pub revenue: f64,
}

#[derive(sqlx::FromRow)]
struct RepeatClientStats {
    total_clients: i64,
    repeat_clients: i64,
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn get_analytics(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path((target_type, target_id)): Path<(String, i32)>,
    Query(params): Query<AnalyticsParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest(
            "target_type must be 'provider' or 'business'".to_string(),
        ));
    }
    if target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID".to_string()));
    }

    // Ownership check — only the owner can see analytics
    let is_owner = match target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            target_id,
            user_id
        )
        .fetch_optional(&pool)
        .await?,
        _ => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            target_id,
            user_id
        )
        .fetch_optional(&pool)
        .await?,
    };

    if is_owner.is_none() {
        return Err(AppError::Forbidden(
            "You do not own this profile".to_string(),
        ));
    }

    let days = params.days.unwrap_or(30).clamp(1, 365);
    let since: NaiveDateTime = Utc::now().naive_utc() - Duration::days(days);

    // Run all 7 sub-queries in parallel
    let (
        booking_stats,
        total_revenue,
        average_rating,
        review_count,
        bookings_over_time,
        revenue_over_time,
        top_services,
        repeat_stats,
    ) = tokio::try_join!(
        query_booking_stats(&pool, &target_type, target_id, since),
        query_total_revenue(&pool, &target_type, target_id, since),
        query_average_rating(&pool, &target_type, target_id),
        query_review_count(&pool, &target_type, target_id),
        query_bookings_over_time(&pool, &target_type, target_id, since),
        query_revenue_over_time(&pool, &target_type, target_id, since),
        query_top_services(&pool, &target_type, target_id, since),
        query_repeat_clients(&pool, &target_type, target_id, since),
    )?;

    let repeat_rate = if repeat_stats.total_clients > 0 {
        repeat_stats.repeat_clients as f64 / repeat_stats.total_clients as f64
    } else {
        0.0
    };

    Ok((
        StatusCode::OK,
        Json(json!({
            "period_days": days,
            "overview": {
                "total_bookings":  booking_stats.total_bookings,
                "pending":         booking_stats.pending,
                "confirmed":       booking_stats.confirmed,
                "completed":       booking_stats.completed,
                "cancelled":       booking_stats.cancelled,
                "total_revenue":   total_revenue,
                "average_rating":  average_rating,
                "review_count":    review_count,
            },
            "bookings_over_time": bookings_over_time,
            "revenue_over_time":  revenue_over_time,
            "top_services":       top_services,
            "repeat_clients": {
                "total_clients":  repeat_stats.total_clients,
                "repeat_clients": repeat_stats.repeat_clients,
                "repeat_rate":    repeat_rate,
            },
        })),
    ))
}

// ── Sub-queries ───────────────────────────────────────────────────────────────

async fn query_booking_stats(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    since: NaiveDateTime,
) -> AppResult<BookingStats> {
    sqlx::query_as::<_, BookingStats>(
        r#"SELECT
               COUNT(*)                                  AS total_bookings,
               COUNT(*) FILTER (WHERE status = 'pending')   AS pending,
               COUNT(*) FILTER (WHERE status = 'confirmed') AS confirmed,
               COUNT(*) FILTER (WHERE status = 'completed') AS completed,
               COUNT(*) FILTER (WHERE status = 'cancelled') AS cancelled
           FROM bookings
           WHERE target_type = $1 AND target_id = $2 AND created_at >= $3"#,
    )
    .bind(target_type)
    .bind(target_id)
    .bind(since)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

async fn query_total_revenue(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    since: NaiveDateTime,
) -> AppResult<f64> {
    sqlx::query_scalar::<_, Option<f64>>(
        r#"SELECT SUM(p.amount)::float8
           FROM payments p
           JOIN bookings b ON p.booking_id = b.id
           WHERE b.target_type = $1 AND b.target_id = $2
             AND p.status = 'completed'
             AND p.created_at >= $3"#,
    )
    .bind(target_type)
    .bind(target_id)
    .bind(since)
    .fetch_one(pool)
    .await
    .map(|v| v.unwrap_or(0.0))
    .map_err(AppError::Database)
}

async fn query_average_rating(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
) -> AppResult<f64> {
    sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(rating)::float8 FROM reviews WHERE target_type = $1 AND target_id = $2",
    )
    .bind(target_type)
    .bind(target_id)
    .fetch_one(pool)
    .await
    .map(|v| v.unwrap_or(0.0))
    .map_err(AppError::Database)
}

async fn query_review_count(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
) -> AppResult<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM reviews WHERE target_type = $1 AND target_id = $2",
    )
    .bind(target_type)
    .bind(target_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

async fn query_bookings_over_time(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    since: NaiveDateTime,
) -> AppResult<Vec<DailyStat>> {
    sqlx::query_as::<_, DailyStat>(
        r#"SELECT DATE(created_at) AS date, COUNT(*) AS count
           FROM bookings
           WHERE target_type = $1 AND target_id = $2 AND created_at >= $3
           GROUP BY DATE(created_at)
           ORDER BY date ASC"#,
    )
    .bind(target_type)
    .bind(target_id)
    .bind(since)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

async fn query_revenue_over_time(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    since: NaiveDateTime,
) -> AppResult<Vec<DailyRevenue>> {
    sqlx::query_as::<_, DailyRevenue>(
        r#"SELECT DATE(p.created_at) AS date, COALESCE(SUM(p.amount)::float8, 0.0::float8) AS amount
           FROM payments p
           JOIN bookings b ON p.booking_id = b.id
           WHERE b.target_type = $1 AND b.target_id = $2
             AND p.status = 'completed'
             AND p.created_at >= $3
           GROUP BY DATE(p.created_at)
           ORDER BY date ASC"#,
    )
    .bind(target_type)
    .bind(target_id)
    .bind(since)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

async fn query_top_services(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    since: NaiveDateTime,
) -> AppResult<Vec<TopService>> {
    sqlx::query_as::<_, TopService>(
        r#"SELECT
               COALESCE(s.title, b.service_description, 'Custom Service') AS service_name,
               COUNT(DISTINCT b.id)                                        AS booking_count,
               COALESCE((SUM(p.amount) FILTER (WHERE p.status = 'completed'))::float8, 0.0::float8)
                                                                           AS revenue
           FROM bookings b
           LEFT JOIN services s ON b.service_id = s.id
           LEFT JOIN payments p ON p.booking_id = b.id
           WHERE b.target_type = $1 AND b.target_id = $2 AND b.created_at >= $3
           GROUP BY COALESCE(s.title, b.service_description, 'Custom Service')
           ORDER BY booking_count DESC
           LIMIT 10"#,
    )
    .bind(target_type)
    .bind(target_id)
    .bind(since)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

async fn query_repeat_clients(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    since: NaiveDateTime,
) -> AppResult<RepeatClientStats> {
    sqlx::query_as::<_, RepeatClientStats>(
        r#"SELECT
               COUNT(DISTINCT client_id) AS total_clients,
               COUNT(DISTINCT CASE WHEN booking_count > 1 THEN client_id END) AS repeat_clients
           FROM (
               SELECT client_id, COUNT(*) AS booking_count
               FROM bookings
               WHERE target_type = $1 AND target_id = $2 AND created_at >= $3
               GROUP BY client_id
           ) AS stats"#,
    )
    .bind(target_type)
    .bind(target_id)
    .bind(since)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}
