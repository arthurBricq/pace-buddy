use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityTag {
    Normal,
    Intervals,
    Race,
}

impl ActivityTag {
    /// Infer tag from Strava workout_type field.
    /// workout_type: 0 = default, 1 = race, 2 = long run, 3 = workout/intervals
    pub fn from_strava_workout_type(workout_type: Option<i32>) -> Self {
        match workout_type {
            Some(1) => ActivityTag::Race,
            Some(3) => ActivityTag::Intervals,
            _ => ActivityTag::Normal,
        }
    }
}

impl std::fmt::Display for ActivityTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityTag::Normal => write!(f, "normal"),
            ActivityTag::Intervals => write!(f, "intervals"),
            ActivityTag::Race => write!(f, "race"),
        }
    }
}

impl std::str::FromStr for ActivityTag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(ActivityTag::Normal),
            "intervals" => Ok(ActivityTag::Intervals),
            "race" => Ok(ActivityTag::Race),
            other => Err(format!("Unknown activity tag: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub strava_id: i64,
    pub name: String,
    pub sport_type: String,
    pub start_date: DateTime<Utc>,
    pub elapsed_time: i32,
    pub moving_time: i32,
    pub distance: f64,
    pub total_elevation_gain: f64,
    pub average_speed: f64,
    pub max_speed: f64,
    pub average_heartrate: Option<f64>,
    pub max_heartrate: Option<f64>,
    pub average_cadence: Option<f64>,
    pub average_watts: Option<f64>,
    pub calories: Option<f64>,
    pub tag: ActivityTag,
    pub summary_polyline: Option<String>,
    pub workout_type: Option<i32>,
    pub streams_fetched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
