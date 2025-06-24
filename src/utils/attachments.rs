use crate::extractors::current_user::CurrentUser;
use axum::{
    Router,
    extract::{Json, Multipart, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::fs::File;
use std::io::Write;
use uuid::Uuid;
//allow users to upload images and videos

pub fn attachments_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/uploadAttachments", post(upload_attachments))
        .with_state(pool)
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct AttachmentParams {
    pub target_type: String,
    pub target_id: i32,
    pub uploaded_by: i32,
}

pub async fn upload_attachments(
    State(pool): State<PgPool>,
    Query(params): Query<AttachmentParams>,
    CurrentUser { user_id }: CurrentUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let target_type = params.target_type;
    let target_id = params.target_id;
    let uploaded_by = params.uploaded_by;
    let created_at = NaiveDateTime::from_timestamp(chrono::Utc::now().timestamp(), 0);

    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let data = field.bytes().await.unwrap();
        if data.is_empty() {
            return (StatusCode::BAD_REQUEST, "File is empty".to_string()).into_response();
        }
        let extension = file_name
            .split('.')
            .last()
            .and_then(|ext| Some(ext.to_lowercase()))
            .unwrap_or("bin".to_string());

        let file_type = match extension.as_ref() {
            "jpg" | "jpeg" | "png" | "gif" => "image",
            "mp4" | "avi" | "mov" => "video",
            _ => "other",
        };

        if file_type == "other" {
            continue; // Skip unsupported file types
        }

        let unique_name = format!("{}.{}", Uuid::new_v4(), file_name);
        let path = format!("attachments/{}", unique_name);

        let mut file = File::create(&path).unwrap();
        file.write_all(&data).unwrap();

        // Save the file metadata to the database
        let query = sqlx::query!(
            "INSERT INTO attachments (file_name, file_path, file_type, target_type,target_id, uploaded_by,created_at) VALUES ($1, $2, $3, $4, $5,$6,$7 )",
            unique_name,
            path,
            file_type,
            target_type,
            target_id,
            uploaded_by,
            created_at
        ).execute(&pool)
        .await.unwrap();
    }
    (
        StatusCode::OK,
        Json(json!({"message": "Files uploaded successfully"})),
    )
        .into_response()
    // Return a success response
}

//retrieve the attchments uploaded by  provider or business
#[derive(Deserialize, Serialize)]
pub struct AttachmentQuery {
    pub target_type: String,
    pub target_id: i32,
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct SerializableAttachment {
    pub id: i32,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    pub post_id: Option<i32>,
    pub target_type: String,
    pub target_id: i32,
    pub created_at: Option<NaiveDateTime>,
}

pub async fn get_attachments(
    State(pool): State<PgPool>,
    Query(params): Query<AttachmentQuery>,
) -> impl IntoResponse {
    let target_type = params.target_type;
    let target_id = params.target_id;
    let attachments = sqlx::query_as!(
    SerializableAttachment,
    "SELECT id, file_name, file_path, file_type, post_id, target_type, target_id, created_at FROM attachments WHERE target_type = $1 AND target_id = $2",
    target_type,
    target_id,   
  ).fetch_all(&pool).await;

    match attachments {
        Ok(rows) => {
            let attactments_vec: Vec<SerializableAttachment> = rows
                .into_iter()
                .map(|row| SerializableAttachment {
                    id: row.id,
                    file_name: row.file_name,
                    file_path: row.file_path,
                    file_type: row.file_type,
                    post_id: row.post_id,
                    target_type: row.target_type,
                    target_id: row.target_id,
                    created_at: row.created_at,
                })
                .collect();

            (
                StatusCode::OK,
                Json(json!({ "attachments": attactments_vec })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to fetch attachments: {}", e) })),
        ),
    }
}
