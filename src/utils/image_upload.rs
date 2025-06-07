use axum::extract::Multipart;
use std::fs;
use uuid::Uuid;
use std::path::PathBuf;

pub fn async upload_image(mut multipart: Multipart, upload_dir: &str) -> Result<String,String> {
    while let Some(field) = multipart.next_field().await.unwrap(){
        let file_name = field.file_name().unwrap_or("upload.jpg"); 
        let unique_name = format!("{}_{}", Uuid::new_v4(), file_name);
        let file_path = PathBuf::from(format!("{}/{}", upload_dir, unique_name ));

        let data = field.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&file_path, &data).map_err(|e| e.to_string())?;

        return Ok(unique_name);
    }
    Ok("File uploaded successfully".into())
}