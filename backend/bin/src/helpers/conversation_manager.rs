use std::sync::Arc;

use chrono::Utc;
use domain::{AiChat, AiChatMessage, DomainError};
use llm::{ChatMessage, LlmClient};
use storage::SqliteStorage;
use storage::Storage;
use uuid::Uuid;

const DEFAULT_SYSTEM_PROMPT: &str = "You are an experienced running coach. \
    Provide specific, actionable advice based on the data provided. \
    Use metric units (km, min/km pace). Be concise but remain precise.";

pub async fn send_message(
    storage: &Arc<SqliteStorage>,
    llm_client: &Arc<dyn LlmClient>,
    chat: &AiChat,
    user_content: &str,
) -> Result<AiChatMessage, DomainError> {
    let existing_messages = storage.get_ai_chat_messages(chat.id).await?;

    // Store user message
    let user_msg = AiChatMessage {
        id: Uuid::new_v4(),
        chat_id: chat.id,
        role: "user".to_string(),
        content: user_content.to_string(),
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        cost: 0.0,
        context_label: None,
        created_at: Utc::now(),
    };
    storage.store_ai_chat_message(&user_msg).await?;

    // Build messages for LLM
    let mut llm_messages: Vec<ChatMessage> = Vec::new();

    // Separate system messages from other messages
    let (system_messages, other_messages): (Vec<_>, Vec<_>) = existing_messages
        .iter()
        .partition(|m| m.role == "system");

    // Add system message if it exists, otherwise use default
    if let Some(system_msg) = system_messages.first() {
        llm_messages.push(ChatMessage {
            role: system_msg.role.clone(),
            content: system_msg.content.clone(),
        });
    } else {
        llm_messages.push(ChatMessage::system(DEFAULT_SYSTEM_PROMPT));
    }

    // Always include context messages (they carry authoritative data)
    for msg in &other_messages {
        if msg.context_label.is_some() {
            llm_messages.push(ChatMessage::system(&msg.content));
        }
    }

    // Apply conversation length limit to non-context messages if set
    let mut conversation_messages: Vec<_> = other_messages
        .iter()
        .filter(|m| m.context_label.is_none())
        .collect();
    if let Some(max_messages) = chat.conversation_length {
        if max_messages > 0 {
            let start_idx = conversation_messages.len().saturating_sub(max_messages as usize);
            conversation_messages = conversation_messages[start_idx..].to_vec();
        }
    }

    for msg in &conversation_messages {
        llm_messages.push(ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }
    llm_messages.push(ChatMessage::user(user_content));

    // Call LLM
    let result = llm_client
        .chat_completion(&chat.model, llm_messages, None)
        .await
        .map_err(|e| DomainError::Internal(format!("LLM call failed: {e}")))?;

    // Store assistant message with usage
    let assistant_msg = AiChatMessage {
        id: Uuid::new_v4(),
        chat_id: chat.id,
        role: "assistant".to_string(),
        content: result.content,
        prompt_tokens: result.usage.prompt_tokens,
        completion_tokens: result.usage.completion_tokens,
        total_tokens: result.usage.total_tokens,
        cost: result.usage.cost,
        context_label: None,
        created_at: Utc::now(),
    };
    storage.store_ai_chat_message(&assistant_msg).await?;

    // Update chat timestamp
    storage.touch_ai_chat(chat.id).await?;

    Ok(assistant_msg)
}

pub async fn create_from_insight(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    training_id: Uuid,
    insight_id: Uuid,
    system_prompt: &str,
    user_prompt: &str,
    assistant_response: &str,
    model: &str,
    title: &str,
    conversation_length: Option<u32>,
) -> Result<AiChat, DomainError> {
    let now = Utc::now();
    let chat = AiChat {
        id: Uuid::new_v4(),
        user_id,
        training_id: Some(training_id),
        source_insight_id: Some(insight_id),
        title: title.to_string(),
        model: model.to_string(),
        conversation_length,
        created_at: now,
        updated_at: now,
    };
    storage.create_ai_chat(&chat).await?;

    // Store the 3 seed messages
    let system_msg = AiChatMessage {
        id: Uuid::new_v4(),
        chat_id: chat.id,
        role: "system".to_string(),
        content: system_prompt.to_string(),
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        cost: 0.0,
        context_label: None,
        created_at: now,
    };
    storage.store_ai_chat_message(&system_msg).await?;

    let user_msg = AiChatMessage {
        id: Uuid::new_v4(),
        chat_id: chat.id,
        role: "user".to_string(),
        content: user_prompt.to_string(),
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        cost: 0.0,
        context_label: None,
        created_at: now,
    };
    storage.store_ai_chat_message(&user_msg).await?;

    let assistant_msg = AiChatMessage {
        id: Uuid::new_v4(),
        chat_id: chat.id,
        role: "assistant".to_string(),
        content: assistant_response.to_string(),
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        cost: 0.0,
        context_label: None,
        created_at: now,
    };
    storage.store_ai_chat_message(&assistant_msg).await?;

    Ok(chat)
}
