use jsonwebtoken::{encode, decode, Header, Validation, Encoding,DecodingKey};
use serde::{Serialize, Deserialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (usually user ID)
    pub exp: usize,  // Expiration time (as a UNIX timestamp)
}

const SECRET: &[u8] = env::var("JWT_SECRET")
    .expect("JWT_SECRET must be set")
    .as_bytes();

    pub fn create_jwt(user_id: &str) -> String{
        let expiration = chrono::utc::now()
             .checked_add_signed(chrono::Duration::hours(24))
             .unwrap()
                .timestamp() as usize;

        let claims = Claims{
            sub: user_id.to_owned(),
            exp: expiration,
        };

        encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET))
            .expect("Failed to create JWT")
    }

    pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(SECRET),
            &Validation::default(),
        )
        .map(|data| data.claims)
    }