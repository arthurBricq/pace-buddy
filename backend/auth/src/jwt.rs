use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use domain::DomainError;

/// JWT claims payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject -- the user ID encoded as a string.
    pub sub: String,
    /// Expiration time (as UTC timestamp in seconds).
    pub exp: usize,
}

/// Service for creating and verifying JWT tokens.
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// Create a new `JwtService` from a shared secret string.
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// Create a signed JWT token for the given `user_id`.
    ///
    /// The token expires in 7 days.
    pub fn create_token(&self, user_id: Uuid) -> Result<String, DomainError> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::days(7))
            .ok_or_else(|| DomainError::Auth("Failed to compute token expiration".to_string()))?;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration.timestamp() as usize,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| DomainError::Auth(format!("JWT encode error: {e}")))
    }

    /// Verify a JWT token and return the `user_id` it was issued for.
    ///
    /// Returns `DomainError::Auth` if the token is invalid or expired.
    pub fn verify_token(&self, token: &str) -> Result<Uuid, DomainError> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map_err(|e| DomainError::Auth(format!("JWT decode error: {e}")))?;

        Uuid::parse_str(&token_data.claims.sub)
            .map_err(|e| DomainError::Auth(format!("Invalid user_id in token: {e}")))
    }
}
