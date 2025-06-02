use jsonwebtoken::{encode, decode, Header, Validation,DecodingKey,EncodingKey};
use serde::{Serialize, Deserialize};
use std::env;


#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (usually user ID)
    pub exp: usize,  // Expiration time (as a UNIX timestamp)
}

 fn jwt_secret() -> Vec<u8> {
    env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set")
        .as_bytes().to_vec()   
 }
   

    pub fn create_jwt(user_id: &str) -> String{
        let expiration = chrono::Utc::now()
             .checked_add_signed(chrono::Duration::hours(24))
             .unwrap()
                .timestamp() as usize;

        let claims = Claims{
            sub: user_id.to_owned(),
            exp: expiration,
        };

        encode(&Header::default(), &claims, &EncodingKey::from_secret(&jwt_secret()))
            .expect("Failed to create JWT")
    }

    pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(&jwt_secret()),
            &Validation::default(),
        )
        .map(|data| data.claims)
    }