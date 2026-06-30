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

/// Claims used for OAuth state tokens (e.g. Strava login/link callbacks).
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthStateClaims {
    /// Purpose of this state token (`strava_login` or `strava_link`).
    pub purpose: String,
    /// Optional app user id (present for `strava_link` flow).
    pub user_id: Option<String>,
    /// Optional invite code hash carried from login start to callback.
    pub invite_code_hash: Option<String>,
    /// Unique nonce to prevent accidental token reuse collisions.
    pub nonce: String,
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

    /// Create a signed OAuth state token for Strava login.
    pub fn create_strava_login_state(
        &self,
        invite_code_hash: Option<String>,
    ) -> Result<String, DomainError> {
        self.create_oauth_state("strava_login", None, invite_code_hash)
    }

    /// Create a signed OAuth state token for Strava account linking.
    pub fn create_strava_link_state(&self, user_id: Uuid) -> Result<String, DomainError> {
        self.create_oauth_state("strava_link", Some(user_id), None)
    }

    /// Verify an OAuth state token and return claims.
    pub fn verify_oauth_state(&self, token: &str) -> Result<OAuthStateClaims, DomainError> {
        let token_data =
            decode::<OAuthStateClaims>(token, &self.decoding_key, &Validation::default())
                .map_err(|e| DomainError::Auth(format!("OAuth state decode error: {e}")))?;
        Ok(token_data.claims)
    }

    fn create_oauth_state(
        &self,
        purpose: &str,
        user_id: Option<Uuid>,
        invite_code_hash: Option<String>,
    ) -> Result<String, DomainError> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::minutes(10))
            .ok_or_else(|| {
                DomainError::Auth("Failed to compute oauth state expiration".to_string())
            })?;

        let claims = OAuthStateClaims {
            purpose: purpose.to_string(),
            user_id: user_id.map(|id| id.to_string()),
            invite_code_hash,
            nonce: Uuid::new_v4().to_string(),
            exp: expiration.timestamp() as usize,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| DomainError::Auth(format!("OAuth state encode error: {e}")))
    }
}
