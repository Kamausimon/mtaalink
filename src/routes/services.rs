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
#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct Service{
    pub title: String,
    pub description: String,
    pub price: f64,
    pub duration: i32,
    pub category_id: i32,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub attachments: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]
pub struct ServiceParams {
    pub provider_id: i32,
}

pub  async fn create_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<ServiceParams>,
  Json(payload): Json<Service>,
)-> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(tx)=> tx, 
        Err(_)=> {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to start transaction"})));
        }
    };

    //check if the user is a provider
    let provider_id = params.provider_id;

    let provider_results = sqlx::query!(
        "SELECT id FROM service_providers WHERE user_id = $1 AND id = $2",
        user_id.parse::<i32>().unwrap(),
        provider_id
    ).fetch_optional(&mut *tx).await;

    //match the results
    let provider = match provider_results {
        Ok(Some(provider))=> provider,
        Ok(None)=> {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "you are not authorized to create services"})));
        },
        Err(_)=> {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to fetch provider"})));
        }
    };

    //validate the required fields
    let title = match &payload.title {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Title is required"})));
        }
    };

    let description = match description {
        Some(d) if !d.trim().is_empty() => d,
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Description is required"})));
        }
    };

    let price = match price {
        Some(p) if p > 0.0 => p,
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Price must be greater than 0"})));
        }
    };

    let duration = match duration {
        Some(d) if d > 0 => d,
        _ => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Duration must be greater than 0"})));
        }
    };

    let category_id = match category_id {
        Some(c) => c,
        None => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "Category ID is required"})));
        }
    };

    let service_result = sqlx::query!(
        r#"
        INSERT INTO services (provider_id, title, description, price, duration, category_id, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        provider_id,
        title,
        description,
        price,
        duration,
        category_id,
        payload.is_active
    )
    .fetch_one(&mut *tx)
    .await;

        //commit the transaction
    if let Err(_) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to commit transaction"})));
    }

//return the service if so that the frontend can use that to upload attahcments 


    let service_id = match service_result {
        Ok(record) => {
          return (  StatusCode::OK,
            Json(json!({"message": "Service created successfully", "service_id": record.id})))
        },
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to create service"})));
        }
    };


}

#[derive(Deserialize, Serialize,sqlx::FromRow)]
pub struct GetServicesParams {
    pub provider_id: Option<i32>,
}

pub async fn get_services(
    State(pool): State<PgPool>,
    Query(params): Query<GetServicesParams>,
) -> impl IntoResponse {
    let provider_id = params.provider_id;

    let mut query = String::from("SELECT * FROM services");
let mut query_params: Vec<Box<dyn sqlx::postgres::PgArgumentBuffer + '_>> = Vec::new();

    if let Some(id) = provider_id {
        query.push_str(" WHERE provider_id = $1");
        query_params.push(Box::new(id));
    }

    let services_result = sqlx::query_as::<_, Service>(&query)
        .bind(query_params)
        .fetch_all(&pool)
        .await;

    match services_result {
        Ok(services) => {
           (  StatusCode::OK,
            Json(json!({"services": services})))
        },
        Err(_) => {
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
    pub price: Option<f64>,
    pub duration: Option<i32>,
    pub category_id: Option<i32>,
    pub is_active: Option<bool>,
    pub provider_id: i32,
}

#[derive(Deserialize, Serialize)]
pub struct EditParams{
    pub provider_id: i32,
}

pub async fn edit_service(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<EditServiceParams>,
)-> impl IntoResponse{
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to start transaction"})));
        }
    };

    //check if the user is a provider
    let provider_results = sqlx::query!(
            "SELECT id from providers WHERE user_id = $1 AND id = $2",
            user_id.parse::<i32>().unwrap(),
            payload.provider_id
        ).fetch_optional(&mut *tx).await;

        match provider_results {
            Ok(Some(_)) => {},
            Ok(None) => {
                return (StatusCode::BAD_REQUEST, Json(json!({"message": "You are not authorized to edit this service"})));
            },
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to fetch provider"})));
            }
        };

        //if so proceed to update the service
     let service_update_result = sqlx::query!(
        r#"
        UPDATE services
        SET title = COALESCE($1, title),
            description = COALESCE($2, description),
            price = COALESCE($3, price),
            duration = COALESCE($4, duration),
            category_id = COALESCE($5, category_id),
            is_active = COALESCE($6, is_active)
        WHERE id = $7 AND provider_id = $8
        RETURNING id
        "#,
        payload.title,
        payload.description,
        payload.price,
        payload.duration,
        payload.category_id,
        payload.is_active,
        payload.service_id,
        payload.provider_id
     ).fetch_one(&mut *tx).await;

    let service_id = match service_update_result {
        Ok(record) => record.id,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to update service"})));
        }
    };

    //commit the transaction
    if let Err(_)= tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to commit transaction"})));
    }

    (
        StatusCode::OK,
        Json(json!({"message": "Service updated successfully", "service_id": service_id})),
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
)-> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to start transaction"})));
        }
    };

    //check if the user is a provider
    let provider_results = sqlx::query!(
        "SELECT id FROM service_providers WHERE user_id = $1",
        user_id.parse::<i32>().unwrap()
    ).fetch_optional(&mut *tx).await;

    match provider_results {
        Ok(Some(provider)) => provider,
        Ok(None) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"message": "You are not authorized to delete this service"})));
        },
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Failed to fetch provider"})));
        }
    };

    //delete the service
    let delete_result = sqlx::query!("DELETE FROM services WHERE id = $1 RETURNING id", payload.service_id)
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

//upload attachments to the service
pub async fn upload_service_attachments (
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(service_id): Path<i32>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let permission_check  = sqlx::query!(
        "SELECT s.id FROM services s JOIN providersp ON s.provider_id = p.id 
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

