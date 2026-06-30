use actix_web::{web, HttpResponse};
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{DomainError, SessionStatus};

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

pub async fn list_training_sessions(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, AppError> {
    let status = query
        .status
        .as_deref()
        .map(|s| {
            s.parse::<SessionStatus>()
                .map_err(|e| AppError(DomainError::BadRequest(e)))
        })
        .transpose()?;

    log::debug!(
        "GET /training-sessions user={} status={:?}",
        user.user_id,
        status
    );

    let sessions = state
        .storage
        .list_training_sessions(user.user_id, status)
        .await?;

    Ok(HttpResponse::Ok().json(sessions))
}

pub async fn get_training_session(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let session_id = path.into_inner();
    log::debug!("GET /training-sessions/{session_id} user={}", user.user_id);

    let session = state
        .storage
        .get_training_session(session_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(session))
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

pub async fn update_training_session_status(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateStatusRequest>,
) -> Result<HttpResponse, AppError> {
    let session_id = path.into_inner();
    let status: SessionStatus = body
        .status
        .parse()
        .map_err(|e: String| AppError(DomainError::BadRequest(e)))?;

    log::info!(
        "PATCH /training-sessions/{session_id}/status user={} status={status}",
        user.user_id
    );

    state
        .storage
        .update_training_session_status(session_id, user.user_id, status)
        .await?;

    let updated = state
        .storage
        .get_training_session(session_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(updated))
}
