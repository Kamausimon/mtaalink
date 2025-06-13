use axum::extract::Multipart;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

pub async fn save_image_to_fs(
    mut multipart: Multipart,
    upload_dir: &str,
) -> Result<String, String> {
    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let file_name = field.file_name().ok_or("Missing file name")?.to_string();
        if file_name.is_empty() {
            return Err("File name is empty".to_string());
        }
        println!("Received file: {}", file_name);
        // Extract the file extension or default to "jpg"
        let extension = Path::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");

        let unique_name = format!("{}.{}", Uuid::new_v4(), extension);
        println!("Saving file as: {}", unique_name);
        // Create the full path for the file
        let path = format!("{}/{}", upload_dir, unique_name);
        println!("Full path for saving: {}", path);

        let data = field.bytes().await.map_err(|e| e.to_string())?;
        // Ensure the upload directory exists
        if data.is_empty() {
            return Err("File is empty".to_string());
        }
        println!("File size: {} bytes", data.len());
        // Create the upload directory if it doesn't exist

        fs::create_dir_all(upload_dir)
            .await
            .map_err(|e| e.to_string())?;
        fs::write(&path, &data).await.map_err(|e| e.to_string())?;

        return Ok(unique_name);
    }

    Err("No file uploaded".to_string())
}
