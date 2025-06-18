use axum::{
    extract::{State, Query, Json, },
    response::IntoResponse,
    http::StatusCode,
    Router,
    routing::{get, post}
};
use serde_json::json;
use sqlx::PgPool;
use crate::extractors::administrator::require_admin;
use crate::extractors::current_user::CurrentUser;

pub fn admin_routes(pool: PgPool)-> Router {
    Router::new()
        .route("/categories", get(get_categories))
        .route("/create_category", post(create_category))
        .route("/delete_category", post(delete_category))
        .route("/users", get(get_users))
        .route("/delete_user", post(delete_user))
        .route("/userAnalytics", get(get_user_analytics))
        .route("/flagContent", post(flag_content))
        .route("/resolveFlag", post(resolve_flag))
        .route("/moderateReviews", get(moderate_reviews))
        .layer(axum::middleware::from_fn(require_admin))
        .with_state(pool)
}