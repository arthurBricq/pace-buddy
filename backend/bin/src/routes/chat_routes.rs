use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::helpers::context_builder::{self, ContextRequest};
use crate::helpers::conversation_manager;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{AiChat, DomainError};
use llm::LlmClient;

const DEFAULT_MODEL: &str = "google/gemini-2.5-flash";

// ---------------------------------------------------------------------------
// Create empty chat
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateChatRequest {
    pub title: String,
    pub model: Option<String>,
    pub training_id: Option<String>,
}

pub async fn create_chat(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CreateChatRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /chats user={} title={}", user.user_id, body.title);
    let now = Utc::now();
    let training_id = body
        .training_id
        .as_deref()
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|e| AppError(DomainError::BadRequest(format!("Invalid training_id: {e}"))))?;

    let chat = AiChat {
        id: Uuid::new_v4(),
        user_id: user.user_id,
        training_id,
        source_insight_id: None,
        title: body.title.clone(),
        model: body
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        conversation_length: None,
        created_at: now,
        updated_at: now,
    };
    state.storage.create_ai_chat(&chat).await?;

    Ok(HttpResponse::Created().json(chat))
}

// ---------------------------------------------------------------------------
// List chats
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ChatListItem {
    pub id: String,
    pub title: String,
    pub model: String,
    pub training_id: Option<String>,
    pub message_count: i64,
    pub total_cost: f64,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_chats(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /chats user={}", user.user_id);
    let chats = state.storage.list_ai_chats(user.user_id).await?;

    let mut items = Vec::new();
    for chat in chats {
        let messages = state.storage.get_ai_chat_messages(chat.id).await?;
        let message_count = messages.len() as i64;
        let total_cost: f64 = messages
            .iter()
            .map(|m| state.cost_to_user_quota(m.cost))
            .sum::<f64>();

        items.push(ChatListItem {
            id: chat.id.to_string(),
            title: chat.title,
            model: chat.model,
            training_id: chat.training_id.map(|id| id.to_string()),
            message_count,
            total_cost,
            created_at: chat.created_at.to_rfc3339(),
            updated_at: chat.updated_at.to_rfc3339(),
        });
    }

    Ok(HttpResponse::Ok().json(items))
}

// ---------------------------------------------------------------------------
// Get chat + messages
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ChatResponse {
    pub chat: AiChat,
    pub messages: Vec<domain::AiChatMessage>,
    pub total_cost: f64,
    pub total_tokens: u64,
}

pub async fn get_chat(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let chat_id = path.into_inner();
    log::debug!("GET /chats/{chat_id} user={}", user.user_id);
    let chat = state.storage.get_ai_chat(chat_id, user.user_id).await?;
    let mut messages = state.storage.get_ai_chat_messages(chat_id).await?;

    // Apply cost conversion so displayed costs reflect what users are charged
    for msg in &mut messages {
        msg.cost = state.cost_to_user_quota(msg.cost);
    }

    let total_cost: f64 = messages.iter().map(|m| m.cost).sum();
    let total_tokens: u64 = messages.iter().map(|m| m.total_tokens as u64).sum();

    Ok(HttpResponse::Ok().json(ChatResponse {
        chat,
        messages,
        total_cost,
        total_tokens,
    }))
}

// ---------------------------------------------------------------------------
// Update chat title
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UpdateChatRequest {
    pub title: String,
}

pub async fn update_chat(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateChatRequest>,
) -> Result<HttpResponse, AppError> {
    let chat_id = path.into_inner();
    log::info!("PATCH /chats/{chat_id} user={}", user.user_id);
    state
        .storage
        .update_ai_chat_title(chat_id, user.user_id, &body.title)
        .await?;

    // Return updated chat
    let chat = state.storage.get_ai_chat(chat_id, user.user_id).await?;
    Ok(HttpResponse::Ok().json(chat))
}

// ---------------------------------------------------------------------------
// Delete chat
// ---------------------------------------------------------------------------

pub async fn delete_chat(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let chat_id = path.into_inner();
    log::info!("DELETE /chats/{chat_id} user={}", user.user_id);
    state.storage.delete_ai_chat(chat_id, user.user_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })))
}

// ---------------------------------------------------------------------------
// Send message
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

