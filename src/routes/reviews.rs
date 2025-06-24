use crate::extractors::current_user::CurrentUser;
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

pub fn reviews_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createReviews", post(create_reviews))
        .route("/getReviews", get(get_reviews))
        .route("/rankProviders", get(rank_providers))
        .route("/rankBusinesses", get(rank_businesses))
        .route("/getReviewAggById", get(get_review_agg_by_id))
        // .route("/:id/replyReview", post(reply_review)) // This can be used to reply to a review
        .with_state(pool)
}

#[derive(Deserialize, Serialize, Debug, Validate)]
pub struct Review {
    #[validate(length(min = 1, message = "Review content cannot be empty"))]
    comment: String,
    rating: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReviewQuery {
    target_type: String, // This can be "provider", "business", etc.
    target_id: i32,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct ReviewResponse {
    id: i32,
    reviewer_id: i32,
    rating: i32,
    comment: String,
    created_at: NaiveDateTime, // Use chrono::DateTime if you want to handle dates more robustly
}

pub async fn create_reviews(
    State(pool): State<PgPool>,
    Query(params): Query<ReviewQuery>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<Review>,
) -> impl IntoResponse {
    //validate the payload
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": format!("Invalid review data: {}", e) })),
        );
    }
    //check if the comment is empty
    if payload.comment.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Comment cannot be empty" })),
        );
    }

    //get reviewer id from the current user
    let reviewer_id = user_id.parse::<i32>().unwrap();

    let target_type = match params.target_type.to_lowercase().as_str() {
        "provider" | "business" => params.target_type.to_lowercase(),
        "service_provider" => "provider".to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "message": "Invalid target type. Use 'provider' or 'business'" })),
            );
        }
    };
    println!("Target Type: {}", target_type);
    let target_id = params.target_id;
    println!("Target ID: {}", target_id);

    if target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID. Must be greater than 0" })),
        );
    }

    //check if the target exists

    // Validate existence
    let target_exists = match target_type.as_str() {
        "provider" => {
            sqlx::query_scalar!("SELECT id FROM providers WHERE id = $1", target_id)
                .fetch_optional(&pool)
                .await
        }
        "business" => {
            sqlx::query_scalar!("SELECT id FROM businesses WHERE id = $1", target_id)
                .fetch_optional(&pool)
                .await
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "message": "Invalid target type. Use 'provider' or 'business'" })),
            );
        }
    };

    if target_exists.is_err() || target_exists.unwrap().is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "message": "Target not found" })),
        );
    }

    //prevent duplicate reviews
    let existing_review = sqlx::query!(
        "SELECT id FROM reviews WHERE reviewer_id = $1 AND target_type = $2 AND target_id = $3",
        reviewer_id,
        target_type,
        target_id
    )
    .fetch_optional(&pool)
    .await;
    if existing_review.is_ok() && existing_review.unwrap().is_some() {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "message": "You have already reviewed this service provider" })),
        );
    }
    //Todo : allow users to review provideers or businesses they have interacted with
    let interaction_exists = sqlx::query!(
        "SELECT id FROM interactions WHERE user_id = $1 AND target_type = $2 AND target_id = $3",
        reviewer_id,
        target_type,
        target_id
    )
    .fetch_optional(&pool)
    .await;

    match interaction_exists {
        Ok(Some(_)) => {
            // Interaction exists, proceed with review creation
        }
        Ok(None) => {
            return (
                StatusCode::FORBIDDEN,
                Json(
                    json!({ "message": "You can only review service providers or businesses you have interacted with" }),
                ),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": format!("Failed to check interactions: {}", e) })),
            );
        }
    }

    let insert_review = sqlx::query!(
        "INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
     VALUES ($1, $2, $3, $4, $5)
     RETURNING id",
        reviewer_id,
        target_type,
        target_id,
        payload.rating,
        payload.comment
    )
    .fetch_one(&pool)
    .await;

    match insert_review {
        Ok(review) => (
            StatusCode::CREATED,
            Json(
                json!({ "message": "Review created successfully", "review_id": review.id , "reviewer_id": reviewer_id, "target_type": target_type, "target_id": target_id }),
            ),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to create review: {}", e) })),
        ),
    }
}

