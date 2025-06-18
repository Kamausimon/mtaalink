use axum::{
    extract::{Multipart, State,Json,Query},
    http::StatusCode,
    response::IntoResponse,
    Router,
    routing::post,
};
use std::path::Path;
use tokio::fs;
use uuid::Uuid;
use sqlx::Pool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::extractors::current_user::CurrentUser;
//allow users to upload images and videos

pub fn attchments_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/uploadAttachments", post(upload_attachments))
        .with_state(pool)
}

#[derive(Deserialize, Serialize)]
pub struct AttachmentParams {
    pub target_type: String,
    pub target_id: i32,
    pub uploaded_by: i32,
}

pub asyn fn upload_attachments(
    State(pool) : State<PgPool>,
    mut multipart: Multipart,
    CurrentUser{ user_id }: CurrentUser,
    Query(params): Query<AttachmentParams>,
)-> impl IntoResponse {
   let target_type = params.target_type;
   let target_id = params.target_id;
    let uploaded_by = params.uploaded_by;

    while let Some(field) = multipart.next_field().await.unwrap(){
        let file_name = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let content_type = field.content_type().map(|s| s.to_string()).unwrap_or_else(|| "application/octet-stream".to_string());
        let data = field.bytes().await.unwrap();
        if data.is_empty() {
            return (StatusCode::BAD_REQUEST, "File is empty".to_string()).into_response();
        }
        let extension = file_name
        .split('.')
        .last()
        .and_then(|ext| ext.to_lowercase().as_str())
        .unwrap_or("bin");

        let file_type = match extension {
            "jpg" | "jpeg" | "png" | "gif" => "image",
            "mp4" | "avi" | "mov" => "video",
            _ => "other",
        };

        if file_type = "other"{
            continue; // Skip unsupported file types
        }

        let unique_name = format!("{}.{}", Uuid::new_v4(), file_name);
        let path = format!("attachments/{}", unique_name);

        let mut file = File::create(&path).unwrap();
         file.write_all(&data).unwrap();

        // Save the file metadata to the database
        let query = sqlx::query!(
            "INSERT INTO attachments (file_name, file_path, file_type, target_type,target_id, uploaded_by,created_at) VALUES ($1, $2, $3, $4, $5, )",
            unique_name,
            path,
            file_type,
            target_type,
            target_id,
            uploaded_by,
            chrono::Utc::now()
        ).execute(&pool)
        .await.unwrap();
    }
    (StatusCode::OK, Json(json!({"message": "Files uploaded successfully"}))).into_response()
    // Return a success response
}


//retrieve the attchments uploaded by  provider or business
#[derive(Deserialize, Serialize)]
pub struct AttachmentQuery {
    pub target_type: String,
    pub target_id: i32,
}


pub async fn get_attachments(
    State(pool): State<PgPool>,
    Query(params): Query<AttachmentQuery>,
) -> impl IntoResponse {
    let target_type = params.target_type;
    let target_id = params.target_id;

    let attachments = sqlx::query!(
        "SELECT * FROM attachments WHERE target_type = $1 AND target_id = $2",
        target_type,
        target_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if attachments.is_empty() {
        return (StatusCode::NOT_FOUND, Json(json!({"message": "No attachments found"}))).into_response();
    }

    (StatusCode::OK, Json(attachments)).into_response()
}



