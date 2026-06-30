use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningStats {
    pub total_distance_m: f64,
    pub total_time_s: i64,
    pub total_elevation_m: f64,
    pub avg_speed_mps: Option<f64>,
    pub activity_count: i64,
    pub interval_count: Option<i64>,
}
