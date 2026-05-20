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