pub async fn get_reviews(
    State(pool): State<PgPool>,
    Query(params): Query<ReviewQuery>,
) -> impl IntoResponse {
    let target_type = params.target_type.to_lowercase();

    if target_type != "provider" && target_type != "business" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target type. Must be 'provider' or 'business'" })),
        );
    }

    if params.target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID. Must be greater than 0" })),
        );
    }

    // Fetch reviews from the database
    let reviews = sqlx::query_as::<sqlx::Postgres, ReviewResponse>(
        "SELECT id, reviewer_id, rating, comment, created_at
     FROM reviews
     WHERE target_type = $1 AND target_id = $2
     ORDER BY created_at DESC",
    )
    .bind(target_type)
    .bind(params.target_id)
    .fetch_all(&pool)
    .await;

    match reviews {
        Ok(data) => (StatusCode::OK, Json(json!({ "reviews": data }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to fetch reviews: {}", e) })),
        ),
    }
}

#[derive(Deserialize, Serialize, Debug, sqlx::FromRow)]
pub struct AggregatedRating {
    pub target_id: i32,
    pub average_rating: f64,
    pub review_count: i64,
}

pub async fn rank_providers(State(pool): State<PgPool>) -> impl IntoResponse {
    let results = sqlx::query_as::<sqlx::Postgres, AggregatedRating>(
        "SELECT target_id, ROUND(AVG(rating)::numeric,2)::float8 as average_rating, COUNT(*) as review_count
         FROM reviews
         WHERE target_type = 'provider'
         GROUP BY target_id
         ORDER BY average_rating DESC, review_count DESC"
    ).fetch_all(&pool).await;

    match results {
        Ok(data) => (StatusCode::OK, Json(json!({ "ranked_providers": data }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to rank providers: {}", e) })),
        ),
    }
}

pub async fn rank_businesses(State(pool): State<PgPool>) -> impl IntoResponse {
    let results = sqlx::query_as::<sqlx::Postgres, AggregatedRating>(
        "SELECT target_id, AVG(rating) as average_rating, COUNT(*) as review_count
         FROM reviews
         WHERE target_type = 'business'
         GROUP BY target_id
         ORDER BY average_rating DESC, review_count DESC",
    )
    .fetch_all(&pool)
    .await;

    match results {
        Ok(data) => (StatusCode::OK, Json(json!({ "ranked_businesses": data }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to rank businesses: {}", e) })),
        ),
    }
}

pub async fn get_review_agg_by_id(
    State(pool): State<PgPool>,
    Query(params): Query<ReviewQuery>,
) -> impl IntoResponse {
    let target_type = params.target_type.to_lowercase();

    if target_type != "provider" && target_type != "business" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target type. Must be 'provider' or 'business'" })),
        );
    }

    if params.target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID. Must be greater than 0" })),
        );
    }

    // Fetch aggregated rating from the database
    let result = sqlx::query_as::<sqlx::Postgres, AggregatedRating>(
        "SELECT target_id,ROUND(AVG(rating)::numeric,2)::float8 as average_rating, COUNT(*) as review_count
         FROM reviews
         WHERE target_type = $1 AND target_id = $2
         GROUP BY target_id"
    )
    .bind(target_type)
    .bind(params.target_id)
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(data) => (StatusCode::OK, Json(json!({ "aggregated_rating": data }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to fetch aggregated rating: {}", e) })),
        ),
    }
}

//todo

// pub struct ReplyReview {
//     comment: String,
// }

// //allow provider or business to reply to a review

// pub async fn reply_review(
//     State(pool): State<PgPool>,
//     CurrentUser { user_id }: CurrentUser,
//     Json(payload): Json<ReplyReview>,
// ) -> impl IntoResponse {
//     // Validate the payload
//     if payload.comment.is_empty() {
//         return (
//             StatusCode::BAD_REQUEST,
//             Json(json!({ "message": "Reply comment cannot be empty" })),
//         );
//     }

//     // Get the user ID
//     let user_id = user_id.parse::<i32>().unwrap_or(0);

//     // Insert the reply into the database
//     let result = sqlx::query!(
//         "INSERT INTO review_replies (reviewer_id, comment) VALUES ($1, $2) RETURNING id",
//         user_id,
//         payload.comment
//     )
//     .fetch_one(&pool)
//     .await;

//     match result {
//         Ok(record) => (
//             StatusCode::CREATED,
//             Json(json!({ "message": "Reply created successfully", "reply_id": record.id })),
//         ),
//         Err(e) => (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             Json(json!({ "message": format!("Failed to create reply: {}", e) })),
//         ),
//     }
// }
