use axum::{
    Router,
    extract::{Json, State},
    response::IntoResponse,
    routing::{get,post},
    http::StatusCode,
    extract::Query,
};
use sqlx::{PgPool};
use sqlx::QueryBuilder;
use serde_json::json;
use crate::extractors::current_user::CurrentUser;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub fn service_providers_routes(pool:PgPool) -> Router {
    Router::new()
        .route("/onboard", post(onboard_service_provider))
        .route("/listProviders", get(list_providers))
        .with_state(pool.clone())
}

#[derive(Deserialize, Debug, Validate)]
pub struct ProviderOnboardRequest{
    #[validate(length(min = 3))] 
    pub service_name: String,
    #[validate(length(min = 10))]
    pub service_description: String,

    pub category: Option<String>,
    pub location: Option<String>,
    #[validate(length(min = 10))]
    pub phone_number: Option<String>,
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
         "INSERT INTO providers (user_id,service_name, service_description, category, location, phone_number, email, website, whatsapp) 
          VALUES ($1, $2, $3, $4, $5, $6, $7,$8,$9) RETURNING id",
          user_id.parse::<i32>().unwrap(),
          payload.service_name,
          payload.service_description,
          payload.category,
          payload.location,
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

    #[derive(Deserialize, Debug)]
    pub struct ProviderQuery{
    pub category: Option<String>,
    pub location: Option<String>,
    }

      #[derive(Serialize, Debug, sqlx::FromRow)]
    struct PublicProvider{
     id: i32,
     service_name: String,
     category : Option<String>,
     location: Option<String>,
     email: Option<String>,
        phone_number: Option<String>,
        website: Option<String>,
    }

    pub async fn list_providers(
        State(pool): State<PgPool>,
        Query(params): Query<ProviderQuery>,
    )-> impl IntoResponse{
        //  let mut query = String::from(
        //     r#"
        //     SELECT 
        //     p.id, p.service_name, p.category, p.location, p.email, p.phone_number, p.website
        //     FROM providers p
        //      JOIN users u on p.user_id = u.id
        //      where 1=1
        //      "#,
        //  );

        //  let mut bindings = Vec::new();

        //  if let Some(ref category) = params.category{
        //     query.push_str("AND p.category = $1 ");
        //     bindings.push(category.to_string());
        //  }

        //  if let Some(ref location) = params.location{
        //     if bindings.is_empty(){
        //         query.push_str("AND p.location = $1 ");
        //     } else {
        //         query.push_str("AND p.location = $2 ");
        //     }
        //     bindings.push(location.to_string());
        //  }

        //  let providers = match bindings.len(){
        //     0 => sqlx::query_as::<_, PublicProvider>(&query)
        //         .fetch_all(&pool)
        //         .await,
        //     1 => sqlx::query_as::<_, PublicProvider>(&query)
        //         .bind(&bindings[0])
        //         .fetch_all(&pool)
        //         .await,
        //     2 => sqlx::query_as::<_, PublicProvider>(&query)
        //         .bind(&bindings[0])
        //         .bind(&bindings[1])
        //         .fetch_all(&pool)
        //         .await,
        //     _ => unreachable!(),
        //  };

        let mut builder = QueryBuilder::new("SELECT * FROM providers WHERE 1=1" );

        if let Some(category) = &params.category{
            builder.push("AND category = ").
                push_bind(category);
        }

        if let Some(location) = &params.location{
            builder.push("AND location = ").
                push_bind(location);
        }

        let query = builder.build_query_as::<PublicProvider>();
        let providers_result = query.fetch_all(&pool).await;

         match providers_result {
    Ok(providers) => Json(json!({
        "status": "success",
        "providers": providers
    })),
    Err(e) => Json(json!({
        "status": "error",
        "message": e.to_string()
    })),
}
    }

