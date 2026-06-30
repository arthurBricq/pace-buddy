use actix_web::{dev::Payload, FromRequest, HttpRequest};
use std::future::{ready, Ready};
use uuid::Uuid;

use crate::state::AppState;
use domain::DomainError;

pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = extract_user(req);
        ready(result.map_err(|e| {
            let err: crate::errors::AppError = e.into();
            err.into()
        }))
    }
}

fn extract_user(req: &HttpRequest) -> Result<AuthenticatedUser, DomainError> {
    let state = req
        .app_data::<actix_web::web::Data<AppState>>()
        .ok_or_else(|| DomainError::Internal("App state not found".into()))?;

    let cookie = req
        .cookie("session")
        .ok_or_else(|| DomainError::Unauthorized("No session cookie".into()))?;

    let user_id = state
        .jwt
        .verify_token(cookie.value())
        .map_err(|_| DomainError::Unauthorized("Invalid session".into()))?;

    Ok(AuthenticatedUser { user_id })
}
