use axum::{
    Router,
    extract::{Json, Multipart, Query, State,Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get,post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use std::fs::File;
use std::io::Write;
use sqlx::{Postgres, Transaction};
use bigdecimal::BigDecimal;

use crate::utils::attachments::upload_attachments;
use crate::extractors::current_user::CurrentUser;

//routes for services
pub fn services_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createService", post(create_service))
        .route("/getServices", get(get_services))
        .route("/deleteService", post(delete_service))
        .route("/updateService", post(edit_service))
        .route("/:service_id/attachments", post(upload_attachments))
        .with_state(pool)
}


//create services with attachments
#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Service {
    pub id: i32,
    pub target_id: i32,             // Changed from provider_id
    pub target_type: String,        // New field
    pub title: String,
    pub description: String,
    pub price: BigDecimal, // Use BigDecimal for monetary values
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
    pub price: BigDecimal, // Use BigDecimal for monetary values
    pub duration: i32,
    pub category_id: Option<i32>,
    pub is_active: bool,
}

pub async fn create_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreateServiceParams>,
) -> impl IntoResponse {
    let user_id = user_id.parse::<i32>().unwrap_or(0);
    
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to start transaction"})));
        }
    };
    
    // Check if the target exists and user has permission
    let target_exists = match payload.target_type.as_str() {
        "provider" => {
            sqlx::query!(
                "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
                payload.target_id, 
                user_id
            )
            .fetch_optional(&mut *tx)
            .await
        },
        "business" => {
            sqlx::query!(
                "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
                payload.target_id, 
                user_id
            )
            .fetch_optional(&mut *tx)
            .await
        },
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Invalid target type"})));
        }
    };
    
    match target_exists {
        Ok(Some(_)) => {}, // User is authorized
        Ok(None) => {
            return (StatusCode::FORBIDDEN, Json(json!({"message": "You are not authorized to create services for this target"})));
        },
        Err(e) => {
            eprintln!("Failed to check target: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to verify permissions"})));
        }
    };
    
    // Insert the service
    let result = sqlx::query!(
        r#"
        INSERT INTO services (target_id, target_type, title, description, price, duration, category_id, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
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
    .await;
    
    let service_id = match result {
        Ok(record) => record.id,
        Err(e) => {
            eprintln!("Failed to create service: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to create service"})));
        }
    };
    
    // Commit the transaction
    if let Err(e) = tx.commit().await {
        eprintln!("Failed to commit transaction: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to commit transaction"})));
    }
    
    // Return success response
    (
        StatusCode::CREATED,
        Json(json!({
            "message": "Service created successfully",
            "service_id": service_id,
            "target_id": payload.target_id,
            "upload_attachments_url": format!("/attachments/uploadAttachments?target_id={}&target_type={}&service_id={}", 
                payload.target_id, payload.target_type, service_id)
        }))
    )
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct GetServicesParams {
    pub provider_id: Option<i32>,     // For backward compatibility
    pub target_id: Option<i32>,       // New field for target-based filtering
    pub target_type: Option<String>,  // New field for target type filtering
    pub category_id: Option<i32>,     // Filter by category
    pub is_active: Option<bool>,      // Filter by active status
}

pub async fn get_services(
    State(pool): State<PgPool>,
    Query(params): Query<GetServicesParams>,
) -> impl IntoResponse {
    // Prepare query based on filters
    let mut query = String::from("SELECT * FROM services WHERE 1=1");
    let mut query_params: Vec<Box<dyn sqlx::postgres::PgArgumentBuffer + '_>> = Vec::new();
    
    // Add target filtering if provided
    if let Some(target_id) = params.target_id {
        if let Some(target_type) = &params.target_type {
            query.push_str(" AND target_type = $1 AND target_id = $2");
            query_params.push(Box::new(target_type.clone()));
            query_params.push(Box::new(target_id));
        } else {
            query.push_str(" AND target_id = $1");
            query_params.push(Box::new(target_id));
        }
    }
    
    // Add category filter if provided
    if let Some(category_id) = params.category_id {
        let param_index = query_params.len() + 1;
        query.push_str(&format!(" AND category_id = ${}", param_index));
        query_params.push(Box::new(category_id));
    }
    
    // Add active filter
    if let Some(is_active) = params.is_active {
        let param_index = query_params.len() + 1;
        query.push_str(&format!(" AND is_active = ${}", param_index));
        query_params.push(Box::new(is_active));
    }
    
    query.push_str(" ORDER BY created_at DESC");
    
    // Execute query using proper binding approach
    let mut sql_query = sqlx::query_as::<_, Service>(&query);
    
    // This approach avoids the issue with trait objects
    // Bind each parameter individually
    for param in query_params {
        sql_query = sql_query.bind(param);
    }
    
    let services_result = sql_query.fetch_all(&pool).await;
    
    match services_result {
        Ok(services) => {
            (StatusCode::OK, Json(json!({"services": services})))
        },
        Err(e) => {
            eprintln!("Failed to fetch services: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to fetch services"})))
        }
    }
}

//enable providers to edit their services and update them
#[derive(Deserialize, Serialize)]
pub struct EditServiceParams {
    pub service_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<BigDecimal>,
    pub duration: Option<i32>,
    pub category_id: Option<i32>,
    pub is_active: Option<bool>,
    pub target_id: i32,            // Changed from provider_id
    pub target_type: String,       // New field for target type
}

