use crate::extractors::administrator::require_admin;
use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn admin_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/categories", get(get_categories)) //done
        .route("/create_category", post(create_category)) //done
        .route("/create_parent_category", post(create_parent_category)) //done
        .route("/delete_category", post(delete_category)) //done
        .route("/users", get(get_users)) //done
        .route("/delete_user", post(delete_user)) //done
        // .route("/userAnalytics", get(get_user_analytics))
        // .route("/flagContent", post(flag_content))
        // .route("/resolveFlag", post(resolve_flag))
        // .route("/moderateReviews", get(moderate_reviews))
        .layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            require_admin,
        ))
        .with_state(pool)
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CategoryWithParent {
    pub id: i32,
    pub category_name: String,
    pub parent_id: Option<i32>,
    pub parent_name: Option<String>,
}

pub async fn get_categories(State(pool): State<PgPool>) -> impl IntoResponse {
    let categories = sqlx::query_as!(
        CategoryWithParent,
        r#"
        SELECT 
            c.id,
            c.name AS category_name,
            c.parent_id,
            p.name AS parent_name
        FROM 
            categories c
        LEFT JOIN 
            categories p ON c.parent_id = p.id
        "#
    )
    .fetch_all(&pool)
    .await;

    match categories {
        Ok(categories) => (StatusCode::OK, Json(json!({ "categories": categories }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to fetch categories: {}", e) })),
        ),
    }
}

//create a new catergory as admin
#[derive(Deserialize, Serialize, Validate)]
pub struct NewCategory {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub parent_id: Option<i32>,
}

pub async fn create_category(
    State(pool): State<PgPool>,
    Json(payload): Json<NewCategory>,
) -> impl IntoResponse {
    //validate payload
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        );
    }

    //insert the new category into the database
    let result = sqlx::query!(
        "INSERT INTO categories (name,parent_id) VALUES  ($1, $2) RETURNING id",
        payload.name,
        payload.parent_id,
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(row) => (
            StatusCode::CREATED,
            Json(json!({ "message": "Category created successfully", "id": row.id })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to create category: {}", e) })),
        ),
    }
}

//create a new parent category as admin
#[derive(Deserialize, Serialize, Validate, Debug, sqlx::FromRow)]
pub struct NewParentCategory {
    subcategory_name: String,
    parent_category_name: String,
}

pub async fn create_parent_category(
    State(pool): State<PgPool>,
    Json(payload): Json<NewParentCategory>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "There was an error starting the transaction" })),
            );
        }
    };

    //check if the parent category exists first
    let parent = sqlx::query!(
        "SELECT id FROM categories WHERE name = $1 AND id IS NULL",
        payload.parent_category_name
    )
    .fetch_optional(&mut *tx);

    //if the parent does not exist continue and create
    let parent_id = match parent.await {
        Ok(Some(record)) => record.id, // Parent category exists
        //if the parent does not exist, create it
        Ok(None) => {
            let new_parent = sqlx::query!(
                "INSERT INTO categories (name, parent_id) VALUES ($1, NULL) RETURNING id",
                payload.parent_category_name
            )
            .fetch_one(&mut *tx)
            .await
            .expect("Failed to create parent category");
            new_parent.id
        }
        Err(e) => {
            eprintln!("Failed to check parent category: {}", e);
            tx.rollback().await.expect("Failed to rollback transaction");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to check parent category" })),
            );
        }
    };

    //create the subcategory
    let subcategory = sqlx::query!(
        "INSERT INTO categories(name,parent_id) VALUES ($1, $2) RETURNING id",
        payload.subcategory_name,
        parent_id
    )
    .fetch_one(&mut *tx)
    .await;

    match subcategory {
        Ok(record) => {
            tx.commit().await.expect("Failed to commit transaction");
            (
                StatusCode::CREATED,
                Json(
                    json!({ "message": "Parent category and subcategory created successfully", "subcategory_id": record.id }),
                ),
            )
        }
        Err(e) => {
            eprintln!("Failed to create subcategory: {}", e);
            tx.rollback().await.expect("Failed to rollback transaction");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to create subcategory: {}", e) })),
            )
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct DeleteCategoryParams {
    pub category_id: i32,
}

//delete a category
pub async fn delete_category(
    State(pool): State<PgPool>,
    Json(payload): Json<DeleteCategoryParams>,
) -> impl IntoResponse {
    let result = sqlx::query!("DELETE FROM categories WHERE id = $1", payload.category_id)
        .execute(&pool)
        .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Category deleted successfully" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to delete category: {}", e) })),
        ),
    }
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub role: String,
}

pub async fn get_users(State(pool): State<PgPool>) -> impl IntoResponse {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, email, role FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await;

    match users {
        Ok(users) => (StatusCode::OK, Json(json!({ "users": users }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to fetch users: {}", e) })),
        ),
    }
}

#[derive(Deserialize, Debug)]
pub struct DeleteUserParams {
    pub user_id: i32,
}

pub async fn delete_user(
    State(pool): State<PgPool>,
    Json(payload): Json<DeleteUserParams>,
) -> impl IntoResponse {
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", payload.user_id)
        .execute(&pool)
        .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "User deleted successfully" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to delete user: {}", e) })),
        ),
    }
}

//todo: implement user analytics, flag content, resolve flag, moderate reviews
