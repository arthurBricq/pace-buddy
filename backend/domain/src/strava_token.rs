use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StravaToken {
    pub user_id: Uuid,
    pub strava_athlete_id: i64,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

impl StravaToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}
