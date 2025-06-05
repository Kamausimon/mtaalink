use axum::{
    Router,
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    http::StatusCode,
};
use sqlx::PgPool;
use serde_json::json;
use crate::extractors::current_user::CurrentUser;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub fn businesses_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/onboard", post(onboard_business))
        .with_state(pool.clone())
}

#[derive(Deserialize, Debug, Validate)]
pub struct BusinessOnboardRequest {
    #[validate(length(min = 3))]
    pub business_name: String,
    #[validate(length(min = 10))]
    pub description: String,
    pub category: String,
    pub location: String,
    pub license_number: String,
    #[validate(length(min = ))]
    pub krapin: String,
      #[validate(length(min = 10))]
    pub phone_number: String,
    #[validate(email)]
    pub email: String,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
}

pub async fn onboard_business(
  CurrentUser {user_id}: CurrentUser,
  State(Pool) : State<PgPool>,
  Json(payload): Json<BusinessOnboardRequest>,
)-> impl IntoResponse{
    if let Err(e) = payload.validate() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})));
    }

    let exists = sqlx::query_scalar!(
        "SELECT 1 FROM businesses WHERE user_id = $1",
        user_id.parse::<i32>().unwrap()
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    if exists.is_some() {
        return (StatusCode::CONFLICT, Json(json!({"error": "Business already onboarded"})));
    }

    let result = sqlx::query!(
        "INSERT INTO businesses (user_id, business_name, description, category, location, license_number, krapin) 
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        user_id.parse::<i32>().unwrap(),
        payload.business_name,
        payload.description,
        payload.category,
        payload.location,
        payload.license_number,
        payload.krapin
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(record) => (StatusCode::CREATED, Json(json!({"id": record.id}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    }
}