use axum::extract::Multipart;
use uuid::Uuid;
use std::path::Path;
use tokio::fs;

pub async fn save_image_to_fs(mut multipart: Multipart, upload_dir: &str) -> Result<String, String> {
    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let file_name = field.file_name().ok_or("Missing file name")?.to_string();
        let extension = Path::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");

        let unique_name = format!("{}.{}", Uuid::new_v4(), extension);
        let path = format!("{}/{}", upload_dir, unique_name);

        let data = field.bytes().await.map_err(|e| e.to_string())?;

        fs::create_dir_all(upload_dir).await.map_err(|e| e.to_string())?;
        fs::write(&path, &data).await.map_err(|e| e.to_string())?;

        return Ok(unique_name);
    }

    Err("No file uploaded".to_string())
}