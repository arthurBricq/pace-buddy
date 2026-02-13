use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingInsight {
    pub id: Uuid,
    pub training_id: Uuid,
    pub user_id: Uuid,
    pub prompt_type: String,
    pub display_label: String,
    pub full_prompt: String,
    pub response: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub cost: Option<f64>,
    pub created_at: DateTime<Utc>,
}