pub async fn send_message(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<SendMessageRequest>,
) -> Result<HttpResponse, AppError> {
    let chat_id = path.into_inner();
    log::info!("POST /chats/{chat_id}/messages user={}", user.user_id);
    let chat = state.storage.get_ai_chat(chat_id, user.user_id).await?;

    // Quota check: require at least $0.25 to make a request
    let quota = state.storage.get_user_quota(user.user_id).await?;
    if quota < 0.25 {
        return Err(AppError(DomainError::QuotaExhausted(
            "Your AI token quota is too low. Request more tokens from your profile.".into(),
        )));
    }

    let llm_client = state
        .llm_client
        .as_ref()
        .ok_or_else(|| AppError(DomainError::Internal("LLM not configured".into())))?;

    // We need an Arc<dyn LlmClient> — clone the Arc
    let llm_arc: std::sync::Arc<dyn LlmClient> = llm_client.clone();

    let mut assistant_msg =
        conversation_manager::send_message(&state.storage, &llm_arc, &chat, &body.content).await?;

    // Update the price
    assistant_msg.cost = state.cost_to_user_quota(assistant_msg.cost);

    // Deduct quota
    if assistant_msg.cost > 0.0 {
        if let Err(err) = state
            .storage
            .deduct_quota(user.user_id, assistant_msg.cost)
            .await
        {
            log::error!("Failed to deduct quota: {}", err);
        }
    }

    // Return with markup-adjusted cost
    Ok(HttpResponse::Ok().json(assistant_msg))
}

// ---------------------------------------------------------------------------
// List available models
// ---------------------------------------------------------------------------

pub async fn list_models(
    state: web::Data<AppState>,
    _user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /chats/models");
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

// ---------------------------------------------------------------------------
// Create from insight
// ---------------------------------------------------------------------------

const SYSTEM_PROMPT: &str =
    "You are an experienced running coach analyzing a runner's training plan. \
    Provide specific, actionable advice based on the data provided. \
    Use metric units (km, min/km pace). Be concise but remain precise.";

#[derive(Deserialize)]
pub struct CreateFromInsightRequest {
    pub model: Option<String>,
    pub conversation_length: Option<u32>,
}

pub async fn create_from_insight(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: Option<web::Json<CreateFromInsightRequest>>,
) -> Result<HttpResponse, AppError> {
    let insight_id = path.into_inner();
    log::info!(
        "POST /chats/from-insight/{insight_id} user={}",
        user.user_id
    );

    let insight = state
        .storage
        .get_training_insight_by_id(insight_id, user.user_id)
        .await?;

    let title = format!("Chat: {}", insight.display_label);
    let model = body
        .as_ref()
        .and_then(|b| b.model.as_ref())
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_MODEL);
    let conversation_length = body.as_ref().and_then(|b| b.conversation_length);

    let chat = conversation_manager::create_from_insight(
        &state.storage,
        user.user_id,
        insight.training_id,
        insight.id,
        SYSTEM_PROMPT,
        &insight.full_prompt,
        &insight.response,
        model,
        &title,
        conversation_length,
    )
    .await?;

    Ok(HttpResponse::Created().json(chat))
}

// ---------------------------------------------------------------------------
// Add context to chat
// ---------------------------------------------------------------------------

pub async fn add_context(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<ContextRequest>,
) -> Result<HttpResponse, AppError> {
    let chat_id = path.into_inner();
    log::info!("POST /chats/{chat_id}/context user={}", user.user_id);

    // Verify chat belongs to user
    let _chat = state.storage.get_ai_chat(chat_id, user.user_id).await?;

    // Build context
    let result =
        context_builder::build_context(&state.storage, user.user_id, body.into_inner()).await?;

    // Store as a user message with context_label
    let msg = domain::AiChatMessage {
        id: Uuid::new_v4(),
        chat_id,
        role: "user".to_string(),
        content: result.content,
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        cost: 0.0,
        context_label: Some(result.label),
        created_at: chrono::Utc::now(),
    };
    state.storage.store_ai_chat_message(&msg).await?;
    state.storage.touch_ai_chat(chat_id).await?;

    Ok(HttpResponse::Ok().json(msg))
}
