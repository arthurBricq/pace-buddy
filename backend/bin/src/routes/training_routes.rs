use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::helpers::insight_builder;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::Training;

fn parse_date(s: &str) -> Result<DateTime<Utc>, String> {
    s.parse::<DateTime<Utc>>()
        .map_err(|e| format!("Invalid date '{}': {}", s, e))
}

#[derive(Deserialize)]
pub struct CreateTrainingRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub race_distance: Option<String>,
    pub race_objectif: Option<String>,
}

pub async fn create_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CreateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /trainings user={} name={}", user.user_id, body.name);

    let start_date_raw = body.start_date.as_deref().ok_or_else(|| {
        AppError(domain::DomainError::BadRequest(
            "start_date is required for trainings".into(),
        ))
    })?;
    let end_date_raw = body.end_date.as_deref().ok_or_else(|| {
        AppError(domain::DomainError::BadRequest(
            "end_date is required for trainings".into(),
        ))
    })?;

    let start_date = parse_date(start_date_raw)
        .map_err(|e| AppError(domain::DomainError::BadRequest(e)))?;
    let end_date = parse_date(end_date_raw)
        .map_err(|e| AppError(domain::DomainError::BadRequest(e)))?;
    if start_date >= end_date {
        return Err(AppError(domain::DomainError::BadRequest(
            "start_date must be before end_date".into(),
        )));
    }

    let training = Training {
        id: Uuid::new_v4(),
        user_id: user.user_id,
        name: body.name.clone(),
        description: body.description.clone(),
        start_date: Some(start_date),
        end_date: Some(end_date),
        race_distance: body.race_distance.clone(),
        race_objectif: body.race_objectif.clone(),
        created_at: Utc::now(),
    };

    state.storage.create_training(&training).await?;

    Ok(HttpResponse::Created().json(training))
}

pub async fn list_trainings(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /trainings user={}", user.user_id);

    let trainings = state.storage.list_trainings(user.user_id).await?;

    Ok(HttpResponse::Ok().json(trainings))
}

pub async fn get_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("GET /trainings/{training_id} user={}", user.user_id);

    let training = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(training))
}

#[derive(Deserialize)]
pub struct UpdateTrainingRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub race_distance: Option<String>,
}

pub async fn update_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("PATCH /trainings/{training_id} user={}", user.user_id);

    let current = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    let name = body.name.clone().unwrap_or(current.name);
    let description = body.description.clone().or(current.description);
    let start_date = match &body.start_date {
        Some(s) => Some(parse_date(s).map_err(|e| AppError(domain::DomainError::BadRequest(e)))?),
        None => current.start_date,
    };
    let end_date = match &body.end_date {
        Some(s) => Some(parse_date(s).map_err(|e| AppError(domain::DomainError::BadRequest(e)))?),
        None => current.end_date,
    };
    let race_distance = body.race_distance.clone().or(current.race_distance);

    state
        .storage
        .update_training(
            training_id,
            user.user_id,
            name,
            description,
            start_date,
            end_date,
            race_distance,
        )
        .await?;

    let updated = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(updated))
}

pub async fn delete_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("DELETE /trainings/{training_id} user={}", user.user_id);

    state
        .storage
        .delete_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn get_training_activities(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!(
        "GET /trainings/{training_id}/activities user={}",
        user.user_id
    );

    let activities = state
        .storage
        .get_training_activities(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(activities))
}

pub async fn get_activity_trainings(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
    log::info!(
        "GET /activities/{activity_id}/trainings user={}",
        user.user_id
    );

    let trainings = state
        .storage
        .get_activity_trainings(activity_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(trainings))
}

// ---------------------------------------------------------------------------
// Training insight (LLM)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct InsightRequest {
    pub prompt_type: String,
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct InsightResponse {
    pub id: String,
    pub display_label: String,
    pub full_prompt: String,
    pub response: String,
}

pub async fn training_insight(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<InsightRequest>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!(
        "POST /trainings/{training_id}/insight user={} type={}",
        user.user_id,
        body.prompt_type
    );

    let llm_client = state
        .llm_client
        .as_ref()
        .ok_or_else(|| AppError(domain::DomainError::Internal("LLM not configured".into())))?;

    // Quota check: require at least $0.25 to make a request
    let quota = state.storage.get_user_quota(user.user_id).await?;
    if quota < 0.25 {
        return Err(AppError(domain::DomainError::QuotaExhausted(
            "Your AI token quota is too low. Request more tokens from your profile.".into(),
        )));
    }

    let context = insight_builder::build_insight_context(
        &state,
        user.user_id,
        training_id,
        &body.prompt_type,
    )
    .await
    .map_err(AppError)?;

    let model = body
        .model
        .as_deref()
        .unwrap_or(insight_builder::DEFAULT_LLM_MODEL);

    let insight = insight_builder::generate_insight(
        &state.storage,
        llm_client.as_ref(),
        &context,
        user.user_id,
        &body.prompt_type,
        model,
    )
    .await
    .map_err(AppError)?;

    // Deduct quota
    if let Some(cost) = insight.cost {
        let charge = state.cost_to_user_quota(cost);
        let _ = state.storage.deduct_quota(user.user_id, charge).await;
    }

    Ok(HttpResponse::Ok().json(InsightResponse {
        id: insight.id.to_string(),
        display_label: insight.display_label,
        full_prompt: insight.full_prompt,
        response: insight.response,
    }))
}

// ---------------------------------------------------------------------------
// List training insights
// ---------------------------------------------------------------------------

pub async fn list_training_insights(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::debug!(
        "GET /trainings/{training_id}/insights user={}",
        user.user_id
    );

    // Verify training ownership
    let _training = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    let mut insights = state
        .storage
        .get_training_insights(training_id, user.user_id)
        .await?;

    // Apply markup so displayed costs reflect what users are charged
    for insight in &mut insights {
        insight.cost = insight.cost.map(|c| state.cost_to_user_quota(c));
    }

    Ok(HttpResponse::Ok().json(insights))
}
