use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::parse_image_from_multipart;
use crate::utils::storage::{SharedStorage, generate_key};
use axum::{
    Extension, Json, Router,
    extract::{Multipart, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
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
        .with_state(pool)
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
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let exists = sqlx::query_scalar!(
        "SELECT 1 FROM providers WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::BadRequest(
            "Provider profile not found. Please register as a provider first.".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;

    let record = sqlx::query!(
        "UPDATE providers SET (
             service_name, service_description, category, location,
             phone_number, email, website, whatsapp
         ) = ($1, $2, $3, $4, $5, $6, $7, $8)
         WHERE user_id = $9 RETURNING id",
        payload.service_name,
        payload.service_description,
        payload.category,
        payload.location,
        payload.phone_number,
        payload.email,
        payload.website,
        payload.whatsapp,
        user_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        AppError::Internal(format!("Failed to update provider: {}", e))
    })?;

    tx.commit().await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Provider profile updated successfully", "provider_id": record.id })),
    ))
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
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut query = String::from(
        "SELECT p.id, p.service_name, p.category, p.location, p.email, p.phone_number, p.website \
         FROM providers p JOIN users u ON p.user_id = u.id WHERE 1=1",
    );

    let mut bindings: Vec<String> = Vec::new();
    let mut param_index = 1;

    if let Some(ref category) = params.category {
        query.push_str(&format!(" AND p.category = ${}", param_index));
        param_index += 1;
        bindings.push(category.clone());
    }
    if let Some(ref location) = params.location {
        query.push_str(&format!(" AND p.location = ${}", param_index));
        bindings.push(location.clone());
    }

    let mut q = sqlx::query_as::<_, PublicProvider>(&query);
    for bind in bindings {
        q = q.bind(bind);
    }

    let providers = q.fetch_all(&pool).await.map_err(AppError::Database)?;

    Ok((StatusCode::OK, Json(json!({ "providers": providers }))))
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
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let mut query = String::from("UPDATE providers SET ");
    let mut updates = vec![];
    let mut bindings: Vec<String> = Vec::new();
    let mut idx = 1;

    if let Some(ref v) = payload.service_name {
        updates.push(format!("service_name = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }
    if let Some(ref v) = payload.service_description {
        updates.push(format!("service_description = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }
    if let Some(ref v) = payload.location {
        updates.push(format!("location = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }
    if let Some(ref v) = payload.phone_number {
        updates.push(format!("phone_number = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }
    if let Some(ref v) = payload.email {
        updates.push(format!("email = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }
    if let Some(ref v) = payload.website {
        updates.push(format!("website = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }
    if let Some(ref v) = payload.whatsapp {
        updates.push(format!("whatsapp = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!(" WHERE user_id = ${}", idx));

    let mut q = sqlx::query(&query);
    for b in bindings {
        q = q.bind(b);
    }
    q = q.bind(user_id);

    q.execute(&pool).await.map_err(AppError::Database)?;

    Ok((StatusCode::OK, Json(json!({ "message": "Profile updated successfully" }))))
}

pub async fn upload_provider_profile_photo(
    State(pool): State<PgPool>,
    Extension(storage): Extension<SharedStorage>,
    CurrentUser { user_id }: CurrentUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let (data, ext, _ct) = parse_image_from_multipart(multipart).await?;
    let key = generate_key("providers/profile_photos", &ext);
    let url = storage.save(&key, &data).await?;

    let result = sqlx::query!(
        "UPDATE providers SET profile_photo = $1 WHERE user_id = $2",
        url, user_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        let _ = storage.delete(&key).await;
        return Err(AppError::Database(e));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Profile photo uploaded successfully", "url": url }))))
}

pub async fn upload_provider_cover_photo(
    State(pool): State<PgPool>,
    Extension(storage): Extension<SharedStorage>,
    CurrentUser { user_id }: CurrentUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let (data, ext, _ct) = parse_image_from_multipart(multipart).await?;
    let key = generate_key("providers/cover_photos", &ext);
    let url = storage.save(&key, &data).await?;

    let result = sqlx::query!(
        "UPDATE providers SET cover_photo = $1 WHERE user_id = $2",
        url, user_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        let _ = storage.delete(&key).await;
        return Err(AppError::Database(e));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Cover photo uploaded successfully", "url": url }))))
}

#[derive(Serialize, Debug, sqlx::FromRow)]
pub struct ProviderData {
    id: i32,
    service_name: Option<String>,
    service_description: Option<String>,
    category: Option<String>,
    location: Option<String>,
    phone_number: Option<String>,
    email: Option<String>,
    website: Option<String>,
    whatsapp: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GetProviderDataQuery {
    pub provider_id: i32,
}

pub async fn get_provider_data(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<GetProviderDataQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if params.provider_id != user_id {
        return Err(AppError::Forbidden(
            "You are not authorized to access this data".to_string(),
        ));
    }

    let provider = sqlx::query_as!(
        ProviderData,
        "SELECT id, service_name, service_description, category, location, \
         phone_number, email, website, whatsapp FROM providers WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?;

    match provider {
        Some(data) => Ok((StatusCode::OK, Json(json!({ "provider_data": data })))),
        None => Err(AppError::NotFound("Provider not found".to_string())),
    }
}

#[derive(Deserialize, Debug, Serialize, sqlx::FromRow)]
pub struct ProviderAvailability {
    pub provider_id: i32,
    pub is_available: bool,
    pub day: String,
    pub start_time: String,
    pub end_time: String,
}

pub async fn update_provider_availability(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<ProviderAvailability>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.provider_id != user_id {
        return Err(AppError::Forbidden(
            "You are not authorized to update this provider's availability".to_string(),
        ));
    }

    let start_time = NaiveTime::parse_from_str(&payload.start_time, "%H:%M")
        .map_err(|e| AppError::BadRequest(format!("Invalid start time format: {}", e)))?;
    let end_time = NaiveTime::parse_from_str(&payload.end_time, "%H:%M")
        .map_err(|e| AppError::BadRequest(format!("Invalid end time format: {}", e)))?;

    let provider_exists = sqlx::query_scalar!(
        "SELECT 1 FROM providers WHERE id = $1",
        payload.provider_id
    )
    .fetch_optional(&pool)
    .await?;

    if provider_exists.is_none() {
        return Err(AppError::NotFound("Provider not found".to_string()));
    }

    let availability_exists = sqlx::query_scalar!(
        "SELECT 1 FROM provider_availability WHERE provider_id = $1 AND day = $2",
        payload.provider_id,
        payload.day
    )
    .fetch_optional(&pool)
    .await?;

    if availability_exists.is_some() {
        sqlx::query!(
            "UPDATE provider_availability SET is_available = $1, start_time = $2, end_time = $3 \
             WHERE provider_id = $4 AND day = $5",
            payload.is_available,
            start_time,
            end_time,
            payload.provider_id,
            payload.day
        )
        .execute(&pool)
        .await?;

        Ok((StatusCode::OK, Json(json!({ "message": "Availability updated successfully" }))))
    } else {
        sqlx::query!(
            "INSERT INTO provider_availability (provider_id, is_available, day, start_time, end_time) \
             VALUES ($1, $2, $3, $4, $5)",
            payload.provider_id,
            payload.is_available,
            payload.day,
            start_time,
            end_time
        )
        .execute(&pool)
        .await?;

        Ok((StatusCode::CREATED, Json(json!({ "message": "Availability created successfully" }))))
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AvailabilityItem {
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
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let provider = sqlx::query_scalar!(
        "SELECT id FROM providers WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?;

    let provider_id = provider.ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    let mut tx = pool.begin().await?;
    let mut updated_count = 0u32;
    let mut created_count = 0u32;

    for item in payload.availability {
        let start_time = NaiveTime::parse_from_str(&item.start_time, "%H:%M")
            .map_err(|_| AppError::BadRequest(format!("Invalid start time for {}: use HH:MM", item.day)))?;
        let end_time = NaiveTime::parse_from_str(&item.end_time, "%H:%M")
            .map_err(|_| AppError::BadRequest(format!("Invalid end time for {}: use HH:MM", item.day)))?;

        let existing = sqlx::query_scalar!(
            "SELECT id FROM provider_availability WHERE provider_id = $1 AND day = $2",
            provider_id,
            item.day
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(record_id) = existing {
            sqlx::query!(
                "UPDATE provider_availability SET is_available = $1, start_time = $2, end_time = $3 \
                 WHERE id = $4 AND provider_id = $5",
                item.is_available,
                start_time,
                end_time,
                record_id,
                provider_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update {}: {}", item.day, e)))?;
            updated_count += 1;
        } else {
            sqlx::query!(
                "INSERT INTO provider_availability (provider_id, is_available, day, start_time, end_time) \
                 VALUES ($1, $2, $3, $4, $5)",
                provider_id,
                item.is_available,
                item.day,
                start_time,
                end_time
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create {}: {}", item.day, e)))?;
            created_count += 1;
        }
    }

    tx.commit().await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "message": "Availability updated successfully",
            "updated": updated_count,
            "created": created_count
        })),
    ))
}

pub async fn get_provider_availability(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let provider_id = sqlx::query_scalar!(
        "SELECT id FROM providers WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    let records = sqlx::query!(
        "SELECT id, day, start_time, end_time, is_available \
         FROM provider_availability \
         WHERE provider_id = $1 \
         ORDER BY CASE \
            WHEN day = 'monday' THEN 1 \
            WHEN day = 'tuesday' THEN 2 \
            WHEN day = 'wednesday' THEN 3 \
            WHEN day = 'thursday' THEN 4 \
            WHEN day = 'friday' THEN 5 \
            WHEN day = 'saturday' THEN 6 \
            WHEN day = 'sunday' THEN 7 \
            ELSE 8 \
         END",
        provider_id
    )
    .fetch_all(&pool)
    .await?;

    let availability: Vec<_> = records
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "day": r.day,
                "start_time": r.start_time,
                "end_time": r.end_time,
                "is_available": r.is_available
            })
        })
        .collect();

    Ok((StatusCode::OK, Json(json!({ "availability": availability }))))
}
