use actix_web::{web, HttpResponse};
use chrono::Utc;
use domain::{
    DomainError, RunningCoachMemory, RunningCoachMessage, RunningCoachSettings, RunningCoachState,
};
use llm::LlmClient;
use serde::{Deserialize, Serialize};
use storage::Storage;

use crate::adapters::coach_tools::AppCoachToolExecutor;
use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;

const MIN_QUOTA: f64 = 0.25;

#[derive(Serialize)]
pub struct CoachResponse {
    pub settings: RunningCoachSettings,
    pub memory: RunningCoachMemory,
    pub state: RunningCoachState,
    pub messages: Vec<RunningCoachMessage>,
    pub total_cost: f64,
    pub total_tokens: u64,
}

#[derive(Deserialize)]
pub struct CoachSendMessageRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct CoachSettingsRequest {
    pub model: String,
    pub personality: String,
    pub consider_trail_runs_as_runs: bool,
    pub volume_weeks: i32,
    pub last_workouts_count: i32,
    pub last_long_runs_count: i32,
    pub last_races_count: i32,
    pub new_activities_count: i32,
    pub normalizer_every_n_messages: i32,
}

pub async fn get_coach(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let settings = state
        .storage
        .get_or_create_running_coach_settings(user.user_id)
        .await?;
    let memory = state
        .storage
        .get_or_create_running_coach_memory(user.user_id)
        .await?;
    let coach_state = state
        .storage
        .get_or_create_running_coach_state(user.user_id)
        .await?;
    let mut messages = state
        .storage
        .list_running_coach_messages(user.user_id, 300)
        .await?;

    let total_cost_real: f64 = messages.iter().map(|m| m.cost).sum();
    for message in &mut messages {
        message.cost = state.cost_to_user_quota(message.cost);
    }
    let total_tokens: u64 = messages.iter().map(|m| m.total_tokens as u64).sum();

    Ok(HttpResponse::Ok().json(CoachResponse {
        settings,
        memory,
        state: coach_state,
        messages,
        total_cost: state.cost_to_user_quota(total_cost_real),
        total_tokens,
    }))
}

pub async fn update_coach_settings(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CoachSettingsRequest>,
) -> Result<HttpResponse, AppError> {
    let mut settings = state
        .storage
        .get_or_create_running_coach_settings(user.user_id)
        .await?;

    let personality = body.personality.trim();
    if personality.is_empty() {
        return Err(AppError(DomainError::BadRequest(
            "Coach personality cannot be empty".to_string(),
        )));
    }
    let model = body.model.trim();
    if model.is_empty() {
        return Err(AppError(DomainError::BadRequest(
            "Coach model cannot be empty".to_string(),
        )));
    }

    settings.model = model.to_string();
    settings.personality = personality.to_string();
    settings.consider_trail_runs_as_runs = body.consider_trail_runs_as_runs;
    settings.volume_weeks = clamp(body.volume_weeks, 1, 24, 8);
    settings.last_workouts_count = clamp(body.last_workouts_count, 1, 25, 8);
    settings.last_long_runs_count = clamp(body.last_long_runs_count, 1, 25, 6);
    settings.last_races_count = clamp(body.last_races_count, 1, 25, 4);
    settings.new_activities_count = clamp(body.new_activities_count, 1, 25, 8);
    settings.normalizer_every_n_messages = clamp(body.normalizer_every_n_messages, 1, 20, 6);
    settings.updated_at = Utc::now();

    state
        .storage
        .upsert_running_coach_settings(&settings)
        .await?;

    Ok(HttpResponse::Ok().json(settings))
}

pub async fn reset_coach(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    state.storage.clear_running_coach_data(user.user_id).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })))
}

pub async fn send_coach_message(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CoachSendMessageRequest>,
) -> Result<HttpResponse, AppError> {
    let content = body.content.trim();
    if content.is_empty() {
        return Err(AppError(DomainError::BadRequest(
            "Message cannot be empty".to_string(),
        )));
    }

    let quota = state.storage.get_user_quota(user.user_id).await?;
    if quota < MIN_QUOTA {
        return Err(AppError(DomainError::QuotaExhausted(
            "Your AI token quota is too low. Request more tokens from your profile.".into(),
        )));
    }

    let llm_client = state
        .llm_client
        .as_ref()
        .ok_or_else(|| AppError(DomainError::Internal("LLM not configured".into())))?;
    let llm_arc: std::sync::Arc<dyn LlmClient> = llm_client.clone();
    let tool_executor = AppCoachToolExecutor::new(state.clone());

    let assistant_message = state
        .coach_memory
        .send_message_with_tools(llm_arc.as_ref(), user.user_id, content, &tool_executor)
        .await?;

    let billed_cost = state.cost_to_user_quota(assistant_message.cost);
    if billed_cost > 0.0 {
        if let Err(err) = state.storage.deduct_quota(user.user_id, billed_cost).await {
            log::error!("Failed to deduct quota for running coach: {}", err);
        }
    }

    let mut response_message = assistant_message;
    response_message.cost = billed_cost;
    Ok(HttpResponse::Ok().json(response_message))
}

fn clamp(value: i32, min: i32, max: i32, fallback: i32) -> i32 {
    let val = if value <= 0 { fallback } else { value };
    val.clamp(min, max)
}
