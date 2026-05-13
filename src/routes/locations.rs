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
        .route("/providers/:provider_id", post(create_provider_location))
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

#[derive(Deserialize, Serialize, Debug, Clone, sqlx::FromRow)]
pub struct SearchLocation {
    county_id: Option<i32>,
    constituency_id: Option<i32>,
    ward_id: Option<i32>,
    target_type: String,
}

pub async fn search_business_or_provider_by_location(
    Query(params): Query<SearchLocation>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let table = match params.target_type.as_str() {
        "business" => "businesses",
        "provider" => "providers",
        _ => return Err(AppError::BadRequest("Invalid target type. Must be 'business' or 'provider'".to_string())),
    };

    let mut query = format!("SELECT * FROM {}", table);
    let mut conditions = Vec::new();

    if let Some(county_id) = params.county_id {
        conditions.push(format!("county_id = {}", county_id));
    }
    if let Some(constituency_id) = params.constituency_id {
        conditions.push(format!("constituency_id = {}", constituency_id));
    }
    if let Some(ward_id) = params.ward_id {
        conditions.push(format!("ward_id = {}", ward_id));
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    let results = sqlx::query_as::<_, SearchLocation>(&query)
        .fetch_all(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "data": results }))))
}
