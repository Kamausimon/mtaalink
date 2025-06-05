use axum::{
    Router,
    extract::{Json, State},
    response::IntoResponse,
    routing::{get,post},
    http::StatusCode,
};
use sqlx::{PgPool};
use serde_json::json;
use crate::extractors::current_user::CurrentUser;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub fn service_providers_routes(pool:PgPool) -> Router {
    Router::new()
        .route("/onboard", post(onboard_service_provider))
        .with_state(pool.clone())
}

#[derive(Deserialize, Debug, Validate)]
pub struct ProviderOnboardRequest{
    #[validate(length(min = 3))] 
    pub service_name: String,
    #[validate(length(min = 10))]
    pub service_description: String,

    pub category: String,
    pub location: String,
    #[validate(length(min = 10))]
    pub phone_number: String,
    #[validate(email)]
    pub email: String,
    pub website: Option<String>,
    pub whatsapp: Option<String>,

}

pub async fn onboard_service_provider(
    CurrentUser {user_id}: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<ProviderOnboardRequest>,
) -> impl IntoResponse{
     if let Err(e) = payload.validate() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})));
     }
    
       let exists = sqlx::query_scalar!(
        "SELECT 1 FROM providers WHERE user_id = $1 ",
        user_id.parse::<i32>().unwrap()
       ).fetch_optional(&pool).await.unwrap();

       if exists.is_some(){
        return (StatusCode::CONFLICT, Json(json!({"error": "Service provider already onboarded"})));
       }

       let result = sqlx::query!(
         "INSERT INTO providers (user_id,service_name, service_description, category, location) 
          VALUES ($1, $2, $3, $4, $5) RETURNING id",
          user_id.parse::<i32>().unwrap(),
          payload.service_name,
          payload.service_description,
          payload.category,
          payload.location
          payload.phone_number,
          payload.email,
            payload.website,
            payload.whatsapp
       ).fetch_one(&pool).await;

          match result {
            Ok(_) => {
                (StatusCode::CREATED, Json(json!({"message": "Service provider onboarded successfully"})))
            },
            Err(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to onboard service provider"})))
            }
          }
    }