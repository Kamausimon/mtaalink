use crate::errors::{AppError, AppResult};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

// ── Shared type alias ─────────────────────────────────────────────────────────

pub type SharedStorage = Arc<AppStorage>;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Generate a unique storage key. Returns e.g. `providers/profile_photos/<uuid>.jpg`
pub fn generate_key(prefix: &str, extension: &str) -> String {
    format!("{}/{}.{}", prefix, Uuid::new_v4(), extension)
}

// ── Local filesystem backend ──────────────────────────────────────────────────

#[derive(Clone)]
pub struct LocalStorage {
    base_dir: String, // "uploads"
    base_url: String, // "/uploads"
}

impl LocalStorage {
    async fn save(&self, key: &str, data: &Bytes) -> AppResult<String> {
        let full_path = format!("{}/{}", self.base_dir, key);
        let dir = Path::new(&full_path)
            .parent()
            .and_then(|p| p.to_str())
            .ok_or_else(|| AppError::Internal("Invalid storage key".to_string()))?;
        fs::create_dir_all(dir).await?;
        fs::write(&full_path, data).await?;
        Ok(format!("{}/{}", self.base_url, key))
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let _ = fs::remove_file(format!("{}/{}", self.base_dir, key)).await;
        Ok(())
    }
}

// ── AWS S3 backend ────────────────────────────────────────────────────────────

pub struct S3Storage {
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl S3Storage {
    fn from_env() -> AppResult<Self> {
        let region = env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let bucket = env::var("AWS_S3_BUCKET")
            .map_err(|_| AppError::Internal("AWS_S3_BUCKET not set".to_string()))?;
        let access_key = env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| AppError::Internal("AWS_ACCESS_KEY_ID not set".to_string()))?;
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| AppError::Internal("AWS_SECRET_ACCESS_KEY not set".to_string()))?;
        let base_url = env::var("AWS_S3_BASE_URL")
            .unwrap_or_else(|_| format!("https://{}.s3.{}.amazonaws.com", bucket, region));

        Ok(S3Storage {
            bucket,
            region,
            access_key,
            secret_key,
            base_url,
            client: reqwest::Client::new(),
        })
    }

    /// AWS Signature Version 4 — signs a PUT or DELETE request.
    fn sign(
        &self,
        method: &str,
        key: &str,
        body_hash: &str,
        datetime: &str,          // "20240101T120000Z"
        date: &str,              // "20240101"
        content_type: &str,
    ) -> String {
        let host = format!("{}.s3.{}.amazonaws.com", self.bucket, self.region);
        let service = "s3";
        let scope = format!("{}/{}/{}/aws4_request", date, self.region, service);

        // 1. Canonical request
        let canonical_headers = format!(
            "content-type:{}\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            content_type, host, body_hash, datetime
        );
        let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{}\n/{}\n\n{}\n{}\n{}",
            method, key, canonical_headers, signed_headers, body_hash
        );

        // 2. String to sign
        let cr_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{}\n{}\n{}", datetime, scope, cr_hash);

        // 3. Signing key  (HMAC chain)
        let signing_key = {
            let k = format!("AWS4{}", self.secret_key);
            let k1 = hmac_sha256(k.as_bytes(), date.as_bytes());
            let k2 = hmac_sha256(&k1, self.region.as_bytes());
            let k3 = hmac_sha256(&k2, service.as_bytes());
            hmac_sha256(&k3, b"aws4_request")
        };

        // 4. Signature
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        format!(
            "AWS4-HMAC-SHA256 Credential={}/{},SignedHeaders={},Signature={}",
            self.access_key, scope, signed_headers, signature
        )
    }

    async fn save(&self, key: &str, data: &Bytes, content_type: &str) -> AppResult<String> {
        let url = format!("{}/{}", self.base_url, key);
        let body_hash = hex::encode(Sha256::digest(data));
        let now = chrono::Utc::now();
        let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();

        let auth = self.sign("PUT", key, &body_hash, &datetime, &date, content_type);
        let host = format!("{}.s3.{}.amazonaws.com", self.bucket, self.region);

        let resp = self
            .client
            .put(&url)
            .header("Host", &host)
            .header("Content-Type", content_type)
            .header("x-amz-date", &datetime)
            .header("x-amz-content-sha256", &body_hash)
            .header("Authorization", auth)
            .body(data.clone())
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 upload failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("S3 upload error {}: {}", status, body)));
        }

        Ok(format!("{}/{}", self.base_url, key))
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let url = format!("{}/{}", self.base_url, key);
        let body_hash = hex::encode(Sha256::digest(b""));
        let now = chrono::Utc::now();
        let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();

        let auth = self.sign("DELETE", key, &body_hash, &datetime, &date, "");
        let host = format!("{}.s3.{}.amazonaws.com", self.bucket, self.region);

        self.client
            .delete(&url)
            .header("Host", &host)
            .header("Content-Type", "")
            .header("x-amz-date", &datetime)
            .header("x-amz-content-sha256", &body_hash)
            .header("Authorization", auth)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 delete failed: {}", e)))?;

        Ok(())
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

// ── Unified enum ──────────────────────────────────────────────────────────────

pub enum AppStorage {
    Local(LocalStorage),
    S3(S3Storage),
}

impl AppStorage {
    /// Initialise from `STORAGE_BACKEND` env var ("s3" or "local"/unset).
    pub fn init() -> Self {
        match env::var("STORAGE_BACKEND").as_deref() {
            Ok("s3") => match S3Storage::from_env() {
                Ok(s3) => {
                    tracing::info!("Storage backend: AWS S3");
                    AppStorage::S3(s3)
                }
                Err(e) => {
                    tracing::error!("S3 init failed ({}); falling back to local storage", e);
                    AppStorage::Local(LocalStorage {
                        base_dir: "uploads".to_string(),
                        base_url: "/uploads".to_string(),
                    })
                }
            },
            _ => {
                tracing::info!("Storage backend: local disk (uploads/)");
                AppStorage::Local(LocalStorage {
                    base_dir: "uploads".to_string(),
                    base_url: "/uploads".to_string(),
                })
            }
        }
    }

    /// Save `data` at `key` and return a public URL.
    /// `content_type` is used only by the S3 backend.
    pub async fn save(&self, key: &str, data: &Bytes) -> AppResult<String> {
        match self {
            AppStorage::Local(s) => s.save(key, data).await,
            AppStorage::S3(s) => s.save(key, data, "application/octet-stream").await,
        }
    }

    pub async fn save_with_type(&self, key: &str, data: &Bytes, content_type: &str) -> AppResult<String> {
        match self {
            AppStorage::Local(s) => s.save(key, data).await,
            AppStorage::S3(s) => s.save(key, data, content_type).await,
        }
    }

    /// Delete the object at `key`.
    pub async fn delete(&self, key: &str) -> AppResult<()> {
        match self {
            AppStorage::Local(s) => s.delete(key).await,
            AppStorage::S3(s) => s.delete(key).await,
        }
    }
}
