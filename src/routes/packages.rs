use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn package_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(create_package))
        .route("/:id", get(get_package).put(update_package).delete(delete_package))
        .route("/:id/items", post(add_item).delete(remove_item))
        .with_state(pool)
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct CreatePackageInput {
    pub target_type: String,
    pub target_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub price: BigDecimal,
    /// Service IDs to include in the package
    pub service_ids: Vec<i32>,
}

#[derive(Deserialize, Debug)]
pub struct UpdatePackageInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<BigDecimal>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct PackageItemInput {
    pub service_id: i32,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct PackageRow {
    pub id: i32,
    pub target_type: String,
    pub target_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub price: BigDecimal,
    pub is_active: bool,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct PackageServiceItem {
    pub service_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub price: Option<BigDecimal>,
    pub duration: Option<i32>,
}

// ── Ownership helper ──────────────────────────────────────────────────────────

async fn verify_owner(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    user_id: i32,
) -> AppResult<()> {
    let owner = match target_type {
        "provider" => sqlx::query_scalar!(
            "SELECT user_id FROM providers WHERE id = $1", target_id
        ).fetch_optional(pool).await?,
        _ => sqlx::query_scalar!(
            "SELECT user_id FROM businesses WHERE id = $1", target_id
        ).fetch_optional(pool).await?,
    };
    if owner != Some(user_id) {
        return Err(AppError::Forbidden("You do not own this profile".to_string()));
    }
    Ok(())
}

// ── POST /packages ────────────────────────────────────────────────────────────

pub async fn create_package(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreatePackageInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = payload.target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("target_type must be 'provider' or 'business'".to_string()));
    }
    if payload.price <= BigDecimal::from(0) {
        return Err(AppError::BadRequest("Price must be greater than zero".to_string()));
    }
    if payload.service_ids.is_empty() {
        return Err(AppError::BadRequest("A package must include at least one service".to_string()));
    }

    verify_owner(&pool, &target_type, payload.target_id, user_id).await?;

    let mut tx = pool.begin().await?;

    let pkg = sqlx::query!(
        r#"INSERT INTO service_packages (target_type, target_id, name, description, price)
           VALUES ($1, $2, $3, $4, $5) RETURNING id"#,
        target_type, payload.target_id, payload.name.trim(),
        payload.description.as_deref(), payload.price
    )
    .fetch_one(&mut *tx)
    .await?;

    for sid in &payload.service_ids {
        // Verify service belongs to this target
        let exists = sqlx::query_scalar!(
            "SELECT id FROM services WHERE id = $1 AND target_type = $2 AND target_id = $3",
            sid, target_type, payload.target_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        if exists.is_none() {
            return Err(AppError::BadRequest(format!("Service {} not found for this target", sid)));
        }

        sqlx::query!(
            "INSERT INTO service_package_items (package_id, service_id) VALUES ($1, $2)",
            pkg.id, sid
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(json!({ "message": "Package created", "package_id": pkg.id }))))
}

// ── GET /packages/:id ─────────────────────────────────────────────────────────

pub async fn get_package(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let pkg = sqlx::query_as!(
        PackageRow,
        "SELECT id, target_type, target_id, name, description, price, is_active, created_at
         FROM service_packages WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Package not found".to_string()))?;

    let services = sqlx::query_as::<_, PackageServiceItem>(
        r#"SELECT s.id AS service_id, s.title, s.description, s.price, s.duration
           FROM service_package_items spi
           JOIN services s ON spi.service_id = s.id
           WHERE spi.package_id = $1"#,
    )
    .bind(id)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "package": pkg, "services": services }))))
}

// ── PUT /packages/:id ─────────────────────────────────────────────────────────

pub async fn update_package(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdatePackageInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let pkg = sqlx::query!(
        "SELECT target_type, target_id FROM service_packages WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Package not found".to_string()))?;

    verify_owner(&pool, &pkg.target_type, pkg.target_id, user_id).await?;

    sqlx::query!(
        r#"UPDATE service_packages
           SET name        = COALESCE($1, name),
               description = COALESCE($2, description),
               price       = COALESCE($3, price),
               is_active   = COALESCE($4, is_active),
               updated_at  = NOW()
           WHERE id = $5"#,
        payload.name.as_deref(), payload.description.as_deref(),
        payload.price, payload.is_active, id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Package updated" }))))
}

// ── DELETE /packages/:id ──────────────────────────────────────────────────────

pub async fn delete_package(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let pkg = sqlx::query!(
        "SELECT target_type, target_id FROM service_packages WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Package not found".to_string()))?;

    verify_owner(&pool, &pkg.target_type, pkg.target_id, user_id).await?;

    sqlx::query!("DELETE FROM service_packages WHERE id = $1", id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Package deleted" }))))
}

// ── POST /packages/:id/items — add a service ──────────────────────────────────

pub async fn add_item(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
    Json(payload): Json<PackageItemInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let pkg = sqlx::query!(
        "SELECT target_type, target_id FROM service_packages WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Package not found".to_string()))?;

    verify_owner(&pool, &pkg.target_type, pkg.target_id, user_id).await?;

    sqlx::query_scalar!(
        "SELECT id FROM services WHERE id = $1 AND target_type = $2 AND target_id = $3",
        payload.service_id, pkg.target_type, pkg.target_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Service not found for this target".to_string()))?;

    sqlx::query!(
        "INSERT INTO service_package_items (package_id, service_id) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        id, payload.service_id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Service added to package" }))))
}

// ── DELETE /packages/:id/items — remove a service ────────────────────────────

pub async fn remove_item(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
    Json(payload): Json<PackageItemInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let pkg = sqlx::query!(
        "SELECT target_type, target_id FROM service_packages WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Package not found".to_string()))?;

    verify_owner(&pool, &pkg.target_type, pkg.target_id, user_id).await?;

    sqlx::query!(
        "DELETE FROM service_package_items WHERE package_id = $1 AND service_id = $2",
        id, payload.service_id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Service removed from package" }))))
}
