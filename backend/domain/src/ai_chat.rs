use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChat {
    pub id: Uuid,
    pub user_id: Uuid,
    pub training_id: Option<Uuid>,
    pub source_insight_id: Option<Uuid>,
    #[serde(default)]
    pub source_insight_cost: f64,
    pub title: String,
    pub model: String,
    #[serde(default)]
    pub conversation_length: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatMessage {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub role: String,
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: f64,
    pub context_label: Option<String>,
    pub created_at: DateTime<Utc>,
}
