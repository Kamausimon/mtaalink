use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::save_image_to_fs;
use axum::{
    Router,
    extract::Query,
    extract::{Json, Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use sqlx::{Postgres, Transaction};
use validator::Validate;

pub fn service_providers_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/onboard", post(onboard_service_provider))
        .route("/listProviders", get(list_providers))
        .route("/updateProfile", post(update_provider_profile))
        .route("/uploadProfilePhoto", post(upload_provider_profile_photo))
        .route("/uploadCoverPhoto", post(upload_provider_cover_photo))
        .route("/getProviderData", get(get_provider_data))
        .with_state(pool.clone())
}

#[derive(Deserialize, Debug, Validate, sqlx::FromRow)]
pub struct ProviderOnboardRequest {
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
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<ProviderOnboardRequest>,
) -> impl IntoResponse {
    let mut tx: Transaction<'_, Postgres> = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database connection error"})),
            );
        }
    };

    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        );
    }

    let exists = sqlx::query_scalar!(
        "SELECT 1 FROM providers WHERE user_id = $1 ",
        user_id.parse::<i32>().unwrap()
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    if let Some(_) = exists {
        //if the business exists continue to update the profile
        let result = sqlx::query!(
            "UPDATE providers SET(
             service_name, service_description,category,location,phone_number,email,website,whatsapp) = 
             ($1, $2, $3, $4, $5, $6, $7,$8) WHERE user_id = $9 RETURNING id",
            payload.service_name,
            payload.service_description,
            payload.category,
            payload.location,
            payload.phone_number,
            payload.email,
            payload.website,
            payload.whatsapp,
            user_id.parse::<i32>().unwrap()
        )
        .fetch_one(&mut *tx)
        .await;
        println!("Update result: {:?}", result);

        //if the update fails rollback the transaction
        if let Err(e) = result {
            let _ = tx.rollback().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            );
        }

        //commit the transaction if the update is successful
        if let Err(e) = tx.commit().await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            );
        }

        match result {
            Ok(record) => (
                StatusCode::CREATED,
                Json(
                    json!({"message": "Business onboarded successfully", "provider_id": record.id}),
                ),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            ),
        }
    } else {
        //if the provider does not exist, return an error
        let _ = tx.rollback().await;
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Provider does not exist, please use the onboarding route"})),
        );
    }
}

#[derive(Deserialize, Debug)]
pub struct ProviderQuery {
    pub category: Option<String>,
    pub location: Option<String>,
}

#[derive(Serialize, Debug, sqlx::FromRow)]
struct PublicProvider {
    id: i32,
    service_name: String,
    category: Option<String>,
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
pub struct UpdateProviderProfileRequest {
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
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateProviderProfileRequest>,
) -> impl IntoResponse {
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        );
    }

    println!("payload: {:?}", payload);

    let mut query = String::from("UPDATE providers SET ");
    let mut updates = vec![];
    let mut bindings: Vec<String> = Vec::new();
    let mut idx = 1;
    println!("updates: {:?}", updates);
    println!("bindings: {:?}", bindings);

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
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No fields to update"})),
        );
    }

    query.push_str(&updates.join(", ")); // Join updates with commas
    query.push_str(&format!(" WHERE user_id = ${}", idx)); // Add the user_id condition
    let user_id: i32 = user_id.parse().unwrap(); // Ensure user_id is an i32

    let mut q = sqlx::query(&query);
    for b in bindings {
        q = q.bind(b);
    }

    q = q.bind(user_id); // Bind the user_id at the end

    match q.execute(&pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({"message": "Profile updated successfully"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

pub async fn upload_provider_profile_photo(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    multipart: Multipart,
) -> impl IntoResponse {
    let dir = "uploads/providers/profile_photos";

    match save_image_to_fs(multipart, dir).await {
        Ok(file_name) => {
            let file_url = format!("/uploads/providers/profile_photos/{}", file_name);
            println!("File URL: {}", file_url);

            let _ = sqlx::query!(
                "UPDATE providers SET profile_photo = $1 WHERE user_id = $2",
                file_url,
                user_id.parse::<i32>().unwrap()
            )
            .execute(&pool)
            .await;

            (
                StatusCode::OK,
                Json(json!({
                    "message": "Profile photo uploaded successfully",
                    "url": file_url
                })),
            )
        }

        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Failed to upload profile photo",
                "details": e
            })),
        ),
    }
}

pub async fn upload_provider_cover_photo(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    multipart: Multipart,
) -> impl IntoResponse {
    let dir = "uploads/providers/cover_photos";

    match save_image_to_fs(multipart, dir).await {
        Ok(file_name) => {
            let file_url = format!("/uploads/providers/cover_photos/{}", file_name);

            let _ = sqlx::query!(
                "UPDATE providers SET cover_photo = $1 WHERE user_id = $2",
                file_url,
                user_id.parse::<i32>().unwrap()
            )
            .execute(&pool)
            .await;

            (
                StatusCode::OK,
                Json(json!({"message": "Cover photo uploaded successfully", "url": file_url})),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

#[derive(Serialize, Debug, sqlx::FromRow)]
pub struct ProviderData {
    id: i32,
    service_name: String,
    service_description: String,
    category: Option<String>,
    location: Option<String>,
    phone_number: Option<String>,
    email: String,
    website: Option<String>,
    whatsapp: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GetProviderDataQuery {
    pub provider_id: String,
}

pub async fn get_provider_data (
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<GetProviderDataQuery>,
)-> impl IntoResponse {
     let user_id = user_id.parse::<i32>().unwrap_or(0);
    //check if the provider_id in the query matches the current user
        if params.provider_id.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Provider ID is required"})),
            );
        }
     //check if the user_id in the query matches the current user
        if params.provider_id != user_id.to_string() {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "You are not authorized to access this data"})),
            );
        }

        //check if the user is a provider 
       let provider_results = sqlx::query!(
          "SELECT * FROM providers WHERE user_id = $1",
            user_id
       ).fetch_optional(&pool).await;

        match provider_results {
            Ok(Some(provider)) => {
                let provider_data = ProviderData {
                    id: provider.id,
                    service_name: provider.service_name,
                    service_description: provider.service_description,
                    category: provider.category,
                    location: provider.location,
                    phone_number: provider.phone_number,
                    email: provider.email,
                    website: provider.website,
                    whatsapp: provider.whatsapp,
                };
                Json(json!({"provider_data": provider_data}))
            }
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Provider not found"})),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            ),
        }
}
