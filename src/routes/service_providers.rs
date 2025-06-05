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
) -> impl IntoResponse {
    let mut query = String::from(
        r#"
        SELECT 
            p.id, p.service_name, p.category, p.location, p.email, p.phone_number, p.website
        FROM providers p
        JOIN users u ON p.user_id = u.id
        WHERE 1=1
        "#,
    );

    let mut bindings: Vec<String> = Vec::new();
    let mut param_index = 1;
  

    if let Some(ref category) = params.category {
        query.push_str(&format!(" AND p.category = ${}", param_index));
        param_index += 1;
        bindings.push(category.to_string());
    }

    if let Some(ref location) = params.location {
        query.push_str(&format!(" AND p.location = ${}", param_index));
        bindings.push(location.to_string());
    }

    // Prepare query
    let mut q = sqlx::query_as::<_, PublicProvider>(&query);
    for bind in bindings {
        q = q.bind(bind);
    }

   
  

    // Execute
    match q.fetch_all(&pool).await {
        Ok(bindings) => Json(json!({
            "status": "success",
            "providers": bindings
                .into_iter()
                .map(|p| json!({
                    "id": p.id,
                    "service_name": p.service_name,
                    "category": p.category,
                    "location": p.location,
                    "email": p.email,
                    "phone_number": p.phone_number,
                    "website": p.website
                }))
                .collect::<Vec<_>>()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}


#[derive(Deserialize, Debug, Validate)]
pub struct UpdateProviderProfileRequest{
    #[validate(length(min = 3))]
    pub service_name: Option<String>,
    #[validate(length(min = 10))]
    pub service_description: Option<String>,
    pub location: Option<String>,
    #[validate(length(min = 10))]
    pub phone_number: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
}

pub async fn update_provider_profile(
    CurrentUser {user_id}: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateProviderProfileRequest>,
)-> impl IntoResponse { 
       if let Err(e) = payload.validate() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})));
     }

      let mut query = String::from(
    "UPDATE providers SET "
      );
      let mut updates = vec![];
      let mut bindings: Vec<String> = Vec::new();
      let mut idx = 1;

      if let Some(ref value) = payload.service_name {
        updates.push(format!("service_name = ${}", idx));
        bindings.push(value.clone());
        idx += 1;
      }

        if let Some(ref value) = payload.service_description {
            updates.push(format!("service_description = ${}", idx));
            bindings.push(value.clone());
            idx += 1;
        }

       
        if let Some(ref value) = payload.location {
            updates.push(format!("location = ${}", idx));
            bindings.push(value.clone());
            idx += 1;
        }

        if let Some(ref value) = payload.phone_number {
            updates.push(format!("phone_number = ${}", idx));
            bindings.push(value.clone());
            idx += 1;
        }

        if let Some(ref value) = payload.email {
            updates.push(format!("email = ${}", idx));
            bindings.push(value.clone());
            idx += 1;
        }

        if let Some(ref value) = payload.website {
            updates.push(format!("website = ${}", idx));
            bindings.push(value.clone());
            idx += 1;
        }

        if let Some(ref value) = payload.whatsapp {
            updates.push(format!("whatsapp = ${}", idx));
            bindings.push(value.clone());
            idx += 1;
        }
        if updates.is_empty() {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": "No fields to update"})));
        }

        query.push_str(&updates.join(", "));
        query.push_str(&format!("WHERE user_id = ${}", idx));
        bindings.push(user_id.to_string());

        let mut q = sqlx::query(&query);
        for b in bindings{
            q = q.bind(b);
        }

        match q.execute(&pool).await {
            Ok(_) => {
                (StatusCode::OK, Json(json!({"message": "Profile updated successfully"})))
            },
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
            }
        }

}

