use actix_web::{web, HttpResponse};
use llm::LlmClient;
use storage::Storage;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::DomainError;

pub async fn list_models(
    state: web::Data<AppState>,
    _user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /llm/models");
    let llm_client = state
        .llm_client
        .as_ref()
        .ok_or_else(|| AppError(DomainError::Internal("LLM not configured".into())))?;

    let models = llm_client
        .list_models()
        .await
        .map_err(|e| AppError(DomainError::Internal(format!("Failed to list models: {e}"))))?;

    Ok(HttpResponse::Ok().json(models))
}

pub async fn list_model_cost_tiers(
    state: web::Data<AppState>,
    _user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /llm/models/cost-tiers");
    let tiers = state.storage.list_model_cost_tiers().await?;
    Ok(HttpResponse::Ok().json(tiers))
}
