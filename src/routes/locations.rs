use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn locations_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/allcounties", get(get_locations_counties))
        .route("/counties/:county_id/constituencies", get(get_constituencies_by_county))
        .route("/constituencies/:constituency_id/wards", get(get_wards_by_constituency))
        .route("/branches/:business_id/location", post(create_branch_location))
        .route("/branches/:business_id/locations", get(get_branch_locations))
        .route("/branches/location/:id", get(get_branch_by_id))
        .route("/branches/location/:id/update", post(update_branch_location))
        .route("/branches/location/:id/delete", post(delete_branch_location))
        .route("/providers/:provider_id", post(create_provider_location))
        .route("/providers/location/:id", get(get_provider_location_by_id))
        .route("/providers/location/:id/update", post(update_provider_location))
        .route("/providers/location/:id/delete", post(delete_provider_location))
        .route("/search", get(search_business_or_provider_by_location))
        .with_state(pool)
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct Counties {
    pub id: i32,
    pub name: String,
}

pub async fn get_locations_counties(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let counties = sqlx::query_as::<_, Counties>("SELECT id, name FROM counties")
        .fetch_all(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "data": counties }))))
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct Constituency {
    pub id: i32,
    pub name: String,
}

pub async fn get_constituencies_by_county(
    Path(county_id): Path<i32>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let constituencies = sqlx::query_as::<_, Constituency>(
        "SELECT id, name FROM constituencies WHERE county_id = $1",
    )
    .bind(county_id)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "data": constituencies }))))
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct Ward {
    pub id: i32,
    pub name: String,
}

pub async fn get_wards_by_constituency(
    Path(constituency_id): Path<i32>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let wards = sqlx::query_as::<_, Ward>(
        "SELECT id, name FROM wards WHERE constituency_id = $1",
    )
    .bind(constituency_id)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "data": wards }))))
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct BusinessBranchLocation {
    id: i32,
    created_at: NaiveDateTime,
    updated_at: Option<NaiveDateTime>,
    name: String,
    latitude: f64,
    longitude: f64,
    ward_id: i32,
    phone: String,
    address: String,
}

#[derive(Deserialize, Validate, Serialize, Debug, Clone, sqlx::FromRow)]
pub struct CreateBranchLocationRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub ward_id: i32,
    #[validate(length(min = 1, max = 15))]
    pub phone: String,
    #[validate(length(min = 1, max = 255))]
    pub address: String,
}

pub async fn create_branch_location(
    Path(business_id): Path<i32>,
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreateBranchLocationRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let owns = sqlx::query_scalar!(
        "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
        business_id, user_id
    )
    .fetch_optional(&pool)
    .await?;

    if owns.is_none() {
        return Err(AppError::Forbidden(
            "You do not have permission to create a branch for this business".to_string(),
        ));
    }

    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    if business_id <= 0 {
        return Err(AppError::BadRequest("Invalid business ID".to_string()));
    }

    let now = chrono::Utc::now().naive_utc();

    let location = sqlx::query_as::<_, CreateBranchLocationRequest>(
        r#"INSERT INTO business_branches (business_id, name, latitude, longitude, ward_id, phone, address, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING name, latitude, longitude, ward_id, phone, address"#,
    )
    .bind(business_id)
    .bind(&payload.name)
    .bind(payload.latitude)
    .bind(payload.longitude)
    .bind(payload.ward_id)
    .bind(&payload.phone)
    .bind(&payload.address)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "data": location }))))
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct BranchLocationResponse {
    pub id: i32,
    pub business_id: i32,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub ward_id: i32,
    pub ward_name: String,
    pub constituency_name: String,
    pub county_name: String,
    pub phone: String,
    pub address: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub async fn get_branch_locations(
    Path(business_id): Path<i32>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let locations = sqlx::query_as::<_, BranchLocationResponse>(
        r#"SELECT bl.id, bl.business_id, bl.name, bl.latitude, bl.longitude,
                  bl.ward_id, w.name AS ward_name, c.name AS constituency_name,
                  co.name AS county_name, bl.phone, bl.address, bl.created_at, bl.updated_at
           FROM business_branches bl
           JOIN wards w ON bl.ward_id = w.id
           JOIN constituencies c ON w.constituency_id = c.id
           JOIN counties co ON c.county_id = co.id
           WHERE bl.business_id = $1"#,
    )
    .bind(business_id)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "data": locations }))))
}

