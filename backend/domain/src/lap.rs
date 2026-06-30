use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLap {
    pub activity_id: Uuid,
    pub lap_index: i32,
    pub name: String,
    pub start_date: DateTime<Utc>,
    pub elapsed_time: i32,
    pub moving_time: i32,
    pub distance: f64,
    pub average_speed: f64,
    pub max_speed: f64,
    pub total_elevation_gain: f64,
    pub average_heartrate: Option<f64>,
    pub max_heartrate: Option<f64>,
}
