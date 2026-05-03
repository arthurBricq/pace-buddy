use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_COACH_MODEL: &str = "openai/gpt-5.3-chat";
pub const DEFAULT_COACH_PERSONALITY: &str =
    "Direct and practical running coach. Be concise, specific, and evidence-based.";
pub const RUN_SPORT_TYPE: &str = "Run";
pub const TRAIL_RUN_SPORT_TYPE: &str = "TrailRun";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningCoachSettings {
    pub user_id: Uuid,
    pub model: String,
    pub personality: String,
    pub consider_trail_runs_as_runs: bool,
    pub volume_weeks: i32,
    pub last_workouts_count: i32,
    pub last_long_runs_count: i32,
    pub last_races_count: i32,
    pub new_activities_count: i32,
    pub normalizer_every_n_messages: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for RunningCoachSettings {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            user_id: Uuid::nil(),
            model: DEFAULT_COACH_MODEL.to_string(),
            personality: DEFAULT_COACH_PERSONALITY.to_string(),
            consider_trail_runs_as_runs: false,
            volume_weeks: 8,
            last_workouts_count: 8,
            last_long_runs_count: 6,
            last_races_count: 4,
            new_activities_count: 8,
            normalizer_every_n_messages: 6,
            created_at: now,
            updated_at: now,
        }
    }
}

pub fn coach_considers_sport_type_as_run(
    settings: &RunningCoachSettings,
    sport_type: &str,
) -> bool {
    sport_type.eq_ignore_ascii_case(RUN_SPORT_TYPE)
        || (settings.consider_trail_runs_as_runs
            && sport_type.eq_ignore_ascii_case(TRAIL_RUN_SPORT_TYPE))
}

pub fn coach_sport_type_matches_filter(
    settings: &RunningCoachSettings,
    activity_sport_type: &str,
    requested_sport_type: Option<&str>,
) -> bool {
    match requested_sport_type {
        None => true,
        Some(expected) if expected.eq_ignore_ascii_case(RUN_SPORT_TYPE) => {
            coach_considers_sport_type_as_run(settings, activity_sport_type)
        }
        Some(expected) => activity_sport_type.eq_ignore_ascii_case(expected),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RunningCoachMemoryData {
    pub pinned_facts: Vec<String>,
    pub active_coaching_plan: String,
    pub episodic_memory: Vec<String>,
    pub rolling_summary: String,
    pub recent_tool_results: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningCoachMemory {
    pub user_id: Uuid,
    pub data: RunningCoachMemoryData,
    pub message_count_since_normalization: i32,
    pub updated_at: DateTime<Utc>,
}

impl Default for RunningCoachMemory {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            data: RunningCoachMemoryData::default(),
            message_count_since_normalization: 0,
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningCoachState {
    pub user_id: Uuid,
    pub last_interaction_at: Option<DateTime<Utc>>,
    pub last_seen_activity_start_date: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningCoachMessage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: f64,
    pub created_at: DateTime<Utc>,
}
