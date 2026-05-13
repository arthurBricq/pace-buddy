use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Suggested,
    Planned,
    Done,
    Skipped,
    Rejected,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Suggested => write!(f, "suggested"),
            SessionStatus::Planned => write!(f, "planned"),
            SessionStatus::Done => write!(f, "done"),
            SessionStatus::Skipped => write!(f, "skipped"),
            SessionStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl std::str::FromStr for SessionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "suggested" => Ok(SessionStatus::Suggested),
            "planned" => Ok(SessionStatus::Planned),
            "done" => Ok(SessionStatus::Done),
            "skipped" => Ok(SessionStatus::Skipped),
            "rejected" => Ok(SessionStatus::Rejected),
            other => Err(format!("Unknown session status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    Intervals,
    Tempo,
    Threshold,
    Hill,
    Fartlek,
    Progression,
    RacePace,
    TimeTrial,
    Strides,
    OtherQuality,
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::Intervals => write!(f, "intervals"),
            SessionType::Tempo => write!(f, "tempo"),
            SessionType::Threshold => write!(f, "threshold"),
            SessionType::Hill => write!(f, "hill"),
            SessionType::Fartlek => write!(f, "fartlek"),
            SessionType::Progression => write!(f, "progression"),
            SessionType::RacePace => write!(f, "race_pace"),
            SessionType::TimeTrial => write!(f, "time_trial"),
            SessionType::Strides => write!(f, "strides"),
            SessionType::OtherQuality => write!(f, "other_quality"),
        }
    }
}

impl std::str::FromStr for SessionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "intervals" => Ok(SessionType::Intervals),
            "tempo" => Ok(SessionType::Tempo),
            "threshold" => Ok(SessionType::Threshold),
            "hill" => Ok(SessionType::Hill),
            "fartlek" => Ok(SessionType::Fartlek),
            "progression" => Ok(SessionType::Progression),
            "race_pace" => Ok(SessionType::RacePace),
            "time_trial" => Ok(SessionType::TimeTrial),
            "strides" => Ok(SessionType::Strides),
            "other_quality" => Ok(SessionType::OtherQuality),
            other => Err(format!("Unknown session type: {other}")),
        }
    }
}

/// A planned quality session — the user-facing object that the coach proposes
/// (status `suggested`) and the user accepts/rejects (status flips to
/// `planned`/`rejected`). Stored top-level under the user; `training_id` is
/// optional context for Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub training_id: Option<Uuid>,
    pub status: SessionStatus,
    pub title: String,
    pub session_type: SessionType,
    /// Optional deadline by which the user intends to do this session. After
    /// this point the matching engine and UI may treat it as stale.
    pub expiry: Option<DateTime<Utc>>,
    pub estimated_duration_s: Option<i64>,
    pub estimated_distance_m: Option<f64>,
    pub intensity_summary: Option<String>,
    /// Structured JSON describing the prescription (warmup / sets / cooldown /
    /// targets). Stored as TEXT and parsed on demand by callers (matches the
    /// convention used for `interval_results.result_json`, etc.).
    pub prescription_json: String,
    pub coach_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    Candidate,
    AutoMatched,
    Confirmed,
    Rejected,
    Manual,
}

impl std::fmt::Display for MatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchStatus::Candidate => write!(f, "candidate"),
            MatchStatus::AutoMatched => write!(f, "auto_matched"),
            MatchStatus::Confirmed => write!(f, "confirmed"),
            MatchStatus::Rejected => write!(f, "rejected"),
            MatchStatus::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for MatchStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "candidate" => Ok(MatchStatus::Candidate),
            "auto_matched" => Ok(MatchStatus::AutoMatched),
            "confirmed" => Ok(MatchStatus::Confirmed),
            "rejected" => Ok(MatchStatus::Rejected),
            "manual" => Ok(MatchStatus::Manual),
            other => Err(format!("Unknown match status: {other}")),
        }
    }
}

/// Explicit link between a planned `TrainingSession` and a Strava `Activity`.
/// Schema-only in Phase 1 — Phase 5 introduces the matching engine and the
/// storage methods that read/write this table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSessionActivityMatch {
    pub id: Uuid,
    pub training_session_id: Uuid,
    pub activity_id: Uuid,
    pub user_id: Uuid,
    pub match_status: MatchStatus,
    pub confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_status_round_trip() {
        for s in [
            SessionStatus::Suggested,
            SessionStatus::Planned,
            SessionStatus::Done,
            SessionStatus::Skipped,
            SessionStatus::Rejected,
        ] {
            let displayed = s.to_string();
            let parsed: SessionStatus = displayed.parse().expect("parse");
            assert_eq!(parsed, s);
            let json = serde_json::to_string(&s).expect("serialize");
            let de: SessionStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(de, s);
        }
    }

    #[test]
    fn session_type_round_trip() {
        for t in [
            SessionType::Intervals,
            SessionType::Tempo,
            SessionType::Threshold,
            SessionType::Hill,
            SessionType::Fartlek,
            SessionType::Progression,
            SessionType::RacePace,
            SessionType::TimeTrial,
            SessionType::Strides,
            SessionType::OtherQuality,
        ] {
            let displayed = t.to_string();
            let parsed: SessionType = displayed.parse().expect("parse");
            assert_eq!(parsed, t);
        }
    }

    #[test]
    fn match_status_round_trip() {
        for m in [
            MatchStatus::Candidate,
            MatchStatus::AutoMatched,
            MatchStatus::Confirmed,
            MatchStatus::Rejected,
            MatchStatus::Manual,
        ] {
            let displayed = m.to_string();
            let parsed: MatchStatus = displayed.parse().expect("parse");
            assert_eq!(parsed, m);
        }
    }

    #[test]
    fn unknown_value_errors() {
        let e: Result<SessionStatus, _> = "nonsense".parse();
        assert!(e.is_err());
    }
}
