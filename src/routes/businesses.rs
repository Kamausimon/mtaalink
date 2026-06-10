use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::parse_image_from_multipart;
use crate::utils::storage::{SharedStorage, generate_key};
use axum::{
    Extension, Json, Router,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn businesses_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/onboard", post(onboard_business))
        .route("/listBusinesses", get(list_businesses))
        .route("/:id", get(get_business_public_profile))
        .route("/updateProfile", post(update_business_profile))
        .route("/uploadLogo", post(upload_business_logo))
        .route("/uploadProfilePicture", post(upload_business_profile_picture))
        .route("/uploadCoverPhoto", post(upload_business_cover_photo))
        .with_state(pool)
}

#[derive(Deserialize, Debug, Validate)]
pub struct BusinessOnboardRequest {
    #[validate(length(min = 3))]
    pub business_name: String,
    #[validate(length(min = 10))]
    pub description: String,
    pub category: Option<String>,
    pub location: Option<String>,
    pub license_number: String,
    #[validate(length(min = 11))]
    pub krapin: String,
    #[validate(length(min = 10))]
    pub phone_number: String,
    #[validate(email)]
    pub email: String,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
    pub profile_photo: Option<String>,
}

pub async fn onboard_business(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<BusinessOnboardRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let exists = sqlx::query_scalar!(
        "SELECT 1 FROM businesses WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::BadRequest(
            "Business not found. Please use the onboard endpoint to register first.".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;

    let record = sqlx::query!(
        r#"UPDATE businesses SET
            business_name = $1,
            description = $2,
            category = $3,
            location = $4,
            license_number = $5,
            krapin = $6,
            phone_number = $7,
            email = $8,
            website = $9,
            whatsapp = $10,
            profile_photo = COALESCE($11, profile_photo),
            onboarding_completed = TRUE
         WHERE user_id = $12 RETURNING id"#,
        payload.business_name,
        payload.description,
        payload.category,
        payload.location,
        payload.license_number,
        payload.krapin,
        payload.phone_number,
        payload.email,
        payload.website,
        payload.whatsapp,
        payload.profile_photo,
        user_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update business: {}", e)))?;

    tx.commit().await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Business onboarded successfully", "business_id": record.id })),
    ))
}

#[derive(Deserialize, Debug)]
pub struct BusinessQuery {
    pub category: Option<String>,
    pub business_name: Option<String>,
    pub location: Option<String>,
}

#[derive(Serialize, Debug, sqlx::FromRow)]
struct BusinessRecord {
    pub id: i32,
    pub business_name: String,
    pub description: String,
    pub category: Option<String>,
    pub location: Option<String>,
    pub phone_number: String,
    pub email: String,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
}

pub async fn list_businesses(
    State(pool): State<PgPool>,
    Query(params): Query<BusinessQuery>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let role = sqlx::query_scalar!(
        "SELECT role FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .flatten()
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if role == "business" || role == "provider" {
        return Err(AppError::Forbidden(
            "You are not authorized to view this resource".to_string(),
        ));
    }

    let mut query = String::from(
        "SELECT b.id, b.business_name, b.description, b.category, b.location, \
         b.phone_number, b.email, b.website, b.whatsapp \
         FROM businesses b JOIN users u ON b.user_id = u.id \
         WHERE b.onboarding_completed = TRUE",
    );
    let mut bindings: Vec<String> = Vec::new();
    let mut param_index = 1;

    if let Some(ref category) = params.category {
        query.push_str(&format!(" AND b.category = ${}", param_index));
        param_index += 1;
        bindings.push(category.clone());
    }
    if let Some(ref name) = params.business_name {
        query.push_str(&format!(" AND b.business_name ILIKE ${}", param_index));
        param_index += 1;
        bindings.push(format!("%{}%", name));
    }
    if let Some(ref location) = params.location {
        query.push_str(&format!(" AND b.location ILIKE ${}", param_index));
        bindings.push(format!("%{}%", location));
    }

    let mut q = sqlx::query_as::<_, BusinessRecord>(&query);
    for bind in bindings {
        q = q.bind(bind);
    }

    let businesses = q.fetch_all(&pool).await.map_err(AppError::Database)?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Businesses fetched successfully", "businesses": businesses })),
    ))
}

#[derive(Deserialize, Debug, Validate)]
pub struct BusinessUpdateRequest {
    #[validate(length(min = 10))]
    pub description: Option<String>,
    #[validate(length(min = 10))]
    pub phone_number: Option<String>,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
    pub profile_photo: Option<String>,
}

