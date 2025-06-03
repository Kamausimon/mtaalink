use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts. StatusCode},
};

use crate::utils::jwt::decode_jwt;

pub struct CurrentUser{
    pub user_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser 
WHERE S:Send + Sync, 
{
    async fn from_request_parts(parts: &mut Parts, _state:&S) -> Result<Self, StatusCode> {
        //extract the Authorization header
        let auth_header = parts.headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

        //extract the token from the header
        let token = auth_header.strip_prefix("Bearer")
        .ok_or(StatusCode::UNAUTHORIZED)?

        let claims = decode_wt(token).map_err(|_| StatusCode::UNAUTHORIZED)?;

        //return the current usero
        Ok(CurrentUser {
            user_id: claims.sub,
        })
    }
}