#[derive(Deserialize, Validate, Serialize, Debug, Clone, sqlx::FromRow)]
pub struct ProviderLocationRequest {
    latitude: f64,
    longitude: f64,
    ward_id: i32,
    phone: String,
    address: String,
}

pub async fn create_provider_location(
    Path(provider_id): Path<i32>,
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<ProviderLocationRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let owns = sqlx::query_scalar!(
        "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
        provider_id, user_id
    )
    .fetch_optional(&pool)
    .await?;

    if owns.is_none() {
        return Err(AppError::Forbidden(
            "You do not have permission to create a location for this provider".to_string(),
        ));
    }

    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    if provider_id <= 0 {
        return Err(AppError::BadRequest("Invalid provider ID".to_string()));
    }

    let now = chrono::Utc::now().naive_utc();

    let location = sqlx::query_as::<_, ProviderLocationRequest>(
        r#"INSERT INTO provider_locations (provider_id, latitude, longitude, ward_id, phone, address, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING latitude, longitude, ward_id, phone, address"#,
    )
    .bind(provider_id)
    .bind(payload.latitude)
    .bind(payload.longitude)
    .bind(payload.ward_id)
    .bind(&payload.phone)
    .bind(&payload.address)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "data": location }))))
}

#[derive(Deserialize, Debug)]
pub struct LocationSearchQuery {
    county_id: Option<i32>,
    constituency_id: Option<i32>,
    ward_id: Option<i32>,
    target_type: String,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct LocationSearchResult {
    pub id: i32,
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub ward_name: Option<String>,
    pub constituency_name: Option<String>,
    pub county_name: Option<String>,
}

pub async fn search_business_or_provider_by_location(
    Query(params): Query<LocationSearchQuery>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let results = match params.target_type.to_lowercase().as_str() {
        "business" => sqlx::query_as::<_, LocationSearchResult>(
            r#"SELECT DISTINCT ON (b.id) b.id, b.business_name AS name,
                      bb.address, bb.phone,
                      w.name AS ward_name, c.name AS constituency_name, co.name AS county_name
               FROM businesses b
               JOIN business_branches bb ON bb.business_id = b.id
               JOIN wards w ON bb.ward_id = w.id
               JOIN constituencies c ON w.constituency_id = c.id
               JOIN counties co ON c.county_id = co.id
               WHERE ($1::int IS NULL OR co.id = $1)
                 AND ($2::int IS NULL OR c.id = $2)
                 AND ($3::int IS NULL OR w.id = $3)
               ORDER BY b.id"#,
        )
        .bind(params.county_id)
        .bind(params.constituency_id)
        .bind(params.ward_id)
        .fetch_all(&pool)
        .await?,

        "provider" => sqlx::query_as::<_, LocationSearchResult>(
            r#"SELECT DISTINCT ON (p.id) p.id, p.service_name AS name,
                      pl.address, pl.phone,
                      w.name AS ward_name, c.name AS constituency_name, co.name AS county_name
               FROM providers p
               JOIN provider_locations pl ON pl.provider_id = p.id
               JOIN wards w ON pl.ward_id = w.id
               JOIN constituencies c ON w.constituency_id = c.id
               JOIN counties co ON c.county_id = co.id
               WHERE ($1::int IS NULL OR co.id = $1)
                 AND ($2::int IS NULL OR c.id = $2)
                 AND ($3::int IS NULL OR w.id = $3)
               ORDER BY p.id"#,
        )
        .bind(params.county_id)
        .bind(params.constituency_id)
        .bind(params.ward_id)
        .fetch_all(&pool)
        .await?,

        _ => return Err(AppError::BadRequest(
            "Invalid target type. Must be 'business' or 'provider'".to_string(),
        )),
    };

    Ok((StatusCode::OK, Json(json!({ "data": results }))))
}

// ── Branch location CRUD ──────────────────────────────────────────────────────

pub async fn get_branch_by_id(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let branch = sqlx::query_as::<_, BranchLocationResponse>(
        r#"SELECT bl.id, bl.business_id, bl.name, bl.latitude, bl.longitude,
                  bl.ward_id, w.name AS ward_name, c.name AS constituency_name,
                  co.name AS county_name, bl.phone, bl.address, bl.created_at, bl.updated_at
           FROM business_branches bl
           JOIN wards w ON bl.ward_id = w.id
           JOIN constituencies c ON w.constituency_id = c.id
           JOIN counties co ON c.county_id = co.id
           WHERE bl.id = $1"#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Branch location not found".to_string()))?;

