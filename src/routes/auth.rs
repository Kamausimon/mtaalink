use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{
    Router,
    extract::{Json, State},
    routing::post,
};
use serde::Deserialize;
use sqlx::{PgPool, query};
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
}

pub fn auth_routes(pool:PgPool) -> Router {
    Router::new().route("/register", post(register)).with_state(pool)
    // You can add more routes here in the future
}

pub async fn register(State(pool): State<PgPool>, Json(payload): Json<RegisterInput>) -> String {
    //confirm that passwords match
    if payload.password != payload.confirm_password {
        return "❌ Passwords do not match".to_string();
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
        return "❌ User already exists with this username or email".to_string();
    }

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
        Err(e) => "❌ Error registering user: ".to_string(),
    }
}
