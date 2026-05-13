use crate::errors::{AppError, AppResult};
use crate::extractors::administrator::require_admin;
use axum::{
    Json, Router,
    extract::{State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn admin_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/categories", get(get_categories))
        .route("/create_category", post(create_category))
        .route("/create_parent_category", post(create_parent_category))
        .route("/delete_category", post(delete_category))
        .route("/users", get(get_users))
        .route("/delete_user", post(delete_user))
        .layer(axum::middleware::from_fn_with_state(pool.clone(), require_admin))
        .with_state(pool)
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Deserialize, Serialize, Validate)]
pub struct NewCategory {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub parent_id: Option<i32>,
}

pub async fn create_category(
    State(pool): State<PgPool>,
    Json(payload): Json<NewCategory>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let row = sqlx::query!(
        "INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id",
        payload.name,
        payload.parent_id,
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "message": "Category created successfully", "id": row.id }))))
}

#[derive(Deserialize, Serialize, Validate, Debug, sqlx::FromRow)]
pub struct NewParentCategory {
    subcategory_name: String,
    parent_category_name: String,
}

pub async fn create_parent_category(
    State(pool): State<PgPool>,
    Json(payload): Json<NewParentCategory>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut tx = pool.begin().await?;

    let existing_parent = sqlx::query_scalar!(
        "SELECT id FROM categories WHERE name = $1 AND parent_id IS NULL",
        payload.parent_category_name
    )
    .fetch_optional(&mut *tx)
    .await?;

    let parent_id = if let Some(id) = existing_parent {
        id
    } else {
        sqlx::query!(
            "INSERT INTO categories (name, parent_id) VALUES ($1, NULL) RETURNING id",
            payload.parent_category_name
        )
        .fetch_one(&mut *tx)
        .await?
        .id
    };

    let subcategory = sqlx::query!(
        "INSERT INTO categories (name, parent_id) VALUES ($1, $2) RETURNING id",
        payload.subcategory_name,
        parent_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Parent category and subcategory created successfully",
            "subcategory_id": subcategory.id
        })),
    ))
}

#[derive(Deserialize, Debug)]
pub struct DeleteCategoryParams {
    pub category_id: i32,
}

pub async fn delete_category(
    State(pool): State<PgPool>,
    Json(payload): Json<DeleteCategoryParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query!("DELETE FROM categories WHERE id = $1", payload.category_id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Category deleted successfully" }))))
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub role: Option<String>,
}

pub async fn get_users(
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let users = sqlx::query_as!(
        User,
        "SELECT id, username, email, role FROM users ORDER BY id DESC"
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "users": users }))))
}

#[derive(Deserialize, Debug)]
pub struct DeleteUserParams {
    pub user_id: i32,
}

pub async fn delete_user(
    State(pool): State<PgPool>,
    Json(payload): Json<DeleteUserParams>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query!("DELETE FROM users WHERE id = $1", payload.user_id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "User deleted successfully" }))))
}
