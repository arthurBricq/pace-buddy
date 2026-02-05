use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Strava API error: {0}")]
    StravaApi(String),

    #[error("Strava rate limited")]
    StravaRateLimited,

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Auth error: {0}")]
    Auth(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
