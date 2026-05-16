use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::notifications::{notify_and_push, notify_target_owner_and_push};
use crate::utils::ws_state::WsConnections;
use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn reviews_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createReviews", post(create_reviews))
        .route("/getReviews", get(get_reviews))
        .route("/rankProviders", get(rank_providers))
        .route("/rankBusinesses", get(rank_businesses))
        .route("/getReviewAggById", get(get_review_agg_by_id))
        .route("/:id/replyReview", post(reply_review))
        .with_state(pool)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Review {
    comment: String,
    rating: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReviewQuery {
    target_type: String,
    target_id: i32,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct ReviewResponse {
    id: i32,
    reviewer_id: i32,
    rating: i32,
    comment: String,
    created_at: NaiveDateTime,
    /// True when the review is backed by a completed booking.
    verified: bool,
}

pub async fn create_reviews(
    State(pool): State<PgPool>,
    Extension(ws_conns): Extension<WsConnections>,
    Query(params): Query<ReviewQuery>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<Review>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.comment.trim().is_empty() {
        return Err(AppError::BadRequest("Comment cannot be empty".to_string()));
    }
    if !(1..=5).contains(&payload.rating) {
        return Err(AppError::BadRequest("Rating must be between 1 and 5".to_string()));
    }

    let target_type = match params.target_type.to_lowercase().as_str() {
        "provider" | "business" => params.target_type.to_lowercase(),
        "service_provider" => "provider".to_string(),
        _ => return Err(AppError::BadRequest("Invalid target type. Use 'provider' or 'business'".to_string())),
    };

    let target_id = params.target_id;
    if target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID. Must be greater than 0".to_string()));
    }

    let target_exists = match target_type.as_str() {
        "provider" => sqlx::query_scalar!("SELECT id FROM providers WHERE id = $1", target_id)
            .fetch_optional(&pool)
            .await?,
        _ => sqlx::query_scalar!("SELECT id FROM businesses WHERE id = $1", target_id)
            .fetch_optional(&pool)
            .await?,
    };

    if target_exists.is_none() {
        return Err(AppError::NotFound("Target not found".to_string()));
    }

    let existing_review = sqlx::query_scalar!(
        "SELECT id FROM reviews WHERE reviewer_id = $1 AND target_type = $2 AND target_id = $3",
        user_id,
        target_type,
        target_id
    )
    .fetch_optional(&pool)
    .await?;

    if existing_review.is_some() {
        return Err(AppError::Conflict("You have already reviewed this provider or business".to_string()));
    }

    // Only clients with a completed booking may leave a review
    let verified_booking_id = sqlx::query_scalar!(
        r#"SELECT id FROM bookings
           WHERE client_id = $1 AND target_type = $2 AND target_id = $3
             AND status = 'completed'
           ORDER BY updated_at DESC
           LIMIT 1"#,
        user_id,
        target_type,
        target_id
    )
    .fetch_optional(&pool)
    .await?;

    if verified_booking_id.is_none() {
        return Err(AppError::Forbidden(
            "You can only review a provider or business after completing a booking with them".to_string(),
        ));
    }

    let review = sqlx::query!(
        r#"INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment, verified_booking_id)
           VALUES ($1, $2, $3, $4, $5, $6) RETURNING id"#,
        user_id,
        target_type,
        target_id,
        payload.rating,
        payload.comment.trim(),
        verified_booking_id
    )
    .fetch_one(&pool)
    .await?;

    notify_target_owner_and_push(
        &pool, &ws_conns, &target_type, target_id,
        "new_review", "New Review",
        &format!("You received a {}-star review", payload.rating),
        Some("review"), Some(review.id),
    ).await;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Review created successfully",
            "review_id": review.id,
            "verified": true,
        })),
    ))
}

