use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, SaltString
    },
    Argon2,
};
use axum::{
    extract::{Json, State},
};
use serde::Deserialize;
use sqlx::{PgPool, query};

#[derive(Deserialize)]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
}

pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterInput>,
) -> String {
    // 🔐 Hash the password using Argon2
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hashed_password = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .unwrap()
        .to_string(); // <- convert to string for storing

    // 📥 Insert into DB
    let result = query!(
        "INSERT INTO users (username, email, password) VALUES ($1, $2, $3)",
        payload.username,
        payload.email,
        hashed_password
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => "✅ User registered successfully".to_string(),
        Err(e) => format!("❌ Failed to register user: {}", e),
    }
}
