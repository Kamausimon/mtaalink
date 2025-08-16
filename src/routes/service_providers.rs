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
        .route("/updateAvailability", post(update_provider_availability))
        .route("/updateBulkAvailability", post(update_bulk_availability))
        .route("/getAvailability", get(get_provider_availability))
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
    service_name: Option<String>,  // Changed from String to Option<String>
    service_description: Option<String>, // This should also be optional for consistency
    category: Option<String>,
    location: Option<String>,
    phone_number: Option<String>,
    email: Option<String>,  // Make email optional too if it might be
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
         (
            StatusCode::OK,
                    Json(json!({"provider_data": provider_data}))
         )
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

#[derive(Deserialize, Debug, Serialize, sqlx::FromRow)]
pub struct ProviderAvailabilty{
    pub provider_id: i32,
    pub is_available: bool,
    pub day : String,
    pub start_time: String,
    pub end_time: String,
}



pub async fn update_provider_availability(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<ProviderAvailabilty>,
)-> impl IntoResponse {
    //validate the payload 
    if payload.provider_id != user_id.parse::<i32>().unwrap() {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "You are not authorized to update this provider's availability"})),
        );
    }

    //check if the provider exists
    let provider_exists = sqlx::query!(
        "SELECT 1 FROM providers WHERE id = $1",
        payload.provider_id
    )
    .fetch_optional(&pool)
    .await;

    match provider_exists{
        Ok(Some(_))=> {},
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Provider not found"})),
            );
        }
        Err(e)=> {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            );
        }
    }

    //check if the availability already exists
    let availability_exists = sqlx::query!(
        "SELECT 1 FROM provider_availability WHERE provider_id = $1 AND day = $2",
        payload.provider_id,
        payload.day
    ).fetch_optional(&pool).await;

    //if it exists update it if not create it 
    match availability_exists {
        Ok(Some(_)) => {
            //update the availability
            let update_result = sqlx::query!(
                "UPDATE provider_availability SET is_available = $1, start_time = $2, end_time = $3 WHERE provider_id = $4 AND day = $5",
                payload.is_available,
                payload.start_time,
                payload.end_time,
                payload.provider_id,
                payload.day
            )
            .execute(&pool)
            .await;

            match update_result {
                Ok(_) => (
                    StatusCode::OK,
                    Json(json!({"message": "Availability updated successfully"})),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                ),
            }
        }
        Ok(None) => {
            //insert the availability
            let insert_result = sqlx::query!(
                "INSERT INTO provider_availability (provider_id, is_available, day, start_time, end_time) VALUES ($1, $2, $3, $4, $5)",
                payload.provider_id,
                payload.is_available,
                payload.day,
                payload.start_time,
                payload.end_time
            )
            .execute(&pool)
            .await;

            match insert_result {
                Ok(_) => (
                    StatusCode::CREATED,
                    Json(json!({"message": "Availability created successfully"})),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                ),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AvailabilityItem {
    pub id: Option<i32>,  // Optional for new entries
    pub day: String,
    pub start_time: String,
    pub end_time: String,
    pub is_available: bool,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct BulkAvailabilityUpdate {
    pub availability: Vec<AvailabilityItem>,
}

pub async fn update_bulk_availability(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<BulkAvailabilityUpdate>,
) -> impl IntoResponse {
    // Get provider ID from user_id
    let provider_result = sqlx::query!(
        "SELECT id FROM providers WHERE user_id = $1",
        user_id.parse::<i32>().unwrap()
    )
    .fetch_optional(&pool)
    .await;

    let provider_id = match provider_result {
        Ok(Some(provider)) => provider.id,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Provider not found"})),
            )
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        }
    };

    // Start a transaction for bulk operations
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            );
        }
    };

    // Process each availability item
    let mut updated_count = 0;
    let mut created_count = 0;

    for item in payload.availability {
        // For each day, either update or insert
        let availability_exists = sqlx::query!(
            "SELECT id FROM provider_availability WHERE provider_id = $1 AND day = $2",
            provider_id,
            item.day
        )
        .fetch_optional(&mut *tx)
        .await;

        match availability_exists {
            Ok(Some(record)) => {
                // Update existing record
                let update_result = sqlx::query!(
                    "UPDATE provider_availability SET is_available = $1, start_time = $2, end_time = $3 
                     WHERE id = $4 AND provider_id = $5",
                    item.is_available,
                    item.start_time,
                    item.end_time,
                    record.id,
                    provider_id
                )
                .execute(&mut *tx)
                .await;

                if let Err(e) = update_result {
                    let _ = tx.rollback().await;
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Failed to update {}: {}", item.day, e.to_string())})),
                    );
                }
                
                updated_count += 1;
            }
            Ok(None) => {
                // Insert new record
                let insert_result = sqlx::query!(
                    "INSERT INTO provider_availability (provider_id, is_available, day, start_time, end_time) 
                     VALUES ($1, $2, $3, $4, $5)",
                    provider_id,
                    item.is_available,
                    item.day,
                    item.start_time,
                    item.end_time
                )
                .execute(&mut *tx)
                .await;

                if let Err(e) = insert_result {
                    let _ = tx.rollback().await;
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Failed to create {}: {}", item.day, e.to_string())})),
                    );
                }
                
                created_count += 1;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                );
            }
        }
    }

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "message": "Availability updated successfully",
            "updated": updated_count,
            "created": created_count
        })),
    )
}

pub async fn get_provider_availability(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> impl IntoResponse {
    // Get provider ID from user_id
    let provider_result = sqlx::query!(
        "SELECT id FROM providers WHERE user_id = $1",
        user_id.parse::<i32>().unwrap()
    )
    .fetch_optional(&pool)
    .await;

    let provider_id = match provider_result {
        Ok(Some(provider)) => provider.id,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Provider not found"})),
            )
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        }
    };

    // Fetch availability records
    let availability_result = sqlx::query!(
        "SELECT id, day, start_time, end_time, is_available 
         FROM provider_availability 
         WHERE provider_id = $1
         ORDER BY CASE 
            WHEN day = 'monday' THEN 1
            WHEN day = 'tuesday' THEN 2
            WHEN day = 'wednesday' THEN 3
            WHEN day = 'thursday' THEN 4
            WHEN day = 'friday' THEN 5
            WHEN day = 'saturday' THEN 6
            WHEN day = 'sunday' THEN 7
            ELSE 8
         END",
        provider_id
    )
    .fetch_all(&pool)
    .await;

    match availability_result {
        Ok(records) => {
            let availability = records
                .into_iter()
                .map(|record| {
                    json!({
                        "id": record.id,
                        "day": record.day,
                        "start_time": record.start_time,
                        "end_time": record.end_time,
                        "is_available": record.is_available
                    })
                })
                .collect::<Vec<_>>();

            (StatusCode::OK, Json(json!({ "availability": availability })))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}
