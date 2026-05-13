use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::email::{EmailConfig, password_reset_html, send_email};
use crate::utils::jwt::create_jwt;
use argon2::{
    Argon2, PasswordVerifier,
    password_hash::{PasswordHash, PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::{State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::env;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

pub fn auth_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login_handler))
        .route("/me", get(me))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .with_state(pool)
}

#[derive(Deserialize, Validate)]
pub struct RegisterInput {
    #[validate(length(min = 3, max = 32))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
    pub confirm_password: String,

    pub role: String,

    pub service_description: Option<String>,
    pub business_name: Option<String>,
}

fn normalize_email(email: &str) -> String {
    let normalized = email.trim().to_lowercase();
    if let Some(at_pos) = normalized.find('@') {
        let (username, domain) = normalized.split_at(at_pos);
        if domain == "@gmail.com" {
            return username.replace(".", "") + domain;
        }
    }
    normalized
}

fn normalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

pub async fn register(
    State(pool): State<PgPool>,
    Json(mut payload): Json<RegisterInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.email = normalize_email(&payload.email);
    payload.username = normalize_username(&payload.username);

    if payload.password != payload.confirm_password {
        return Err(AppError::BadRequest("Passwords do not match".to_string()));
    }
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let existing_user = sqlx::query_scalar!(
        "SELECT 1 FROM users WHERE username = $1 OR email = $2",
        payload.username,
        payload.email
    )
    .fetch_optional(&pool)
    .await?;

    if existing_user.is_some() {
        return Err(AppError::Conflict(
            "User with this username or email already exists".to_string(),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)?
        .to_string();

    let mut tx = pool.begin().await?;

    let user = sqlx::query!(
        "INSERT INTO users (username, email, password) VALUES ($1, $2, $3) RETURNING id",
        payload.username,
        payload.email,
        hashed_password
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        AppError::Internal(format!("Error creating user: {}", e))
    })?;

    let user_id = user.id;

    let role_result = match payload.role.as_str() {
        "client" => {
            sqlx::query!("INSERT INTO clients (user_id) VALUES ($1)", user_id)
                .execute(&mut *tx)
                .await
        }
        "provider" => {
            let service_description = payload
                .service_description
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "Service description is required for provider role".to_string(),
                    )
                })?
                .to_string();

            sqlx::query!(
                "INSERT INTO providers (user_id, service_description) VALUES ($1, $2)",
                user_id,
                service_description
            )
            .execute(&mut *tx)
            .await
        }
        "business" => {
            let business_name = payload.business_name.as_deref().ok_or_else(|| {
                AppError::BadRequest("Business name is required for business role".to_string())
            })?;

            sqlx::query!(
                "INSERT INTO businesses (user_id, business_name) VALUES ($1, $2)",
                user_id,
                business_name
            )
            .execute(&mut *tx)
            .await
        }
        _ => return Err(AppError::BadRequest("Invalid role".to_string())),
    };

    if let Err(e) = role_result {
        let _ = tx.rollback().await;
        return Err(AppError::Internal(format!("Error assigning role: {}", e)));
    }

    sqlx::query!(
        "UPDATE users SET role = $1::text WHERE id = $2",
        payload.role,
        user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        AppError::Internal(format!("Error updating user role: {}", e))
    })?;

    tx.commit().await?;

    let token = create_jwt(&user_id.to_string())?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "User registered successfully",
            "user_id": user_id,
            "username": payload.username,
            "role": payload.role,
            "token": token,
        })),
    ))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login_handler(
    State(db): State<PgPool>,
    Json(mut payload): Json<LoginRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    payload.email = normalize_email(&payload.email);

    let user = sqlx::query!(
        "SELECT id, username, password, role FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&db)
    .await?;

    if let Some(user) = user {
        let parsed_hash = PasswordHash::new(&user.password)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            let token = create_jwt(&user.id.to_string())?;
            return Ok((
                StatusCode::OK,
                Json(json!({
                    "message": "Login successful",
                    "token": token,
                    "user_id": user.id,
                    "username": user.username,
                    "role": user.role.unwrap_or_else(|| "unknown".to_string()),
                })),
            ));
        }
    }

    Err(AppError::Unauthorized("Invalid email or password".to_string()))
}

