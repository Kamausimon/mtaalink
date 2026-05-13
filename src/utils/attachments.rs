use crate::errors::AppError;
use chrono::NaiveDateTime;
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tokio::fs;
use uuid::Uuid;

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
    CurrentUser { user_id: _ }: CurrentUser,
    mut multipart: axum::extract::Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let target_type = params.target_type;
    let target_id = params.target_id;
    let uploaded_by = params.uploaded_by;
    let created_at = chrono::Utc::now().naive_utc();

    fs::create_dir_all("uploads/attachments")
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create upload directory: {}", e)))?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read field: {}", e)))?;

        if data.is_empty() {
            return Err(AppError::BadRequest("File is empty".to_string()));
        }

        let extension = file_name
            .split('.')
            .last()
            .map(|ext| ext.to_lowercase())
            .unwrap_or_else(|| "bin".to_string());

        let file_type = match extension.as_str() {
            "jpg" | "jpeg" | "png" | "gif" => "image",
            "mp4" | "avi" | "mov" => "video",
            _ => continue,
        };

        let unique_name = format!("{}.{}", Uuid::new_v4(), file_name);
        let path = format!("uploads/attachments/{}", unique_name);

        fs::write(&path, &data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write file: {}", e)))?;

        let result = sqlx::query!(
            "INSERT INTO attachments (file_name, file_path, file_type, target_type, target_id, uploaded_by, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            unique_name,
            path,
            file_type,
            target_type,
            target_id,
            uploaded_by,
            created_at
        )
        .execute(&pool)
        .await;

        if let Err(e) = result {
            let _ = fs::remove_file(&path).await;
            return Err(AppError::Database(e));
        }
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Files uploaded successfully" }))))
}

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
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let attachments = sqlx::query_as!(
        SerializableAttachment,
        "SELECT id, file_name, file_path, file_type, post_id, target_type, target_id, created_at \
         FROM attachments WHERE target_type = $1 AND target_id = $2",
        params.target_type,
        params.target_id
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "attachments": attachments }))))
}
