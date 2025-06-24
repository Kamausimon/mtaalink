use axum::{
    Router,
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn category_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/allCategories", get(get_categories))
        .route(
            "/allcategories/:id/subcategories",
            get(get_subcategories_by_category_id),
        )
        .route("/providers/by-category", get(get_providers_by_category)) // expects ?category=1
        .route("/businesses/by-category", get(get_businesses_by_category)) // expects ?category=1
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

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Subcategory {
    pub id: i32,
    pub name: String,
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: i32,
    pub name: String,
}

pub async fn get_subcategories_by_category_id(
    Path(parent_id): Path<i32>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let result =
        sqlx::query_as::<_, Category>("SELECT id, name FROM categories WHERE parent_id = $1")
            .bind(parent_id)
            .fetch_all(&pool)
            .await;

    match result {
        Ok(subcategories) => (
            StatusCode::OK,
            Json(json!({ "subcategories": subcategories })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("Failed to fetch subcategories: {}", e) })),
        ),
    }
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
pub struct ProviderWQueryResponse {
    pub category: Option<i32>,
    pub subcategory: Option<i32>,
}

pub async fn get_providers_by_category(
    State(pool): State<PgPool>,
    Query(params): Query<ProviderWQueryResponse>,
) -> impl IntoResponse {
    let category = params.category;
    let subcategory = params.subcategory;

    let mut base_query = r#"
        SELECT 
            pc.provider_id,
            sub.id AS subcategory_id,
            sub.name AS subcategory_name,
            parent.id AS parent_category_id,
            parent.name AS parent_category_name
        FROM 
            provider_categories pc
        JOIN 
            categories sub ON pc.category_id = sub.id
        LEFT JOIN 
            categories parent ON sub.parent_id = parent.id
        WHERE 1=1
    "#
    .to_string();

    if let Some(_) = category {
        base_query.push_str(" AND parent.id = $1");
    }

    if let Some(_) = subcategory {
        base_query.push_str(" AND sub.id = $2");
    }

    base_query.push_str(" ORDER BY pc.provider_id ASC, parent.name ASC, sub.name ASC");

    let results = match (category, subcategory) {
        (Some(c), Some(s)) => {
            sqlx::query_as::<_, ProviderCategoryResponse>(&base_query)
                .bind(c)
                .bind(s)
                .fetch_all(&pool)
                .await
        }
        (Some(c), None) => {
            sqlx::query_as::<_, ProviderCategoryResponse>(&base_query)
                .bind(c)
                .fetch_all(&pool)
                .await
        }
        _ => {
            sqlx::query_as::<_, ProviderCategoryResponse>(&base_query)
                .fetch_all(&pool)
                .await
        }
    };

    match results {
        Ok(providers) => (StatusCode::OK, Json(json!({ "providers": providers }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to fetch providers: {}", e) })),
        ),
    }
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct BusinessCategoryResponse {
    pub business_id: i32,
    pub subcategory_id: i32,
    pub subcategory_name: String,
    pub parent_category_id: Option<i32>,
    pub parent_category_name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct BusinessWQueryResponse {
    pub category: Option<i32>,
    pub subcategory: Option<i32>,
}

pub async fn get_businesses_by_category(
    State(pool): State<PgPool>,
    Query(params): Query<BusinessWQueryResponse>,
) -> impl IntoResponse {
    let category = params.category;
    let subcategory = params.subcategory;

    let mut base_query = r#"
        SELECT 
            bc.business_id,
            sub.id AS subcategory_id,
            sub.name AS subcategory_name,
            parent.id AS parent_category_id,
            parent.name AS parent_category_name
        FROM 
            business_categories bc
        JOIN 
            categories sub ON bc.category_id = sub.id
        LEFT JOIN 
            categories parent ON sub.parent_id = parent.id
        WHERE 1=1
    "#
    .to_string();

    if let Some(_) = category {
        base_query.push_str(" AND parent.id = $1");
    }

    if let Some(_) = subcategory {
        base_query.push_str(" AND sub.id = $2");
    }

    base_query.push_str(" ORDER BY bc.business_id ASC, parent.name ASC, sub.name ASC");

    let results = match (category, subcategory) {
        (Some(c), Some(s)) => {
            sqlx::query_as::<_, BusinessCategoryResponse>(&base_query)
                .bind(c)
                .bind(s)
                .fetch_all(&pool)
                .await
        }
        (Some(c), None) => {
            sqlx::query_as::<_, BusinessCategoryResponse>(&base_query)
                .bind(c)
                .fetch_all(&pool)
                .await
        }
        _ => {
            sqlx::query_as::<_, BusinessCategoryResponse>(&base_query)
                .fetch_all(&pool)
                .await
        }
    };

    match results {
        Ok(businesses) => (StatusCode::OK, Json(json!({ "businesses": businesses }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to fetch businesses: {}", e) })),
        ),
    }
}

#[derive(Deserialize, Debug)]
pub struct CategoryAssignment {
    target_id: i32,
    target_type: String, // "provider" or "business"
    category_ids: Vec<i32>,
}

pub async fn assign_categories(
    State(pool): State<PgPool>,
    Json(payload): Json<CategoryAssignment>,
) -> impl IntoResponse {
    let target_id = payload.target_id;
    let target_type = payload.target_type.to_lowercase();

    if target_type != "provider" && target_type != "business" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid target type. Must be 'provider' or 'business'." })),
        );
    }

    if payload.category_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No category IDs provided." })),
        );
    }

    if target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid target ID." })),
        );
    }

    let delete_query = match target_type.as_str() {
        "provider" => "DELETE FROM provider_categories WHERE provider_id = $1",
        "business" => "DELETE FROM business_categories WHERE business_id = $1",
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Unexpected error occurred." })),
            );
        }
    };

    if let Err(e) = sqlx::query(delete_query)
        .bind(payload.target_id)
        .execute(&pool)
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to delete existing categories: {}", e) })),
        );
    }

    let insert_query = match target_type.as_str() {
        "provider" => "INSERT INTO provider_categories (provider_id, category_id) VALUES ($1, $2)",
        "business" => "INSERT INTO business_categories (business_id, category_id) VALUES ($1, $2)",
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Unexpected error occurred." })),
            );
        }
    };

    for &cat_id in &payload.category_ids {
        if let Err(e) = sqlx::query(insert_query)
            .bind(target_id)
            .bind(cat_id)
            .execute(&pool)
            .await
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to assign category {}: {}", cat_id, e) })),
            );
        }
    }
    (
        StatusCode::OK,
        Json(json!({ "message": "Categories assigned successfully." })),
    )
}
