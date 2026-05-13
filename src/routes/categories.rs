use crate::errors::{AppError, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn category_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/allCategories", get(get_categories))
        .route("/allcategories/:id/subcategories", get(get_subcategories_by_category_id))
        .route("/providers/by-category", get(get_providers_by_category))
        .route("/businesses/by-category", get(get_businesses_by_category))
        .route("/assignCategories", post(assign_categories))
        .with_state(pool)
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct CategoryWithParent {
    pub id: i32,
    pub category_name: String,
    pub parent_id: Option<i32>,
    pub parent_name: Option<String>,
}

pub async fn get_categories(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let categories = sqlx::query_as!(
        CategoryWithParent,
        r#"SELECT c.id, c.name AS category_name, c.parent_id, p.name AS parent_name
           FROM categories c LEFT JOIN categories p ON c.parent_id = p.id"#
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "categories": categories }))))
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: i32,
    pub name: String,
}

pub async fn get_subcategories_by_category_id(
    Path(parent_id): Path<i32>,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let subcategories = sqlx::query_as::<_, Category>(
        "SELECT id, name FROM categories WHERE parent_id = $1",
    )
    .bind(parent_id)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "subcategories": subcategories }))))
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct ProviderCategoryResponse {
    pub provider_id: i32,
    pub subcategory_id: i32,
    pub subcategory_name: String,
    pub parent_category_id: Option<i32>,
    pub parent_category_name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CategoryFilterQuery {
    pub category: Option<i32>,
    pub subcategory: Option<i32>,
}

pub async fn get_providers_by_category(
    State(pool): State<PgPool>,
    Query(params): Query<CategoryFilterQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut base_query = String::from(
        "SELECT pc.provider_id, sub.id AS subcategory_id, sub.name AS subcategory_name,
                parent.id AS parent_category_id, parent.name AS parent_category_name
         FROM provider_categories pc
         JOIN categories sub ON pc.category_id = sub.id
         LEFT JOIN categories parent ON sub.parent_id = parent.id
         WHERE 1=1",
    );

    if params.category.is_some() {
        base_query.push_str(" AND parent.id = $1");
    }
    if params.subcategory.is_some() {
        base_query.push_str(" AND sub.id = $2");
    }
    base_query.push_str(" ORDER BY pc.provider_id ASC, parent.name ASC, sub.name ASC");

    let providers = match (params.category, params.subcategory) {
        (Some(c), Some(s)) => sqlx::query_as::<_, ProviderCategoryResponse>(&base_query).bind(c).bind(s).fetch_all(&pool).await,
        (Some(c), None) => sqlx::query_as::<_, ProviderCategoryResponse>(&base_query).bind(c).fetch_all(&pool).await,
        _ => sqlx::query_as::<_, ProviderCategoryResponse>(&base_query).fetch_all(&pool).await,
    }
    .map_err(AppError::Database)?;

    Ok((StatusCode::OK, Json(json!({ "providers": providers }))))
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct BusinessCategoryResponse {
    pub business_id: i32,
    pub subcategory_id: i32,
    pub subcategory_name: String,
    pub parent_category_id: Option<i32>,
    pub parent_category_name: Option<String>,
}

pub async fn get_businesses_by_category(
    State(pool): State<PgPool>,
    Query(params): Query<CategoryFilterQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut base_query = String::from(
        "SELECT bc.business_id, sub.id AS subcategory_id, sub.name AS subcategory_name,
                parent.id AS parent_category_id, parent.name AS parent_category_name
         FROM business_categories bc
         JOIN categories sub ON bc.category_id = sub.id
         LEFT JOIN categories parent ON sub.parent_id = parent.id
         WHERE 1=1",
    );

    if params.category.is_some() {
        base_query.push_str(" AND parent.id = $1");
    }
    if params.subcategory.is_some() {
        base_query.push_str(" AND sub.id = $2");
    }
    base_query.push_str(" ORDER BY bc.business_id ASC, parent.name ASC, sub.name ASC");

    let businesses = match (params.category, params.subcategory) {
        (Some(c), Some(s)) => sqlx::query_as::<_, BusinessCategoryResponse>(&base_query).bind(c).bind(s).fetch_all(&pool).await,
        (Some(c), None) => sqlx::query_as::<_, BusinessCategoryResponse>(&base_query).bind(c).fetch_all(&pool).await,
        _ => sqlx::query_as::<_, BusinessCategoryResponse>(&base_query).fetch_all(&pool).await,
    }
    .map_err(AppError::Database)?;

    Ok((StatusCode::OK, Json(json!({ "businesses": businesses }))))
}

#[derive(Deserialize, Debug)]
pub struct CategoryAssignment {
    target_id: i32,
    target_type: String,
    category_ids: Vec<i32>,
}

pub async fn assign_categories(
    State(pool): State<PgPool>,
    Json(payload): Json<CategoryAssignment>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = payload.target_type.to_lowercase();
    if target_type != "provider" && target_type != "business" {
        return Err(AppError::BadRequest("Invalid target type. Must be 'provider' or 'business'".to_string()));
    }
    if payload.category_ids.is_empty() {
        return Err(AppError::BadRequest("No category IDs provided".to_string()));
    }
    if payload.category_ids.len() > 5 {
        return Err(AppError::BadRequest("You can assign a maximum of 5 categories".to_string()));
    }
    if payload.target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID".to_string()));
    }

    let top_category_name = sqlx::query_scalar!(
        "SELECT name FROM categories WHERE id = $1",
        payload.category_ids[0]
    )
    .fetch_one(&pool)
    .await?;

    let mut tx = pool.begin().await?;

    let delete_query = match target_type.as_str() {
        "provider" => "DELETE FROM provider_categories WHERE provider_id = $1",
        "business" => "DELETE FROM business_categories WHERE business_id = $1",
        _ => unreachable!(),
    };
    sqlx::query(delete_query)
        .bind(payload.target_id)
        .execute(&mut *tx)
        .await?;

    let update_query = match target_type.as_str() {
        "provider" => "UPDATE providers SET category = $1 WHERE id = $2",
        "business" => "UPDATE businesses SET category = $1 WHERE id = $2",
        _ => unreachable!(),
    };
    sqlx::query(update_query)
        .bind(&top_category_name)
        .bind(payload.target_id)
        .execute(&mut *tx)
        .await?;

    let insert_query = match target_type.as_str() {
        "provider" => "INSERT INTO provider_categories (provider_id, category_id) VALUES ($1, $2)",
        "business" => "INSERT INTO business_categories (business_id, category_id) VALUES ($1, $2)",
        _ => unreachable!(),
    };
    for &cat_id in &payload.category_ids {
        sqlx::query(insert_query)
            .bind(payload.target_id)
            .bind(cat_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to assign category {}: {}", cat_id, e)))?;
    }

    tx.commit().await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Categories assigned successfully" }))))
}