    Ok((StatusCode::OK, Json(json!({ "data": branch }))))
}

#[derive(Deserialize, Validate, Debug)]
pub struct UpdateBranchRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub ward_id: Option<i32>,
    #[validate(length(min = 1, max = 15))]
    pub phone: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub address: Option<String>,
}

pub async fn update_branch_location(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<UpdateBranchRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Ownership check — user must own the business this branch belongs to
    let owned = sqlx::query_scalar!(
        "SELECT 1 FROM business_branches bb
         JOIN businesses b ON bb.business_id = b.id
         WHERE bb.id = $1 AND b.user_id = $2",
        id, user_id
    )
    .fetch_optional(&pool)
    .await?;

    if owned.is_none() {
        return Err(AppError::Forbidden(
            "You do not have permission to update this branch".to_string(),
        ));
    }

    sqlx::query!(
        r#"UPDATE business_branches SET
               name       = COALESCE($1, name),
               latitude   = COALESCE($2, latitude),
               longitude  = COALESCE($3, longitude),
               ward_id    = COALESCE($4, ward_id),
               phone      = COALESCE($5, phone),
               address    = COALESCE($6, address),
               updated_at = NOW()
           WHERE id = $7"#,
        payload.name,
        payload.latitude,
        payload.longitude,
        payload.ward_id,
        payload.phone,
        payload.address,
        id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Branch location updated successfully" }))))
}

pub async fn delete_branch_location(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let owned = sqlx::query_scalar!(
        "SELECT 1 FROM business_branches bb
         JOIN businesses b ON bb.business_id = b.id
         WHERE bb.id = $1 AND b.user_id = $2",
        id, user_id
    )
    .fetch_optional(&pool)
    .await?;

    if owned.is_none() {
        return Err(AppError::Forbidden(
            "You do not have permission to delete this branch".to_string(),
        ));
    }

    sqlx::query!("DELETE FROM business_branches WHERE id = $1", id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Branch location deleted successfully" }))))
}

// ── Provider location CRUD ────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct ProviderLocationFull {
    pub id: i32,
    pub provider_id: i32,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub ward_id: i32,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

pub async fn get_provider_location_by_id(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let loc = sqlx::query_as!(
        ProviderLocationFull,
        "SELECT id, provider_id, latitude, longitude, ward_id, phone, address, created_at, updated_at
         FROM provider_locations WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Provider location not found".to_string()))?;

    Ok((StatusCode::OK, Json(json!({ "data": loc }))))
}

#[derive(Deserialize, Debug)]
pub struct UpdateProviderLocationRequest {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub ward_id: Option<i32>,
    pub phone: Option<String>,
    pub address: Option<String>,
}

pub async fn update_provider_location(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<UpdateProviderLocationRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let owned = sqlx::query_scalar!(
        "SELECT 1 FROM provider_locations pl
         JOIN providers p ON pl.provider_id = p.id
         WHERE pl.id = $1 AND p.user_id = $2",
        id, user_id
    )
    .fetch_optional(&pool)
    .await?;

    if owned.is_none() {
        return Err(AppError::Forbidden(
            "You do not have permission to update this location".to_string(),
        ));
    }

    sqlx::query!(
        r#"UPDATE provider_locations SET
               latitude   = COALESCE($1, latitude),
               longitude  = COALESCE($2, longitude),
               ward_id    = COALESCE($3, ward_id),
               phone      = COALESCE($4, phone),
               address    = COALESCE($5, address),
               updated_at = NOW()
           WHERE id = $6"#,
        payload.latitude,
        payload.longitude,
        payload.ward_id,
        payload.phone,
        payload.address,
        id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Provider location updated successfully" }))))
}

pub async fn delete_provider_location(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let owned = sqlx::query_scalar!(
        "SELECT 1 FROM provider_locations pl
         JOIN providers p ON pl.provider_id = p.id
         WHERE pl.id = $1 AND p.user_id = $2",
        id, user_id
    )
    .fetch_optional(&pool)
    .await?;

    if owned.is_none() {
        return Err(AppError::Forbidden(
            "You do not have permission to delete this location".to_string(),
        ));
    }

    sqlx::query!("DELETE FROM provider_locations WHERE id = $1", id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Provider location deleted successfully" }))))
}
