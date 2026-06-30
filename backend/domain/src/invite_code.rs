use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteCode {
    pub id: Uuid,
    pub code_hash: String,
    pub created_by_user_id: Option<Uuid>,
    pub created_for: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub used_at: Option<DateTime<Utc>>,
    pub used_by_strava_athlete_id: Option<i64>,
    pub revoked_at: Option<DateTime<Utc>>,
}