#[derive(Debug, sqlx::FromRow)]
struct UserWithRole {
    id: i32,
    email: String,
    username: String,
    role: Option<String>,
}

pub async fn me(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let user = sqlx::query_as!(
        UserWithRole,
        r#"SELECT
          u.id,
          u.email,
          u.username,
          CASE
            WHEN c.id IS NOT NULL THEN 'client'
            WHEN p.id IS NOT NULL THEN 'provider'
            WHEN b.id IS NOT NULL THEN 'business'
            ELSE 'unknown'
          END AS role
        FROM users u
        LEFT JOIN clients c ON u.id = c.user_id
        LEFT JOIN providers p ON u.id = p.user_id
        LEFT JOIN businesses b ON u.id = b.user_id
        WHERE u.id = $1"#,
        user_id
    )
    .fetch_optional(&pool)
    .await?;

    match user {
        Some(u) => Ok((
            StatusCode::OK,
            Json(json!({
                "id": user_id,
                "username": u.username,
                "email": u.email,
                "role": u.role.unwrap_or_else(|| "unknown".to_string()),
            })),
        )),
        None => Err(AppError::NotFound("User not found".to_string())),
    }
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

pub async fn forgot_password(
    State(pool): State<PgPool>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let user = sqlx::query!("SELECT id FROM users WHERE email = $1", payload.email)
        .fetch_optional(&pool)
        .await?;

    // Always return the same message to prevent email enumeration
    let Some(user) = user else {
        return Ok((
            StatusCode::OK,
            Json(json!({ "message": "If that email exists, a reset link has been sent" })),
        ));
    };

    let token = Uuid::new_v4().to_string();
    let expiry = (Utc::now() + Duration::minutes(15)).naive_utc();

    // Upsert: replace any existing reset token for this user
    sqlx::query!(
        "INSERT INTO password_resets (user_id, token, expires_at) VALUES ($1, $2, $3)
         ON CONFLICT (user_id) DO UPDATE SET token = EXCLUDED.token, expires_at = EXCLUDED.expires_at",
        user.id,
        token,
        expiry
    )
    .execute(&pool)
    .await?;

    let app_url = env::var("APP_URL").unwrap_or_else(|_| "http://localhost:7878".to_string());
    let reset_url = format!("{}/auth/reset-password?token={}", app_url, token);
    let html = password_reset_html(&reset_url, 15);

    if let Ok(config) = EmailConfig::from_env() {
        if let Err(e) = send_email(&config, &payload.email, "Reset your MtaaLink password", &html).await {
            tracing::error!("Failed to send password reset email: {}", e);
        }
    } else {
        tracing::warn!("Email not configured — skipping password reset email");
    }

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "If that email exists, a reset link has been sent" })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
    pub confirm_password: String,
}

pub async fn reset_password(
    State(pool): State<PgPool>,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.password != payload.confirm_password {
        return Err(AppError::BadRequest("Passwords do not match".to_string()));
    }

    let record = sqlx::query!(
        "SELECT user_id, expires_at FROM password_resets WHERE token = $1",
        payload.token
    )
    .fetch_optional(&pool)
    .await?;

    let reset = record.ok_or_else(|| {
        AppError::NotFound("Invalid or expired token".to_string())
    })?;

    if reset.expires_at < Utc::now().naive_utc() {
        return Err(AppError::Unauthorized("Token has expired".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)?
        .to_string();

    sqlx::query!(
        "UPDATE users SET password = $1 WHERE id = $2",
        hashed_password,
        reset.user_id
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        "DELETE FROM password_resets WHERE token = $1",
        payload.token
    )
    .execute(&pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Password reset successfully" })),
    ))
}
