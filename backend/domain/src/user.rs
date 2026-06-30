use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_INITIAL_USER_QUOTA_USD: f64 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub mas_current: Option<f64>, // Current MAS estimate in km/h
    pub quota_balance_usd: f64,
}

impl User {
    pub fn new(username: String, display_name: String, email: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            display_name,
            email,
            created_at: Utc::now(),
            mas_current: None,
            quota_balance_usd: DEFAULT_INITIAL_USER_QUOTA_USD,
        }
    }
}
