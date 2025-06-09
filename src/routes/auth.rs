use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::save_image_to_fs;
use crate::utils::jwt::create_jwt;
use argon2::{
    Argon2, PasswordVerifier,
    password_hash::{PasswordHash, PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{
    Router,
    extract::{Json, State,Multipart},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, query};
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct RegisterInput {
    #[validate(length(min = 3, max = 32))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
    pub confirm_password: String,

    pub role: String, // This can be "client", "provider", or "business"

    pub service_description: Option<String>, // Optional service description field
    pub business_name: Option<String>,       // Optional business name field
}

pub fn auth_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login_handler))
        .route("/me", get(me))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .with_state(pool.clone())
    // You can add more routes here in the future
}

pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterInput>,
) -> impl IntoResponse {
    //confirm that passwords match
    if payload.password != payload.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Passwords do not match" })),
        );
    }
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Validation failed", "errors": e.to_string() })),
        );
    }

    //check if the user already exists
    let existing_user = sqlx::query_scalar!(
        "SELECT 1 FROM users WHERE username = $1 OR email = $2",
        payload.username,
        payload.email
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    if existing_user.is_some() {
        return (
            StatusCode::CONFLICT,
            Json(json!({"message": "User with this username or email already exists"})),
        );
    }

    // 🔐 Hash the password using Argon2
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hashed_password = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .unwrap()
        .to_string(); // <- convert to string for storing

    // 📥 Insert into DB
    let user = sqlx::query!(
        "INSERT INTO users (username, email, password) VALUES ($1, $2, $3) RETURNING id",
        payload.username,
        payload.email,
        hashed_password
    )
    .fetch_one(&pool)
    .await;

    match user {
        Ok(_) => Json(json!({"message": "User registered successfully"})),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": "Error creating user"})),
            );
        }
    };

    let user_id = match user {
        Ok(record) => record.id,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": "Error creating user"})),
            );
        }
    };

    //insert role-specific data
    let role_result = match payload.role.as_str() {
        "client" => {
            sqlx::query!("INSERT INTO clients (user_id) VALUES ($1)", user_id)
                .execute(&pool)
                .await
        }

        "provider" => {
            sqlx::query!(
                "INSERT INTO providers (user_id, service_description) VALUES ($1, $2)",
                user_id,
                payload.service_description
            )
            .execute(&pool)
            .await
        }
        "business" => {
            sqlx::query!(
                "INSERT INTO businesses (user_id, business_name) VALUES ($1, $2)",
                user_id,
                payload.business_name
            )
            .execute(&pool)
            .await
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"message": "Invalid role"})),
            );
        }
    };

    if role_result.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"message": "Error creating role-specific data"})),
        );
    }

    //successfully registered
    (
        StatusCode::CREATED,
        Json(json!({
            "message": "User registered successfully",
            "user_id": user_id,
            "username": payload.username,
            "role": payload.role
        })),
    )
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login_handler(
    State(db): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let user = sqlx::query!(
        "SELECT id, username, password FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&db)
    .await
    .unwrap();

    if let Some(user) = user {
        let parsed_hash = PasswordHash::new(&user.password).unwrap();

        if Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            let token = create_jwt(&user.id.to_string());
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Login successful",
                    "token": token,
                    "user_id": user.id,
                    "username": user.username
                })),
            );
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "message": "Invalid email or password"
        })),
    )
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
) -> impl IntoResponse {
    let user = sqlx::query_as!(
        UserWithRole,
        r#"SELECT 
          u.id, 
          u.email,
          u.username,
          CASE 
            WHEN c.id IS NOT NULL THEN 'client'
            WHEN p.id IS NOT NULL THEN 'provider'
            when b.id IS NOT NULL THEN 'business'
            ELSE 'unknown'
            END AS role
            FROM users u
            LEFT JOIN clients c ON u.id = c.user_id
            LEFT JOIN providers p on u.id = p.user_id
            Left JOIN businesses b ON u.id = b.user_id
            WHERE u.id = $1
            "#,
        user_id.parse::<i32>().unwrap()
    )
    .fetch_optional(&pool)
    .await;

    match user {
        Ok(Some(user)) => Json(json!({
            "id" : user_id,
            "username" :  user.username,
            "email" : user.email,
            "role" : user.role.unwrap_or("unknown".to_string()),
        })),

        Ok(None) => Json(json!({
            "message": "User not found"
        })),
        Err(_) => Json(json!({
            "message": "Error fetching user data"
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

pub async fn forgot_password(
    State(pool): State<PgPool>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> impl IntoResponse {
    // 1. Look up the user
    let user = sqlx::query!("SELECT id FROM users WHERE email = $1", payload.email)
        .fetch_optional(&pool)
        .await;

    match user {
        Ok(Some(user)) => {
            // 2. Generate reset token
            let token = Uuid::new_v4().to_string();
            let expiry = (Utc::now() + Duration::minutes(15)).naive_utc();

            // 3. Store in password_resets
            let _ = sqlx::query!(
                "INSERT INTO password_resets (user_id, token, expires_at) VALUES ($1, $2, $3)",
                user.id,
                token,
                expiry
            )
            .execute(&pool)
            .await;

            // 4. Return token for now (in real app you'd send via email)
            (
                StatusCode::OK,
                Json(json!({
                    "message": "Password reset link sent",
                    "token": token
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "message": "User not found" })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": "Database error" })),
        ),
    }
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
) -> impl IntoResponse {
    //validate the passwords
    if payload.password != payload.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Passwords do not match" })),
        );
    }

    //check if the token exists and is valid
    let record = sqlx::query!(
        "SELECT user_id, expires_at FROM  password_resets WHERE token = $1",
        payload.token
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    let Some(reset) = record else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "message": "Invalid or expired token" })),
        );
    };

    if reset.expires_at < Utc::now().naive_utc() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "message": "Token has expired" })),
        );
    }

    // Hash the new password
    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    //update the  user passwrd
    let _ = sqlx::query!(
        "UPDATE users SET password = $1 WHERE id = $2",
        hashed_password,
        reset.user_id
    )
    .execute(&pool)
    .await;

    //delete the reset tokem
    let _ = sqlx::query!(
        "DELETE FROM password_resets where token = $1",
        payload.token
    )
    .execute(&pool)
    .await;

    //successfully reset the password
    (
        StatusCode::OK,
        Json(json!({ "message": "Password reset successfully" })),
    )
}