pub async fn get_reviews(
    State(pool): State<PgPool>,
    Query(params): Query<ReviewQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = params.target_type.to_lowercase();
    if target_type != "provider" && target_type != "business" {
        return Err(AppError::BadRequest("Invalid target type. Must be 'provider' or 'business'".to_string()));
    }
    if params.target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID. Must be greater than 0".to_string()));
    }

    let reviews = sqlx::query_as::<sqlx::Postgres, ReviewResponse>(
        r#"SELECT id, reviewer_id, rating, comment, created_at,
                  (verified_booking_id IS NOT NULL) AS verified
           FROM reviews
           WHERE target_type = $1 AND target_id = $2
           ORDER BY verified DESC, created_at DESC"#,
    )
    .bind(target_type)
    .bind(params.target_id)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "reviews": reviews }))))
}

#[derive(Deserialize, Serialize, Debug, sqlx::FromRow)]
pub struct AggregatedRating {
    pub target_id: i32,
    pub average_rating: f64,
    pub review_count: i64,
}

pub async fn rank_providers(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let results = sqlx::query_as::<sqlx::Postgres, AggregatedRating>(
        "SELECT target_id, ROUND(AVG(rating)::numeric,2)::float8 as average_rating, COUNT(*) as review_count
         FROM reviews WHERE target_type = 'provider'
         GROUP BY target_id ORDER BY average_rating DESC, review_count DESC",
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "ranked_providers": results }))))
}

pub async fn rank_businesses(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let results = sqlx::query_as::<sqlx::Postgres, AggregatedRating>(
        "SELECT target_id, AVG(rating) as average_rating, COUNT(*) as review_count
         FROM reviews WHERE target_type = 'business'
         GROUP BY target_id ORDER BY average_rating DESC, review_count DESC",
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "ranked_businesses": results }))))
}

pub async fn get_review_agg_by_id(
    State(pool): State<PgPool>,
    Query(params): Query<ReviewQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = params.target_type.to_lowercase();
    if target_type != "provider" && target_type != "business" {
        return Err(AppError::BadRequest("Invalid target type. Must be 'provider' or 'business'".to_string()));
    }
    if params.target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID. Must be greater than 0".to_string()));
    }

    let result = sqlx::query_as::<sqlx::Postgres, AggregatedRating>(
        "SELECT target_id, ROUND(AVG(rating)::numeric,2)::float8 as average_rating, COUNT(*) as review_count
         FROM reviews WHERE target_type = $1 AND target_id = $2
         GROUP BY target_id",
    )
    .bind(target_type)
    .bind(params.target_id)
    .fetch_optional(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "aggregated_rating": result }))))
}

// ── Review replies ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ReplyPayload {
    pub comment: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ReviewReply {
    pub id: i32,
    pub review_id: i32,
    pub reviewer_id: i32,
    pub comment: String,
    pub created_at: NaiveDateTime,
}

/// Providers and businesses can reply once to a review left on their profile.
pub async fn reply_review(
    State(pool): State<PgPool>,
    Extension(ws_conns): Extension<WsConnections>,
    Path(review_id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<ReplyPayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.comment.trim().is_empty() {
        return Err(AppError::BadRequest("Reply comment cannot be empty".to_string()));
    }

    // Verify the review exists
    sqlx::query_scalar!("SELECT id FROM reviews WHERE id = $1", review_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Review not found".to_string()))?;

    // Prevent duplicate replies from the same user
    let existing = sqlx::query_scalar!(
        "SELECT id FROM review_replies WHERE review_id = $1 AND reviewer_id = $2",
        review_id,
        user_id
    )
    .fetch_optional(&pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("You have already replied to this review".to_string()));
    }

    let reply = sqlx::query_as!(
        ReviewReply,
        "INSERT INTO review_replies (review_id, reviewer_id, comment)
         VALUES ($1, $2, $3)
         RETURNING id, review_id, reviewer_id, comment, created_at",
        review_id,
        user_id,
        payload.comment.trim()
    )
    .fetch_one(&pool)
    .await?;

    // Notify the original reviewer that their review received a reply
    let reviewer_id = sqlx::query_scalar!(
        "SELECT reviewer_id FROM reviews WHERE id = $1", review_id
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    if let Some(rid) = reviewer_id {
        notify_and_push(
            &pool, &ws_conns, rid,
            "review_reply", "Reply to Your Review",
            "Someone replied to your review",
            Some("review"), Some(review_id),
        ).await;
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({ "message": "Reply posted successfully", "reply": reply })),
    ))
}
