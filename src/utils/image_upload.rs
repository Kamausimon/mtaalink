use crate::errors::{AppError, AppResult};
use axum::extract::Multipart;
use bytes::Bytes;
use std::path::Path;

/// Parse the first file field from a multipart request.
/// Returns (data, lowercase_extension, content_type).
pub async fn parse_image_from_multipart(
    mut multipart: Multipart,
) -> AppResult<(Bytes, String, String)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let content_type = field
            .content_type()
            .map(|ct| ct.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload.jpg".to_string());

        let extension = Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_lowercase();

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        if data.is_empty() {
            return Err(AppError::BadRequest("File is empty".to_string()));
        }

        return Ok((data, extension, content_type));
    }

    Err(AppError::BadRequest("No file uploaded".to_string()))
}

/// Kept for backward compatibility — still used by legacy code paths.
pub async fn save_image_to_fs(
    mut multipart: Multipart,
    upload_dir: &str,
) -> Result<String, String> {
    use std::path::Path;
    use tokio::fs;
    use uuid::Uuid;

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let file_name = field.file_name().ok_or("Missing file name")?.to_string();
        if file_name.is_empty() {
            return Err("File name is empty".to_string());
        }
        let extension = Path::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");

        let unique_name = format!("{}.{}", Uuid::new_v4(), extension);
        let path = format!("{}/{}", upload_dir, unique_name);
        let data = field.bytes().await.map_err(|e| e.to_string())?;
        if data.is_empty() {
            return Err("File is empty".to_string());
        }
        fs::create_dir_all(upload_dir)
            .await
            .map_err(|e| e.to_string())?;
        fs::write(&path, &data).await.map_err(|e| e.to_string())?;
        return Ok(unique_name);
    }
    Err("No file uploaded".to_string())
}