#[derive(Deserialize, Serialize)]
pub struct EditParams{
    pub provider_id: i32,
}

pub async fn edit_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<EditServiceParams>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to start transaction"})));
        }
    };

    // Check if the user owns this target
    let target_exists = match payload.target_type.as_str() {
        "provider" => {
            sqlx::query!(
                "SELECT id from providers WHERE user_id = $1 AND id = $2",
                user_id.parse::<i32>().unwrap(),
                payload.target_id
            ).fetch_optional(&mut *tx).await
        },
        "business" => {
            sqlx::query!(
                "SELECT id from businesses WHERE user_id = $1 AND id = $2",
                user_id.parse::<i32>().unwrap(),
                payload.target_id
            ).fetch_optional(&mut *tx).await
        },
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Invalid target type"})));
        }
    };

    match target_exists {
        Ok(Some(_)) => {}, // User is authorized
        Ok(None) => {
            return (StatusCode::FORBIDDEN, Json(json!({"message": "You are not authorized to edit this service"})));
        },
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to verify permissions"})));
        }
    };

    // Update the service
    let service_update_result = sqlx::query!(
        r#"
        UPDATE services
        SET title = COALESCE($1, title),
            description = COALESCE($2, description),
            price = COALESCE($3, price),
            duration = COALESCE($4, duration),
            category_id = COALESCE($5, category_id),
            is_active = COALESCE($6, is_active)
        WHERE id = $7 AND target_id = $8 AND target_type = $9
        RETURNING id
        "#,
        payload.title,
        payload.description,
        payload.price,
        payload.duration,
        payload.category_id,
        payload.is_active,
        payload.service_id,
        payload.target_id,
        payload.target_type
    ).fetch_one(&mut *tx).await;

    // Extract just the ID from the result
    let service_id = match service_update_result {
        Ok(record) => record.id,
        Err(e) => {
            eprintln!("Failed to update service: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to update service"})));
        }
    };

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        eprintln!("Failed to commit transaction: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to commit transaction"})));
    }

    // Return the final response
    (
        StatusCode::OK,
        Json(json!({"message": "Service updated successfully", "service_id": service_id}))
    )
}

//enable deleting services
#[derive(Deserialize, Serialize)]
pub struct DeleteServiceParams {
    pub service_id: i32,
}

pub async fn delete_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<DeleteServiceParams>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to start transaction"})));
        }
    };

    // First get the service to check ownership
    let service_check = sqlx::query!(
        "SELECT target_id, target_type FROM services WHERE id = $1",
        payload.service_id
    ).fetch_optional(&mut *tx).await;

    let (target_id, target_type) = match service_check {
        Ok(Some(record)) => (record.target_id, record.target_type),
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({"message": "Service not found"})));
        },
        Err(e) => {
            eprintln!("Failed to fetch service: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to fetch service"})));
        }
    };

    // Check if user owns this service's target
    let owner_check = match target_type.as_str() {
        "provider" => {
            sqlx::query!(
                "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
                target_id,
                user_id.parse::<i32>().unwrap()
            ).fetch_optional(&mut *tx).await
        },
        "business" => {
            sqlx::query!(
                "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
                target_id,
                user_id.parse::<i32>().unwrap()
            ).fetch_optional(&mut *tx).await
        },
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Invalid target type"})));
        }
    };

    match owner_check {
        Ok(Some(_)) => {}, // User is authorized
        Ok(None) => {
            return (StatusCode::FORBIDDEN, Json(json!({"message": "You are not authorized to delete this service"})));
        },
        Err(e) => {
            eprintln!("Failed to check ownership: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to verify permissions"})));
        }
    }

    // Delete the service
    let delete_result = sqlx::query!(
        "DELETE FROM services WHERE id = $1 RETURNING id", 
        payload.service_id
    )
    .fetch_one(&mut *tx)
    .await;

    match delete_result {
        Ok(record) => {
            tx.commit().await.expect("Failed to commit transaction");
            (
                StatusCode::OK,
                Json(json!({"message": "Service deleted successfully", "service_id": record.id})),
            )
        }
        Err(e) => {
            eprintln!("Failed to delete service: {}", e);
            tx.rollback().await.expect("Failed to rollback transaction");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to delete service: {}", e)})),
            )
        }
    }
}

#[derive(Deserialize)]
pub struct AttachmentParams {
    pub target_type: String,
    pub target_id: i32,
    pub uploaded_by: i32,
}

//upload attachments to the service
pub async fn upload_service_attachments (
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(service_id): Path<i32>,
    mut multipart: Multipart,
) -> impl IntoResponse {
let permission_check = sqlx::query!(
    "SELECT s.id FROM services s 
    JOIN providers p ON s.target_id = p.id AND s.target_type = 'provider'
    WHERE s.id = $1 AND p.user_id = $2",
    service_id,
    user_id.parse::<i32>().unwrap()
).fetch_optional(&pool).await;

    match permission_check {
        Ok(Some(_)) => {},
        Ok(None) => {
            return (StatusCode::FORBIDDEN, Json(json!({"message": "You do not have permission to upload attachments for this service"})));
        },
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to check permissions"})));
        }
    }

    let params = AttachmentParams {
        target_type: "service".to_string(),
        target_id: service_id,
        uploaded_by: user_id.parse::<i32>().unwrap(),
    };

    upload_attachments(
        State(pool),
        Query(params),
        CurrentUser { user_id },
        multipart,
    ).await
}