pub async fn update_business_profile(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<BusinessUpdateRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let role = sqlx::query_scalar!(
        "SELECT role FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .flatten()
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if role != "business" {
        return Err(AppError::Forbidden(
            "You are not authorized to update this business".to_string(),
        ));
    }

    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let mut query = String::from("UPDATE businesses SET ");
    let mut updates: Vec<String> = Vec::new();
    let mut bindings: Vec<String> = Vec::new();
    let mut idx = 1;

    if let Some(ref v) = payload.description {
        updates.push(format!("description = ${}", idx));
        bindings.push(v.clone());
        idx += 1;
    }
    if let Some(ref v) = payload.phone_number {
        updates.push(format!("phone_number = ${}", idx));
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
    if let Some(ref v) = payload.profile_photo {
        updates.push(format!("profile_photo = ${}", idx));
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

    Ok((StatusCode::OK, Json(json!({ "message": "Business profile updated successfully" }))))
}

async fn check_business_role(pool: &PgPool, user_id: i32) -> AppResult<()> {
    let role = sqlx::query_scalar!("SELECT role FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await?
        .flatten()
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if role != "business" {
        return Err(AppError::Forbidden(
            "You are not authorized to perform this action".to_string(),
        ));
    }
    Ok(())
}

pub async fn upload_business_logo(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Extension(storage): Extension<SharedStorage>,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    check_business_role(&pool, user_id).await?;

    let (data, ext, _ct) = parse_image_from_multipart(multipart).await?;
    let key = generate_key("businesses/logos", &ext);
    let url = storage.save(&key, &data).await?;

    let result = sqlx::query!(
        "UPDATE businesses SET logo = $1 WHERE user_id = $2",
        url, user_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        let _ = storage.delete(&key).await;
        return Err(AppError::Database(e));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Logo uploaded successfully", "logo": url }))))
}

pub async fn upload_business_profile_picture(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Extension(storage): Extension<SharedStorage>,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let (data, ext, _ct) = parse_image_from_multipart(multipart).await?;
    let key = generate_key("businesses/profile_pictures", &ext);
    let url = storage.save(&key, &data).await?;

    let result = sqlx::query!(
        "UPDATE businesses SET profile_photo = $1 WHERE user_id = $2",
        url, user_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        let _ = storage.delete(&key).await;
        return Err(AppError::Database(e));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Profile picture uploaded successfully", "profile_picture": url }))))
}

pub async fn upload_business_cover_photo(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Extension(storage): Extension<SharedStorage>,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let (data, ext, _ct) = parse_image_from_multipart(multipart).await?;
    let key = generate_key("businesses/cover_photos", &ext);
    let url = storage.save(&key, &data).await?;

    let result = sqlx::query!(
        "UPDATE businesses SET cover_photo = $1 WHERE user_id = $2",
        url, user_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        let _ = storage.delete(&key).await;
        return Err(AppError::Database(e));
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Cover photo uploaded successfully", "cover_photo": url }))))
}

// ── Public profile ────────────────────────────────────────────────────────────

#[derive(Serialize, Debug, sqlx::FromRow)]
struct BusinessPublicProfile {
    id: i32,
    business_name: Option<String>,
    description: Option<String>,
    category: Option<String>,
    location: Option<String>,
    phone_number: Option<String>,
    email: Option<String>,
    website: Option<String>,
    whatsapp: Option<String>,
    logo: Option<String>,
    profile_photo: Option<String>,
    cover_photo: Option<String>,
    onboarding_completed: bool,
    avg_rating: Option<f64>,
    review_count: Option<i64>,
}

pub async fn get_business_public_profile(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let profile = sqlx::query_as::<_, BusinessPublicProfile>(
        r#"SELECT b.id, b.business_name, b.description, b.category, b.location,
                  b.phone_number, b.email, b.website, b.whatsapp,
                  b.logo, b.profile_photo, b.cover_photo, b.onboarding_completed,
                  ROUND(AVG(r.rating)::numeric, 1)::float8 AS avg_rating,
                  COUNT(r.id) AS review_count
           FROM businesses b
           LEFT JOIN reviews r ON r.target_id = b.id AND r.target_type = 'business'
           WHERE b.id = $1
           GROUP BY b.id"#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Business not found".to_string()))?;

    // Fetch their active services
    let services = sqlx::query!(
        r#"SELECT id, title, description, price, duration, category_id
           FROM services
           WHERE target_type = 'business' AND target_id = $1 AND is_active = true
           ORDER BY id"#,
        id
    )
    .fetch_all(&pool)
    .await?;

    let services_json: Vec<serde_json::Value> = services
        .into_iter()
        .map(|s| json!({
            "id": s.id,
            "title": s.title,
            "description": s.description,
            "price": s.price,
            "duration": s.duration,
            "category_id": s.category_id,
        }))
        .collect();

    // Fetch branch locations
    let branches = sqlx::query!(
        r#"SELECT bb.id, bb.name, bb.address, bb.phone, bb.latitude, bb.longitude,
                  w.name AS ward_name, c.name AS constituency_name, co.name AS county_name
           FROM business_branches bb
           JOIN wards w ON bb.ward_id = w.id
           JOIN constituencies c ON w.constituency_id = c.id
           JOIN counties co ON c.county_id = co.id
           WHERE bb.business_id = $1"#,
        id
    )
    .fetch_all(&pool)
    .await?;

    let branches_json: Vec<serde_json::Value> = branches
        .into_iter()
        .map(|b| json!({
            "id": b.id,
            "name": b.name,
            "address": b.address,
            "phone": b.phone,
            "latitude": b.latitude,
            "longitude": b.longitude,
            "ward": b.ward_name,
            "constituency": b.constituency_name,
            "county": b.county_name,
        }))
        .collect();

    Ok((StatusCode::OK, Json(json!({
        "business": profile,
        "services": services_json,
        "branches": branches_json,
    }))))
}
