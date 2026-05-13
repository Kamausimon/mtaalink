use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn services_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createService", post(create_service))
        .route("/getServices", get(get_services))
        .route("/deleteService", post(delete_service))
        .route("/updateService", post(edit_service))
        .with_state(pool)
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Service {
    pub id: i32,
    pub target_id: i32,
    pub target_type: String,
    pub title: String,
    pub description: String,
    pub price: BigDecimal,
    pub duration: i32,
    pub category_id: Option<i32>,
    pub is_active: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Deserialize, Serialize)]
pub struct CreateServiceParams {
    pub target_id: i32,
    pub target_type: String,
    pub title: String,
    pub description: String,
    pub price: BigDecimal,
    pub duration: i32,
    pub category_id: Option<i32>,
    pub is_active: bool,
}

pub async fn create_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreateServiceParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut tx = pool.begin().await?;

    let target_exists = match payload.target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            payload.target_id, user_id
        ).fetch_optional(&mut *tx).await?,
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            payload.target_id, user_id
        ).fetch_optional(&mut *tx).await?,
        _ => return Err(AppError::BadRequest("Invalid target type".to_string())),
    };

    if target_exists.is_none() {
        return Err(AppError::Forbidden("You are not authorized to create services for this target".to_string()));
    }

    let record = sqlx::query!(
        r#"INSERT INTO services (target_id, target_type, title, description, price, duration, category_id, is_active)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id"#,
        payload.target_id,
        payload.target_type,
        payload.title,
        payload.description,
        payload.price,
        payload.duration,
        payload.category_id,
        payload.is_active
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Service created successfully",
            "service_id": record.id,
            "target_id": payload.target_id,
        })),
    ))
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct GetServicesParams {
    pub target_id: Option<i32>,
    pub target_type: Option<String>,
    pub category_id: Option<i32>,
    pub is_active: Option<bool>,
}

pub async fn get_services(
    State(pool): State<PgPool>,
    Query(params): Query<GetServicesParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut qb = sqlx::QueryBuilder::new("SELECT * FROM services WHERE 1=1");

    if let Some(target_id) = params.target_id {
        if let Some(ref target_type) = params.target_type {
            qb.push(" AND target_type = ").push_bind(target_type);
            qb.push(" AND target_id = ").push_bind(target_id);
        } else {
            qb.push(" AND target_id = ").push_bind(target_id);
        }
    }
    if let Some(category_id) = params.category_id {
        qb.push(" AND category_id = ").push_bind(category_id);
    }
    if let Some(is_active) = params.is_active {
        qb.push(" AND is_active = ").push_bind(is_active);
    }
    qb.push(" ORDER BY created_at DESC");

    let services = qb
        .build_query_as::<Service>()
        .fetch_all(&pool)
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::OK, Json(json!({ "services": services }))))
}

#[derive(Deserialize, Serialize)]
pub struct EditServiceParams {
    pub service_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<BigDecimal>,
    pub duration: Option<i32>,
    pub category_id: Option<i32>,
    pub is_active: Option<bool>,
    pub target_id: i32,
    pub target_type: String,
}

pub async fn edit_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<EditServiceParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut tx = pool.begin().await?;

    let target_exists = match payload.target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE user_id = $1 AND id = $2",
            user_id, payload.target_id
        ).fetch_optional(&mut *tx).await?,
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE user_id = $1 AND id = $2",
            user_id, payload.target_id
        ).fetch_optional(&mut *tx).await?,
        _ => return Err(AppError::BadRequest("Invalid target type".to_string())),
    };

    if target_exists.is_none() {
        return Err(AppError::Forbidden("You are not authorized to edit this service".to_string()));
    }

    let record = sqlx::query!(
        r#"UPDATE services
           SET title = COALESCE($1, title),
               description = COALESCE($2, description),
               price = COALESCE($3, price),
               duration = COALESCE($4, duration),
               category_id = COALESCE($5, category_id),
               is_active = COALESCE($6, is_active)
           WHERE id = $7 AND target_id = $8 AND target_type = $9
           RETURNING id"#,
        payload.title,
        payload.description,
        payload.price,
        payload.duration,
        payload.category_id,
        payload.is_active,
        payload.service_id,
        payload.target_id,
        payload.target_type
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Service updated successfully", "service_id": record.id }))))
}

#[derive(Deserialize, Serialize)]
pub struct DeleteServiceParams {
    pub service_id: i32,
}

pub async fn delete_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<DeleteServiceParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut tx = pool.begin().await?;

    let service = sqlx::query!(
        "SELECT target_id, target_type FROM services WHERE id = $1",
        payload.service_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Service not found".to_string()))?;

    let is_owner = match service.target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            service.target_id, user_id
        ).fetch_optional(&mut *tx).await?,
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            service.target_id, user_id
        ).fetch_optional(&mut *tx).await?,
        _ => return Err(AppError::BadRequest("Invalid target type".to_string())),
    };

    if is_owner.is_none() {
        return Err(AppError::Forbidden("You are not authorized to delete this service".to_string()));
    }

    let record = sqlx::query!(
        "DELETE FROM services WHERE id = $1 RETURNING id",
        payload.service_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Service deleted successfully", "service_id": record.id }))))
}
