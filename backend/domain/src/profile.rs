use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityProfile {
    pub user_id: Uuid,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub email: Option<String>,
    pub gender: Option<String>,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthleteProfile {
    pub user_id: Uuid,
    pub goal_description: Option<String>,
    pub goal_date: Option<String>,
    pub goal_distance_km: Option<f64>,
    pub goal_target_time_seconds: Option<i32>,
    pub goal_sport_type: Option<String>,
    pub goal_elevation_gain_m: Option<f64>,
    pub additional_info: Option<String>,
    pub updated_at: DateTime<Utc>,
}
