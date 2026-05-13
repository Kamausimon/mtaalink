use axum::{
    Json,
    async_trait,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use serde_json::json;

use crate::utils::jwt::decode_jwt;

pub struct CurrentUser {
    pub user_id: i32,
}

type AuthRejection = (StatusCode, Json<serde_json::Value>);

fn auth_error(msg: &'static str) -> AuthRejection {
    (StatusCode::UNAUTHORIZED, Json(json!({ "message": msg })))
}

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, AuthRejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| auth_error("Missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| auth_error("Invalid Authorization header format"))?;

        let claims = decode_jwt(token)
            .map_err(|_| auth_error("Invalid or expired token"))?;

        let user_id = claims
            .sub
            .parse::<i32>()
            .map_err(|_| auth_error("Invalid token subject"))?;

        Ok(CurrentUser { user_id })
    }
}
