use std::str::FromStr;

use actix_web::web;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use coach_memory::CoachToolExecutor;
use domain::{
    coach_sport_type_matches_filter, Activity, ActivityTag, DomainError, Prescription,
    RunningCoachSettings, SessionStatus, SessionType, TrainingSession,
};
use llm::{ToolCall, ToolDefinition};
use serde::Deserialize;
use serde_json::{json, Value};
use storage::{SqliteStorage, Storage};
use uuid::Uuid;

use crate::helpers::activity_description::{build_activity_description, ActivityDescriptionMode};
use crate::helpers::formatting::format_pace_from_activity;
use crate::state::AppState;

// Keep these LLM-visible tool payloads aligned with ../../../../doc/ai-coach-data-inputs.md.
pub struct AppCoachToolExecutor {
    state: web::Data<AppState>,
    created_session_ids: tokio::sync::Mutex<Vec<Uuid>>,
}

impl AppCoachToolExecutor {
    pub fn new(state: web::Data<AppState>) -> Self {
        Self {
            state,
            created_session_ids: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub async fn take_created_session_ids(&self) -> Vec<Uuid> {
        let mut guard = self.created_session_ids.lock().await;
        std::mem::take(&mut *guard)
    }
}

#[async_trait]
impl CoachToolExecutor for AppCoachToolExecutor {
    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "search_sessions".to_string(),
                description: "Search sessions by text query and return candidate activities with canonical activity_id UUID.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "What to search for (date, activity name, tag, race, interval, etc.)"
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 20,
                            "description": "Max number of candidate sessions to return. Default 5."
                        }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }),
            },
            ToolDefinition {
                name: "get_last_sessions".to_string(),
                description: "Get the most recent sessions, optionally filtered by sport type or activity tag, with additional metadata such as distance, pace and elevation gain.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "description": "How many recent sessions to return. Default 1."
                        },
                        "sport_type": {
                            "type": "string",
                            "description": "Optional sport type filter, for example 'Run'."
                        },
                        "tag": {
                            "type": "string",
                            "enum": ["normal", "intervals", "race", "long_run"],
                            "description": "Optional activity tag filter."
                        }
                    },
                    "additionalProperties": false
                }),
            },
            ToolDefinition {
                name: "get_sessions_in_time_range".to_string(),
                description: "Get sessions whose start date falls within an inclusive date range, optionally filtered by sport type or activity tag.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "start_date": {
                            "type": "string",
                            "description": "Inclusive UTC start date in YYYY-MM-DD format."
                        },
                        "end_date": {
                            "type": "string",
                            "description": "Inclusive UTC end date in YYYY-MM-DD format."
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 20,
                            "description": "Max number of sessions to return. Default 10."
                        },
                        "sport_type": {
                            "type": "string",
                            "description": "Optional sport type filter, for example 'Run'."
                        },
                        "tag": {
                            "type": "string",
                            "enum": ["normal", "intervals", "race", "long_run"],
                            "description": "Optional activity tag filter."
                        }
                    },
                    "required": ["start_date", "end_date"],
                    "additionalProperties": false
                }),
            },
            ToolDefinition {
                name: "get_session_detail".to_string(),
                description: "Get high-fidelity markdown description for one activity, addressed by canonical activity_id UUID.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "activity_id": {
                            "type": "string",
                            "description": "Canonical internal activity UUID"
                        },
                        "detail_mode": {
                            "type": "string",
                            "enum": ["auto", "intervals", "race", "long_run", "normal"],
                            "description": "Optional rendering mode. Default auto."
                        }
                    },
                    "required": ["activity_id"],
                    "additionalProperties": false
                }),
            },
            ToolDefinition {
                name: "list_planned_sessions".to_string(),
                description: "List the user's planned/suggested quality sessions (training_sessions table), newest first. Call this before proposing a new session to avoid double-proposing something already on the agenda.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["suggested", "planned", "done", "skipped", "rejected"],
                            "description": "Optional status filter. Omit to return all statuses."
                        }
                    },
                    "additionalProperties": false
                }),
            },
            ToolDefinition {
                name: "propose_sessions".to_string(),
                description: "Persist one or more structured quality-session suggestions. Call ONLY for quality sessions (intervals, tempo, threshold, hill, fartlek, progression, race_pace, time_trial, strides, other_quality). Do NOT call for easy runs, long runs, recovery, or rest days — answer those in prose. Default to ONE session unless the user asks for options. The `prescription` field is REQUIRED on every item and must follow the nested schema below — a tempo session is `sets:[{repeat:1, work:{duration_s|distance_m, target}, recovery: null}]`, an interval session is `sets:[{repeat:N, work:{...,target}, recovery:{...}}]`.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "sessions": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {
                                        "type": "string",
                                        "description": "Short human-readable title, e.g. '6 × 800m at 5K pace'."
                                    },
                                    "session_type": {
                                        "type": "string",
                                        "enum": ["intervals", "tempo", "threshold", "hill", "fartlek", "progression", "race_pace", "time_trial", "strides", "other_quality"]
                                    },
                                    "expiry": {
                                        "type": "string",
                                        "description": "Optional RFC3339 timestamp by which this suggestion is no longer relevant."
                                    },
                                    "estimated_duration_s": {
                                        "type": "integer",
                                        "description": "Optional rough total duration in seconds."
                                    },
                                    "estimated_distance_m": {
                                        "type": "number",
                                        "description": "Optional rough total distance in meters."
                                    },
                                    "intensity_summary": {
                                        "type": "string",
                                        "description": "Optional one-line intent, e.g. 'VO2max stimulus without too much fatigue'."
                                    },
                                    "prescription": {
                                        "type": "object",
                                        "description": "REQUIRED. Structured workout. Provide at least `sets[0].work.target`. For a continuous tempo, use `sets:[{repeat:1, work:{duration_s:1200, target:{type:'pace', min_s_per_km:255, max_s_per_km:265}}}]`.",
                                        "properties": {
                                            "warmup": {
                                                "type": "object",
                                                "description": "Optional. Open block — duration or distance, no formal target (intent is 'easy').",
                                                "properties": {
                                                    "duration_s": { "type": "integer", "minimum": 1 },
                                                    "distance_m": { "type": "integer", "minimum": 1 },
                                                    "notes": { "type": "string" }
                                                }
                                            },
                                            "sets": {
                                                "type": "array",
                                                "description": "Ordered list of repeated work+recovery blocks. Single continuous efforts (e.g. tempo) use one set with repeat=1 and no recovery.",
                                                "items": {
                                                    "type": "object",
                                                    "properties": {
                                                        "repeat": { "type": "integer", "minimum": 1 },
                                                        "work": {
                                                            "type": "object",
                                                            "description": "One work effort. Provide exactly one of duration_s or distance_m, plus a target.",
                                                            "properties": {
                                                                "duration_s": { "type": "integer", "minimum": 1 },
                                                                "distance_m": { "type": "integer", "minimum": 1 },
                                                                "target": {
                                                                    "type": "object",
                                                                    "description": "Tagged union. Set `type` to one of: pace, speed, heart_rate, percent_mas, rpe, effort. Then provide the matching fields: pace → {min_s_per_km, max_s_per_km}; speed → {min_mps, max_mps}; heart_rate → {min_bpm, max_bpm}; percent_mas → {min, max}; rpe → {min, max} (1-10); effort → {label} (free-text intent like 'easy jog').",
                                                                    "properties": {
                                                                        "type": {
                                                                            "type": "string",
                                                                            "enum": ["pace", "speed", "heart_rate", "percent_mas", "rpe", "effort"]
                                                                        },
                                                                        "min_s_per_km": { "type": "integer" },
                                                                        "max_s_per_km": { "type": "integer" },
                                                                        "min_mps": { "type": "number" },
                                                                        "max_mps": { "type": "number" },
                                                                        "min_bpm": { "type": "integer" },
                                                                        "max_bpm": { "type": "integer" },
                                                                        "min": { "type": "number" },
                                                                        "max": { "type": "number" },
                                                                        "label": { "type": "string" }
                                                                    },
                                                                    "required": ["type"]
                                                                }
                                                            },
                                                            "required": ["target"]
                                                        },
                                                        "recovery": {
                                                            "type": "object",
                                                            "description": "Optional recovery between reps. Omit for continuous efforts.",
                                                            "properties": {
                                                                "duration_s": { "type": "integer", "minimum": 1 },
                                                                "distance_m": { "type": "integer", "minimum": 1 },
                                                                "target": {
                                                                    "type": "object",
                                                                    "description": "Tagged union. Set `type` to one of: pace, speed, heart_rate, percent_mas, rpe, effort. Then provide the matching fields: pace → {min_s_per_km, max_s_per_km}; speed → {min_mps, max_mps}; heart_rate → {min_bpm, max_bpm}; percent_mas → {min, max}; rpe → {min, max} (1-10); effort → {label} (free-text intent like 'easy jog').",
                                                                    "properties": {
                                                                        "type": {
                                                                            "type": "string",
                                                                            "enum": ["pace", "speed", "heart_rate", "percent_mas", "rpe", "effort"]
                                                                        },
                                                                        "min_s_per_km": { "type": "integer" },
                                                                        "max_s_per_km": { "type": "integer" },
                                                                        "min_mps": { "type": "number" },
                                                                        "max_mps": { "type": "number" },
                                                                        "min_bpm": { "type": "integer" },
                                                                        "max_bpm": { "type": "integer" },
                                                                        "min": { "type": "number" },
                                                                        "max": { "type": "number" },
                                                                        "label": { "type": "string" }
                                                                    },
                                                                    "required": ["type"]
                                                                }
                                                            }
                                                        }
                                                    },
                                                    "required": ["repeat", "work"]
                                                }
                                            },
                                            "cooldown": {
                                                "type": "object",
                                                "description": "Optional. Same shape as warmup.",
                                                "properties": {
                                                    "duration_s": { "type": "integer", "minimum": 1 },
                                                    "distance_m": { "type": "integer", "minimum": 1 },
                                                    "notes": { "type": "string" }
                                                }
                                            },
                                            "notes": { "type": "string" }
                                        }
                                    }
                                },
                                "required": ["title", "session_type", "prescription"]
                            }
                        }
                    },
                    "required": ["sessions"],
                    "additionalProperties": false
                }),
            },
            ToolDefinition {
                name: "update_planned_session_status".to_string(),
                description: "Transition a planned/suggested session to a new status. Use only when the user has explicitly skipped, rejected, marked-done, or accepted a prior suggestion in conversation. Acceptance is normally a UI action; only call this tool when chat makes intent unambiguous.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Canonical training_session UUID."
                        },
                        "status": {
                            "type": "string",
                            "enum": ["suggested", "planned", "done", "skipped", "rejected"]
                        }
                    },
                    "required": ["id", "status"],
                    "additionalProperties": false
                }),
            },
        ]
    }

    async fn execute_tool_call(
        &self,
        user_id: Uuid,
        call: &ToolCall,
    ) -> Result<String, DomainError> {
        let argument_keys = call
            .arguments
            .as_object()
            .map(|object| object.keys().cloned().collect::<Vec<_>>().join(","))
            .unwrap_or_else(|| "<non-object>".to_string());
        log::info!(
            "Coach tool call user_id={} tool={} tool_call_id={} argument_keys=[{}]",
            user_id,
            call.name,
            call.id,
            argument_keys
        );
        if let Some(parse_error) = &call.arguments_parse_error {
            log::warn!(
                "Coach tool call user_id={} tool={} tool_call_id={} had argument parse error: {}",
                user_id,
                call.name,
                call.id,
                parse_error
            );
        }

        match call.name.as_str() {
            "search_sessions" => self.search_sessions(user_id, &call.arguments).await,
            "get_last_sessions" => self.get_last_sessions(user_id, &call.arguments).await,
            "get_sessions_in_time_range" => {
                self.get_sessions_in_time_range(user_id, &call.arguments)
                    .await
            }
            "get_session_detail" => self.get_session_detail(user_id, &call.arguments).await,
            "list_planned_sessions" => {
                do_list_planned_sessions(self.state.storage.as_ref(), user_id, &call.arguments)
                    .await
            }
            "propose_sessions" => {
                do_propose_sessions(
                    self.state.storage.as_ref(),
                    user_id,
                    &call.arguments,
                    &self.created_session_ids,
                )
                .await
            }
            "update_planned_session_status" => {
                do_update_planned_session_status(
                    self.state.storage.as_ref(),
                    user_id,
                    &call.arguments,
                )
                .await
            }
            other => {
                log::warn!(
                    "Coach tool call user_id={} tool={} tool_call_id={} is unsupported",
                    user_id,
                    other,
                    call.id
                );
                Ok(json!({
                    "error": format!("Unknown tool '{other}'"),
                    "supported_tools": [
                        "search_sessions",
                        "get_last_sessions",
                        "get_sessions_in_time_range",
                        "get_session_detail",
                        "list_planned_sessions",
                        "propose_sessions",
                        "update_planned_session_status"
                    ],
                })
                .to_string())
            }
        }
    }

    fn summarize_tool_result(&self, call: &ToolCall, tool_output: &str) -> Option<String> {
        summarize_tool_result(call, tool_output)
    }
}

impl AppCoachToolExecutor {
    async fn search_sessions(&self, user_id: Uuid, args: &Value) -> Result<String, DomainError> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .map(|s| s.trim())
            .unwrap_or_default();
        if query.is_empty() {
            log::warn!(
                "Coach tool search_sessions user_id={} rejected empty query",
                user_id
            );
            return Ok(json!({
                "query": "",
                "matches": [],
                "ambiguous": false,
                "message": "Missing query. Provide a date, session name, or tag.",
            })
            .to_string());
        }

        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .clamp(1, 20) as usize;
        log::info!(
            "Coach tool search_sessions user_id={} query='{}' limit={}",
            user_id,
            query,
            limit
        );

        if let Ok(activity_id) = Uuid::parse_str(query) {
            if let Ok(activity) = self.state.storage.get_activity(activity_id, user_id).await {
                log::info!(
                    "Coach tool search_sessions user_id={} exact activity_id match activity_id={}",
                    user_id,
                    activity_id
                );
                return Ok(json!({
                    "query": query,
                    "matches": [serialize_match(&activity, 10_000)],
                    "ambiguous": false,
                    "message": "Exact activity_id match found.",
                })
                .to_string());
            }
            log::info!(
                "Coach tool search_sessions user_id={} activity_id fast path missed activity_id={}",
                user_id,
                activity_id
            );
        }

        let activities = self.state.storage.get_activities(user_id, 500, 0).await?;
        let matches = build_search_matches(query, &activities, limit);
        log::info!(
            "Coach tool search_sessions user_id={} scanned={} matches={} ambiguous={}",
            user_id,
            activities.len(),
            matches.len(),
            matches.len() > 1
        );

        let message = if matches.is_empty() {
            "No session matched this query."
        } else if matches.len() == 1 {
            "One matching session found."
        } else {
            "Multiple sessions matched. Ask the user to choose an activity_id before requesting details."
        };

        Ok(json!({
            "query": query,
            "matches": matches,
            "ambiguous": matches.len() > 1,
            "message": message,
        })
        .to_string())
    }

    async fn get_last_sessions(&self, user_id: Uuid, args: &Value) -> Result<String, DomainError> {
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .clamp(1, 20) as usize;
        let sport_type = extract_optional_sport_type(args);
        let tag = extract_optional_tag(args)?;
        let settings = self
            .state
            .storage
            .get_or_create_running_coach_settings(user_id)
            .await?;
        log::info!(
            "Coach tool get_last_sessions user_id={} limit={} sport_type={} tag={} trail_as_run={}",
            user_id,
            limit,
            sport_type.as_deref().unwrap_or("any"),
            tag.map(|v| v.to_string())
                .unwrap_or_else(|| "any".to_string()),
            settings.consider_trail_runs_as_runs
        );

        let activities = self.state.storage.get_activities(user_id, 500, 0).await?;
        let matches =
            build_last_session_matches(&activities, &settings, limit, sport_type.as_deref(), tag);
        log::info!(
            "Coach tool get_last_sessions user_id={} scanned={} matches={}",
            user_id,
            activities.len(),
            matches.len()
        );

        let message = if matches.is_empty() {
            "No recent sessions matched these filters."
        } else if matches.len() == 1 {
            "One recent session found."
        } else {
            "Multiple recent sessions found."
        };

        Ok(json!({
            "limit": limit,
            "sport_type": sport_type,
            "tag": tag.map(|value| value.to_string()),
            "matches": matches,
            "ambiguous": matches.len() > 1,
            "message": message,
        })
        .to_string())
    }

    async fn get_sessions_in_time_range(
        &self,
        user_id: Uuid,
        args: &Value,
    ) -> Result<String, DomainError> {
        let start_date_raw = args
            .get("start_date")
            .and_then(Value::as_str)
            .ok_or_else(|| DomainError::BadRequest("Missing start_date".to_string()))?;
        let end_date_raw = args
            .get("end_date")
            .and_then(Value::as_str)
            .ok_or_else(|| DomainError::BadRequest("Missing end_date".to_string()))?;
        let start_date = parse_yyyy_mm_dd(start_date_raw, "start_date")?;
        let end_date = parse_yyyy_mm_dd(end_date_raw, "end_date")?;
        if end_date < start_date {
            return Err(DomainError::BadRequest(
                "end_date must be greater than or equal to start_date".to_string(),
            ));
        }

        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(10)
            .clamp(1, 20) as usize;
        let sport_type = extract_optional_sport_type(args);
        let tag = extract_optional_tag(args)?;
        let settings = self
            .state
            .storage
            .get_or_create_running_coach_settings(user_id)
            .await?;
        log::info!(
            "Coach tool get_sessions_in_time_range user_id={} start_date={} end_date={} limit={} sport_type={} tag={} trail_as_run={}",
            user_id,
            start_date,
            end_date,
            limit,
            sport_type.as_deref().unwrap_or("any"),
            tag.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string()),
            settings.consider_trail_runs_as_runs
        );

        let activities = self.state.storage.get_activities(user_id, 500, 0).await?;
        let matches = build_time_range_matches(
            &activities,
            &settings,
            start_date,
            end_date,
            limit,
            sport_type.as_deref(),
            tag,
        );
        log::info!(
            "Coach tool get_sessions_in_time_range user_id={} scanned={} matches={}",
            user_id,
            activities.len(),
            matches.len()
        );

        let message = if matches.is_empty() {
            "No sessions matched this time range."
        } else if matches.len() == 1 {
            "One session matched this time range."
        } else {
            "Multiple sessions matched this time range. Ask the user to choose an activity_id before requesting details."
        };

        Ok(json!({
            "start_date": start_date.to_string(),
            "end_date": end_date.to_string(),
            "sport_type": sport_type,
            "tag": tag.map(|value| value.to_string()),
            "matches": matches,
            "ambiguous": matches.len() > 1,
            "message": message,
        })
        .to_string())
    }

    async fn get_session_detail(&self, user_id: Uuid, args: &Value) -> Result<String, DomainError> {
        let activity_id_raw = args
            .get("activity_id")
            .and_then(Value::as_str)
            .ok_or_else(|| DomainError::BadRequest("Missing activity_id".to_string()))?;
        let activity_id = Uuid::parse_str(activity_id_raw)
            .map_err(|e| DomainError::BadRequest(format!("Invalid activity_id: {e}")))?;

        let mode_raw = args
            .get("detail_mode")
            .and_then(Value::as_str)
            .unwrap_or("auto");
        let mode = ActivityDescriptionMode::from_str(mode_raw)?;
        log::info!(
            "Coach tool get_session_detail user_id={} activity_id={} detail_mode={}",
            user_id,
            activity_id,
            mode.as_str()
        );

        let description =
            build_activity_description(self.state.get_ref(), user_id, activity_id, mode).await?;
        log::info!(
            "Coach tool get_session_detail user_id={} activity_id={} description_len={}",
            user_id,
            activity_id,
            description.len()
        );

        Ok(json!({
            "activity_id": activity_id.to_string(),
            "detail_mode": mode.as_str(),
            "description_markdown": description,
        })
        .to_string())
    }
}

fn score_activity(activity: &Activity, query_lc: &str, tokens: &[&str]) -> i64 {
    let mut score = 0_i64;
    let name_lc = activity.name.to_ascii_lowercase();
    let tag = activity.tag.to_string();
    let tag_lc = tag.to_ascii_lowercase();
    let sport_lc = activity.sport_type.to_ascii_lowercase();
    let date = activity.start_date.format("%Y-%m-%d").to_string();
    let date_lc = date.to_ascii_lowercase();

    if name_lc.contains(query_lc) {
        score += 50;
    }
    if tag_lc.contains(query_lc) {
        score += 35;
    }
    if date_lc == query_lc {
        score += 45;
    } else if date_lc.contains(query_lc) {
        score += 25;
    }
    if sport_lc.contains(query_lc) {
        score += 10;
    }

    for token in tokens {
        if name_lc.contains(token) {
            score += 12;
        }
        if tag_lc.contains(token) {
            score += 8;
        }
        if date_lc.contains(token) {
            score += 6;
        }
        if sport_lc.contains(token) {
            score += 4;
        }
    }

    score
}

fn extract_optional_sport_type(args: &Value) -> Option<String> {
    args.get("sport_type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_optional_tag(args: &Value) -> Result<Option<ActivityTag>, DomainError> {
    args.get("tag")
        .and_then(Value::as_str)
        .map(ActivityTag::from_str)
        .transpose()
        .map_err(DomainError::BadRequest)
}

fn parse_yyyy_mm_dd(raw: &str, field_name: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|e| DomainError::BadRequest(format!("Invalid {field_name}: {e}")))
}

fn activity_matches_filters(
    settings: &RunningCoachSettings,
    activity: &Activity,
    sport_type: Option<&str>,
    tag: Option<ActivityTag>,
) -> bool {
    let sport_matches = coach_sport_type_matches_filter(settings, &activity.sport_type, sport_type);
    let tag_matches = tag.map(|expected| activity.tag == expected).unwrap_or(true);
    sport_matches && tag_matches
}

fn build_last_session_matches(
    activities: &[Activity],
    settings: &RunningCoachSettings,
    limit: usize,
    sport_type: Option<&str>,
    tag: Option<ActivityTag>,
) -> Vec<Value> {
    let mut filtered: Vec<&Activity> = activities
        .iter()
        .filter(|activity| activity_matches_filters(settings, activity, sport_type, tag))
        .collect();
    filtered.sort_by_key(|activity| std::cmp::Reverse(activity.start_date));

    filtered
        .into_iter()
        .take(limit)
        .map(|activity| serialize_match(activity, 0))
        .collect()
}

fn build_time_range_matches(
    activities: &[Activity],
    settings: &RunningCoachSettings,
    start_date: NaiveDate,
    end_date: NaiveDate,
    limit: usize,
    sport_type: Option<&str>,
    tag: Option<ActivityTag>,
) -> Vec<Value> {
    let start_at = Utc.from_utc_datetime(
        &start_date
            .and_hms_opt(0, 0, 0)
            .expect("midnight should be valid"),
    );
    let end_exclusive = Utc.from_utc_datetime(
        &(end_date + Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .expect("midnight should be valid"),
    );

    let mut filtered: Vec<&Activity> = activities
        .iter()
        .filter(|activity| activity.start_date >= start_at && activity.start_date < end_exclusive)
        .filter(|activity| activity_matches_filters(settings, activity, sport_type, tag))
        .collect();
    filtered.sort_by_key(|activity| std::cmp::Reverse(activity.start_date));

    filtered
        .into_iter()
        .take(limit)
        .map(|activity| serialize_match(activity, 0))
        .collect()
}

fn build_search_matches(query: &str, activities: &[Activity], limit: usize) -> Vec<Value> {
    let query_lc = query.to_ascii_lowercase();
    let tokens: Vec<&str> = query_lc
        .split_whitespace()
        .filter(|t| !t.trim().is_empty())
        .collect();

    let mut scored = Vec::new();
    for activity in activities {
        let score = score_activity(activity, &query_lc, &tokens);
        if score > 0 {
            scored.push((score, activity));
        }
    }

    scored.sort_by(|(score_a, activity_a), (score_b, activity_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| activity_b.start_date.cmp(&activity_a.start_date))
    });

    scored
        .into_iter()
        .take(limit)
        .map(|(score, activity)| serialize_match(activity, score))
        .collect()
}

fn serialize_match(activity: &Activity, score: i64) -> Value {
    json!({
        "activity_id": activity.id.to_string(),
        "strava_id": activity.strava_id,
        "name": activity.name,
        "start_date": activity.start_date.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        "tag": activity.tag.to_string(),
        "sport_type": activity.sport_type,
        "distance_km": ((activity.distance / 1000.0) * 100.0).round() / 100.0,
        "moving_time_s": activity.moving_time,
        "elevation_gain_m": activity.total_elevation_gain.round() as i64,
        "pace": format_pace_from_activity(activity.distance, activity.moving_time),
        "score": score,
    })
}

fn summarize_tool_result(call: &ToolCall, tool_output: &str) -> Option<String> {
    let payload: Value = serde_json::from_str(tool_output).ok()?;
    match call.name.as_str() {
        "search_sessions" => summarize_match_tool("search_sessions", &call.arguments, &payload),
        "get_last_sessions" => summarize_match_tool("get_last_sessions", &call.arguments, &payload),
        "get_sessions_in_time_range" => {
            summarize_match_tool("get_sessions_in_time_range", &call.arguments, &payload)
        }
        "get_session_detail" => summarize_session_detail_tool(&call.arguments, &payload),
        "list_planned_sessions" => summarize_list_planned_sessions(&call.arguments, &payload),
        "propose_sessions" => summarize_propose_sessions(&payload),
        "update_planned_session_status" => {
            summarize_update_planned_session_status(&call.arguments, &payload)
        }
        _ => None,
    }
}

fn summarize_list_planned_sessions(arguments: &Value, payload: &Value) -> Option<String> {
    let sessions = payload.get("sessions")?.as_array()?;
    let status_arg = arguments
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("any");
    let titles: Vec<String> = sessions
        .iter()
        .take(3)
        .filter_map(|s| {
            let title = s.get("title").and_then(Value::as_str)?;
            let status = s.get("status").and_then(Value::as_str).unwrap_or("?");
            Some(format!("'{title}' [{status}]"))
        })
        .collect();
    let summary = if titles.is_empty() {
        format!(
            "list_planned_sessions(status={status_arg}) -> 0 sessions"
        )
    } else {
        format!(
            "list_planned_sessions(status={status_arg}) -> {} session(s): {}",
            sessions.len(),
            titles.join(", ")
        )
    };
    Some(summary.chars().take(280).collect())
}

fn summarize_propose_sessions(payload: &Value) -> Option<String> {
    let created = payload.get("created")?.as_array()?;
    let count = created.len();
    let titles: Vec<String> = created
        .iter()
        .take(3)
        .filter_map(|c| c.get("title").and_then(Value::as_str).map(|s| format!("'{s}'")))
        .collect();
    let summary = if titles.is_empty() {
        format!("propose_sessions -> created {count} session(s)")
    } else {
        format!(
            "propose_sessions -> created {count} session(s): {}",
            titles.join(", ")
        )
    };
    Some(summary.chars().take(280).collect())
}

fn summarize_update_planned_session_status(arguments: &Value, payload: &Value) -> Option<String> {
    let id = arguments.get("id").and_then(Value::as_str).unwrap_or("?");
    if let Some(err) = payload.get("error").and_then(Value::as_str) {
        return Some(format!(
            "update_planned_session_status(id={id}) -> error: {err}"
        ));
    }
    let new_status = payload
        .get("new_status")
        .and_then(Value::as_str)
        .or_else(|| arguments.get("status").and_then(Value::as_str))
        .unwrap_or("?");
    Some(format!(
        "update_planned_session_status(id={id}) -> {new_status}"
    ))
}

#[derive(Debug, Deserialize)]
struct ProposedSessionPayload {
    title: String,
    session_type: String,
    #[serde(default)]
    expiry: Option<chrono::DateTime<Utc>>,
    #[serde(default)]
    estimated_duration_s: Option<i64>,
    #[serde(default)]
    estimated_distance_m: Option<f64>,
    #[serde(default)]
    intensity_summary: Option<String>,
    prescription: Prescription,
}

async fn do_list_planned_sessions(
    storage: &SqliteStorage,
    user_id: Uuid,
    args: &Value,
) -> Result<String, DomainError> {
    let status = args
        .get("status")
        .and_then(Value::as_str)
        .map(SessionStatus::from_str)
        .transpose()
        .map_err(DomainError::BadRequest)?;
    log::info!(
        "Coach tool list_planned_sessions user_id={} status={}",
        user_id,
        status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "any".to_string())
    );

    let sessions = storage.list_training_sessions(user_id, status).await?;
    let serialized: Vec<Value> = sessions
        .iter()
        .take(20)
        .map(|s| {
            json!({
                "id": s.id.to_string(),
                "title": s.title,
                "session_type": s.session_type.to_string(),
                "status": s.status.to_string(),
                "expiry": s.expiry.map(|d| d.to_rfc3339()),
                "prescription_json": s.prescription_json,
            })
        })
        .collect();
    log::info!(
        "Coach tool list_planned_sessions user_id={} returned={}",
        user_id,
        serialized.len()
    );

    Ok(json!({ "sessions": serialized }).to_string())
}

async fn do_propose_sessions(
    storage: &SqliteStorage,
    user_id: Uuid,
    args: &Value,
    created_session_ids: &tokio::sync::Mutex<Vec<Uuid>>,
) -> Result<String, DomainError> {
    let raw_sessions = args
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    log::info!(
        "Coach tool propose_sessions user_id={} payload_items={}",
        user_id,
        raw_sessions.len()
    );

    let mut created = Vec::new();
    let mut errors = Vec::new();
    for (index, raw) in raw_sessions.into_iter().enumerate() {
        let supplied_title = raw
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string);
        let item = match serde_json::from_value::<ProposedSessionPayload>(raw.clone()) {
            Ok(value) => value,
            Err(e) => {
                let reason = format!("invalid payload shape: {e}");
                log::warn!(
                    "Coach tool propose_sessions user_id={} index={} skipped malformed item: {}",
                    user_id,
                    index,
                    e
                );
                errors.push(json!({
                    "index": index,
                    "title": supplied_title,
                    "reason": reason,
                }));
                continue;
            }
        };
        let session_type = match SessionType::from_str(&item.session_type) {
            Ok(value) => value,
            Err(e) => {
                let reason = format!("invalid session_type '{}': {}", item.session_type, e);
                log::warn!(
                    "Coach tool propose_sessions user_id={} index={} {}",
                    user_id,
                    index,
                    reason
                );
                errors.push(json!({
                    "index": index,
                    "title": supplied_title,
                    "reason": reason,
                }));
                continue;
            }
        };
        let prescription_json = match serde_json::to_string(&item.prescription) {
            Ok(value) => value,
            Err(e) => {
                let reason = format!("failed to serialize prescription: {e}");
                log::warn!(
                    "Coach tool propose_sessions user_id={} index={} {}",
                    user_id,
                    index,
                    reason
                );
                errors.push(json!({
                    "index": index,
                    "title": supplied_title,
                    "reason": reason,
                }));
                continue;
            }
        };
        let now = Utc::now();
        let title = item.title.clone();
        let session = TrainingSession {
            id: Uuid::new_v4(),
            user_id,
            training_id: None,
            status: SessionStatus::Suggested,
            title: item.title,
            session_type,
            expiry: item.expiry,
            estimated_duration_s: item.estimated_duration_s,
            estimated_distance_m: item.estimated_distance_m,
            intensity_summary: item.intensity_summary,
            prescription_json,
            coach_message_id: None,
            created_at: now,
            updated_at: now,
        };
        if let Err(e) = storage.create_training_session(&session).await {
            let reason = format!("storage insert failed: {e}");
            log::warn!(
                "Coach tool propose_sessions user_id={} index={} title='{}' {}",
                user_id,
                index,
                title,
                reason
            );
            errors.push(json!({
                "index": index,
                "title": Some(title.clone()),
                "reason": reason,
            }));
            continue;
        }
        {
            let mut guard = created_session_ids.lock().await;
            guard.push(session.id);
        }
        created.push(json!({
            "id": session.id.to_string(),
            "title": title,
        }));
    }
    log::info!(
        "Coach tool propose_sessions user_id={} created={} errors={}",
        user_id,
        created.len(),
        errors.len()
    );

    let mut response = serde_json::Map::new();
    response.insert("created".to_string(), Value::Array(created));
    if !errors.is_empty() {
        response.insert("errors".to_string(), Value::Array(errors));
        response.insert(
            "retry_hint".to_string(),
            Value::String(
                "Some sessions failed validation. The required shape is: \
                 { sessions: [{ title, session_type, prescription: { sets: [{ repeat, work: { duration_s or distance_m, target: { type, ...range } }, recovery? }], warmup?, cooldown?, notes? } }] }. \
                 target.type ∈ pace|speed|heart_rate|percent_mas|rpe|effort with matching min/max fields. Retry the failed items with valid payloads."
                    .to_string(),
            ),
        );
    }
    Ok(Value::Object(response).to_string())
}

async fn do_update_planned_session_status(
    storage: &SqliteStorage,
    user_id: Uuid,
    args: &Value,
) -> Result<String, DomainError> {
    let id_raw = args
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| DomainError::BadRequest("Missing id".to_string()))?;
    let id = Uuid::parse_str(id_raw)
        .map_err(|e| DomainError::BadRequest(format!("Invalid id: {e}")))?;
    let status_raw = args
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| DomainError::BadRequest("Missing status".to_string()))?;
    let status = SessionStatus::from_str(status_raw).map_err(DomainError::BadRequest)?;
    log::info!(
        "Coach tool update_planned_session_status user_id={} id={} status={}",
        user_id,
        id,
        status
    );

    match storage
        .update_training_session_status(id, user_id, status)
        .await
    {
        Ok(()) => Ok(json!({
            "id": id.to_string(),
            "new_status": status.to_string(),
        })
        .to_string()),
        Err(DomainError::NotFound(msg)) => Ok(json!({ "error": msg }).to_string()),
        Err(e) => Err(e),
    }
}

fn summarize_match_tool(tool_name: &str, arguments: &Value, payload: &Value) -> Option<String> {
    let matches = payload.get("matches")?.as_array()?;
    let match_count = matches.len();
    let top_matches = matches
        .iter()
        .take(2)
        .filter_map(compact_match_label)
        .collect::<Vec<_>>();
    let summary = if top_matches.is_empty() {
        format!(
            "{} -> {} match(es)",
            summarize_tool_args(tool_name, arguments),
            match_count
        )
    } else {
        format!(
            "{} -> {} match(es): {}",
            summarize_tool_args(tool_name, arguments),
            match_count,
            top_matches.join("; ")
        )
    };
    Some(summary.chars().take(280).collect())
}

fn summarize_session_detail_tool(arguments: &Value, payload: &Value) -> Option<String> {
    let activity_id = payload
        .get("activity_id")
        .and_then(Value::as_str)
        .or_else(|| arguments.get("activity_id").and_then(Value::as_str))?;
    let detail_mode = payload
        .get("detail_mode")
        .and_then(Value::as_str)
        .or_else(|| arguments.get("detail_mode").and_then(Value::as_str))
        .unwrap_or("auto");
    Some(format!(
        "get_session_detail(activity_id={}, detail_mode={}) -> loaded detailed session context",
        activity_id, detail_mode
    ))
}

fn summarize_tool_args(tool_name: &str, arguments: &Value) -> String {
    match tool_name {
        "search_sessions" => format!(
            "search_sessions(query='{}')",
            arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        "get_last_sessions" => {
            let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(1);
            let sport = arguments.get("sport_type").and_then(Value::as_str);
            let tag = arguments.get("tag").and_then(Value::as_str);
            format!(
                "get_last_sessions(limit={}, sport_type={}, tag={})",
                limit,
                sport.unwrap_or("any"),
                tag.unwrap_or("any")
            )
        }
        "get_sessions_in_time_range" => format!(
            "get_sessions_in_time_range(start_date={}, end_date={}, sport_type={}, tag={})",
            arguments
                .get("start_date")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            arguments
                .get("end_date")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            arguments
                .get("sport_type")
                .and_then(Value::as_str)
                .unwrap_or("any"),
            arguments
                .get("tag")
                .and_then(Value::as_str)
                .unwrap_or("any")
        ),
        _ => tool_name.to_string(),
    }
}

fn compact_match_label(value: &Value) -> Option<String> {
    let activity_id = value.get("activity_id")?.as_str()?;
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("session");
    let date = value
        .get("start_date")
        .and_then(Value::as_str)
        .and_then(|raw| raw.split_whitespace().next())
        .unwrap_or("unknown-date");
    Some(format!("{} on {} ({})", name, date, activity_id))
}

#[cfg(test)]
mod tests {
    use super::{
        build_last_session_matches, build_search_matches, build_time_range_matches,
        summarize_tool_result,
    };
    use chrono::{Duration, NaiveDate, Utc};
    use domain::{Activity, ActivityTag, RunningCoachSettings};
    use llm::ToolCall;
    use uuid::Uuid;

    fn sample_activity(name: &str, date_offset_days: i64, tag: ActivityTag) -> Activity {
        sample_activity_with_sport_type(name, date_offset_days, tag, "Run")
    }

    fn sample_activity_with_sport_type(
        name: &str,
        date_offset_days: i64,
        tag: ActivityTag,
        sport_type: &str,
    ) -> Activity {
        sample_activity_with_start_date(
            name,
            Utc::now() - Duration::days(date_offset_days),
            tag,
            sport_type,
        )
    }

    fn sample_activity_with_start_date(
        name: &str,
        start_date: chrono::DateTime<Utc>,
        tag: ActivityTag,
        sport_type: &str,
    ) -> Activity {
        Activity {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            strava_id: 1,
            name: name.to_string(),
            sport_type: sport_type.to_string(),
            start_date,
            elapsed_time: 3600,
            moving_time: 3500,
            distance: 10_000.0,
            total_elevation_gain: 100.0,
            average_speed: 2.85,
            max_speed: 4.5,
            average_heartrate: Some(150.0),
            max_heartrate: Some(175.0),
            average_cadence: Some(172.0),
            average_watts: None,
            calories: None,
            tag,
            summary_polyline: None,
            workout_type: None,
            streams_fetched_at: None,
            created_at: Utc::now(),
        }
    }

    fn settings(consider_trail_runs_as_runs: bool) -> RunningCoachSettings {
        RunningCoachSettings {
            consider_trail_runs_as_runs,
            ..RunningCoachSettings::default()
        }
    }

    #[test]
    fn search_returns_canonical_uuid_fields() {
        let activities = vec![sample_activity(
            "Tuesday Intervals",
            1,
            ActivityTag::Intervals,
        )];
        let matches = build_search_matches("intervals", &activities, 5);

        assert_eq!(matches.len(), 1);
        let id = matches[0]
            .get("activity_id")
            .and_then(|v| v.as_str())
            .expect("activity_id");
        assert!(Uuid::parse_str(id).is_ok());
        assert_eq!(
            matches[0].get("elevation_gain_m").and_then(|v| v.as_i64()),
            Some(100)
        );
    }

    #[test]
    fn search_keeps_multiple_candidates_for_disambiguation() {
        let activities = vec![
            sample_activity("Lunch Run", 1, ActivityTag::Normal),
            sample_activity("Lunch Run", 2, ActivityTag::Normal),
            sample_activity("Easy Run", 3, ActivityTag::Normal),
        ];

        let matches = build_search_matches("lunch run", &activities, 5);
        assert!(matches.len() >= 2);
    }

    #[test]
    fn get_last_sessions_returns_most_recent_filtered_matches() {
        let activities = vec![
            sample_activity("Older Race", 5, ActivityTag::Race),
            sample_activity("Latest Normal", 1, ActivityTag::Normal),
            sample_activity("Latest Race", 0, ActivityTag::Race),
        ];

        let matches = build_last_session_matches(
            &activities,
            &settings(false),
            2,
            Some("Run"),
            Some(ActivityTag::Race),
        );

        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches[0].get("name").and_then(|v| v.as_str()),
            Some("Latest Race")
        );
        assert_eq!(
            matches[1].get("name").and_then(|v| v.as_str()),
            Some("Older Race")
        );
    }

    #[test]
    fn time_range_matches_are_inclusive_by_day() {
        let activities = vec![
            sample_activity_with_start_date(
                "Mar 01",
                NaiveDate::from_ymd_opt(2026, 3, 1)
                    .expect("valid date")
                    .and_hms_opt(8, 0, 0)
                    .expect("valid time")
                    .and_utc(),
                ActivityTag::Normal,
                "Run",
            ),
            sample_activity_with_start_date(
                "Mar 03",
                NaiveDate::from_ymd_opt(2026, 3, 3)
                    .expect("valid date")
                    .and_hms_opt(8, 0, 0)
                    .expect("valid time")
                    .and_utc(),
                ActivityTag::Intervals,
                "Run",
            ),
            sample_activity_with_start_date(
                "Mar 05",
                NaiveDate::from_ymd_opt(2026, 3, 5)
                    .expect("valid date")
                    .and_hms_opt(8, 0, 0)
                    .expect("valid time")
                    .and_utc(),
                ActivityTag::Normal,
                "Run",
            ),
        ];

        let start = NaiveDate::from_ymd_opt(2026, 3, 3).expect("valid date");
        let end = NaiveDate::from_ymd_opt(2026, 3, 5).expect("valid date");
        let matches = build_time_range_matches(
            &activities,
            &settings(false),
            start,
            end,
            10,
            Some("Run"),
            None,
        );

        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches[0].get("name").and_then(|v| v.as_str()),
            Some("Mar 05")
        );
        assert_eq!(
            matches[1].get("name").and_then(|v| v.as_str()),
            Some("Mar 03")
        );
    }

    #[test]
    fn tool_result_summary_is_compact_and_includes_activity_id() {
        let call = ToolCall {
            id: "call_1".to_string(),
            name: "get_last_sessions".to_string(),
            arguments: serde_json::json!({ "limit": 1 }),
            arguments_raw: "{\"limit\":1}".to_string(),
            arguments_parse_error: None,
        };
        let output = serde_json::json!({
            "matches": [{
                "activity_id": "93d3cd28-a734-4b25-9e5d-113ee5f640a7",
                "name": "Lunch Run",
                "start_date": "2026-03-03 10:49:49 UTC"
            }]
        })
        .to_string();

        let summary = summarize_tool_result(&call, &output).expect("summary");

        assert!(summary.contains("get_last_sessions"));
        assert!(summary.contains("Lunch Run"));
        assert!(summary.contains("93d3cd28-a734-4b25-9e5d-113ee5f640a7"));
    }

    #[test]
    fn run_filter_optionally_includes_trail_runs() {
        let activities = vec![
            sample_activity_with_sport_type("Road Run", 2, ActivityTag::Normal, "Run"),
            sample_activity_with_sport_type("Trail Run", 1, ActivityTag::Normal, "TrailRun"),
        ];

        let without_trails =
            build_last_session_matches(&activities, &settings(false), 10, Some("Run"), None);
        let with_trails =
            build_last_session_matches(&activities, &settings(true), 10, Some("Run"), None);

        assert_eq!(without_trails.len(), 1);
        assert_eq!(
            without_trails[0].get("name").and_then(|v| v.as_str()),
            Some("Road Run")
        );
        assert_eq!(with_trails.len(), 2);
        assert_eq!(
            with_trails[0].get("name").and_then(|v| v.as_str()),
            Some("Trail Run")
        );
    }

    #[test]
    fn explicit_trail_run_filter_remains_specific() {
        let activities = vec![
            sample_activity_with_sport_type("Road Run", 2, ActivityTag::Normal, "Run"),
            sample_activity_with_sport_type("Trail Run", 1, ActivityTag::Normal, "TrailRun"),
        ];

        let matches =
            build_last_session_matches(&activities, &settings(true), 10, Some("TrailRun"), None);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].get("name").and_then(|v| v.as_str()),
            Some("Trail Run")
        );
    }

    // ---- Phase 3 write-tool tests (propose / list / update) ----

    use super::{
        do_list_planned_sessions, do_propose_sessions, do_update_planned_session_status,
    };
    use domain::{SessionStatus, User};
    use serde_json::json;
    use storage::{SqliteStorage, Storage};

    async fn fresh_storage_and_user() -> (SqliteStorage, User, String) {
        let path = format!("/tmp/coach_tools_test_{}.db", Uuid::new_v4().simple());
        let url = format!("sqlite:{path}?mode=rwc");
        let db = SqliteStorage::new(&url).await.expect("open storage");
        let user = User::new("cttest".into(), "Coach Tools Test".into(), None);
        db.create_user(&user).await.expect("create user");
        (db, user, path)
    }

    fn cleanup(path: &str) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{path}-wal"));
        let _ = std::fs::remove_file(format!("{path}-shm"));
    }

    fn valid_intervals_payload() -> serde_json::Value {
        json!({
            "sessions": [{
                "title": "6 × 800m",
                "session_type": "intervals",
                "intensity_summary": "VO2max stimulus",
                "prescription": {
                    "warmup": { "duration_s": 1200 },
                    "sets": [{
                        "repeat": 6,
                        "work": {
                            "duration_s": 180,
                            "target": { "type": "pace", "min_s_per_km": 230, "max_s_per_km": 240 }
                        },
                        "recovery": {
                            "duration_s": 120,
                            "target": { "type": "effort", "label": "easy jog" }
                        }
                    }],
                    "cooldown": { "duration_s": 600 }
                }
            }]
        })
    }

    #[tokio::test]
    async fn propose_sessions_happy_path_inserts_row_and_records_id() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        let out = do_propose_sessions(&db, user.id, &valid_intervals_payload(), &created_ids)
            .await
            .expect("propose");
        let payload: serde_json::Value = serde_json::from_str(&out).unwrap();
        let created = payload.get("created").and_then(|v| v.as_array()).unwrap();
        assert_eq!(created.len(), 1);
        // Happy path: no `errors` key (omitted when empty).
        assert!(payload.get("errors").is_none());
        let id_str = created[0]["id"].as_str().unwrap();
        let id = Uuid::parse_str(id_str).unwrap();

        let stored = db.get_training_session(id, user.id).await.expect("get");
        assert_eq!(stored.status, SessionStatus::Suggested);
        assert!(stored.coach_message_id.is_none());
        assert_eq!(stored.title, "6 × 800m");

        let drained = std::mem::take(&mut *created_ids.lock().await);
        assert_eq!(drained, vec![id]);

        // Stamping after the loop wires the message link.
        let msg_id = Uuid::new_v4();
        db.set_training_session_coach_message_id(id, user.id, msg_id)
            .await
            .expect("stamp");
        let stamped = db.get_training_session(id, user.id).await.expect("re-get");
        assert_eq!(stamped.coach_message_id, Some(msg_id));

        cleanup(&path);
    }

    #[tokio::test]
    async fn propose_sessions_reports_errors_for_malformed_items() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        // Mix of one valid + two invalid (bad target.type, bad session_type).
        let payload = json!({
            "sessions": [
                {
                    "title": "Bogus target",
                    "session_type": "intervals",
                    "prescription": {
                        "sets": [{
                            "repeat": 4,
                            "work": {
                                "duration_s": 60,
                                "target": { "type": "bogus", "label": "??" }
                            },
                            "recovery": { "duration_s": 60 }
                        }]
                    }
                },
                {
                    "title": "Bad session type",
                    "session_type": "not_a_type",
                    "prescription": { "sets": [] }
                },
                {
                    "title": "Valid tempo",
                    "session_type": "tempo",
                    "prescription": { "sets": [] }
                }
            ]
        });

        let out = do_propose_sessions(&db, user.id, &payload, &created_ids)
            .await
            .expect("propose");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        let created = parsed.get("created").and_then(|v| v.as_array()).unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0]["title"].as_str(), Some("Valid tempo"));

        let errors = parsed.get("errors").and_then(|v| v.as_array()).unwrap();
        assert_eq!(errors.len(), 2);
        let titles: Vec<&str> = errors
            .iter()
            .filter_map(|e| e.get("title").and_then(serde_json::Value::as_str))
            .collect();
        assert!(titles.contains(&"Bogus target"));
        assert!(titles.contains(&"Bad session type"));
        assert!(parsed.get("retry_hint").is_some());

        let all = db
            .list_training_sessions(user.id, None)
            .await
            .expect("list");
        assert_eq!(all.len(), 1);

        cleanup(&path);
    }

    #[tokio::test]
    async fn propose_sessions_missing_prescription_is_surfaced_as_error() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        // The exact failure mode seen in production: title + session_type but no prescription.
        let payload = json!({
            "sessions": [{
                "title": "Tempo without prescription",
                "session_type": "tempo"
            }]
        });

        let out = do_propose_sessions(&db, user.id, &payload, &created_ids)
            .await
            .expect("propose");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["created"].as_array().unwrap().len(), 0);
        let errors = parsed["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        let reason = errors[0]["reason"].as_str().unwrap();
        assert!(
            reason.contains("prescription"),
            "expected reason to mention 'prescription', got: {reason}"
        );

        cleanup(&path);
    }

    #[tokio::test]
    async fn propose_sessions_with_empty_payload_returns_empty_created() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        let out = do_propose_sessions(&db, user.id, &json!({}), &created_ids)
            .await
            .expect("propose");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            parsed.get("created").and_then(|v| v.as_array()).unwrap().len(),
            0
        );

        cleanup(&path);
    }

    #[tokio::test]
    async fn update_planned_session_status_flips_status() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        // Seed a row via propose_sessions.
        let _ = do_propose_sessions(&db, user.id, &valid_intervals_payload(), &created_ids)
            .await
            .expect("propose");
        let id = std::mem::take(&mut *created_ids.lock().await)
            .into_iter()
            .next()
            .unwrap();

        let out = do_update_planned_session_status(
            &db,
            user.id,
            &json!({ "id": id.to_string(), "status": "planned" }),
        )
        .await
        .expect("update");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["new_status"].as_str(), Some("planned"));

        let planned = db
            .list_training_sessions(user.id, Some(SessionStatus::Planned))
            .await
            .expect("list planned");
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].id, id);

        cleanup(&path);
    }

    #[tokio::test]
    async fn update_planned_session_status_missing_returns_error_in_payload() {
        let (db, user, path) = fresh_storage_and_user().await;

        let out = do_update_planned_session_status(
            &db,
            user.id,
            &json!({ "id": Uuid::new_v4().to_string(), "status": "planned" }),
        )
        .await
        .expect("update");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed["error"].as_str().is_some());

        cleanup(&path);
    }

    #[tokio::test]
    async fn list_planned_sessions_filters_by_status() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        let _ = do_propose_sessions(&db, user.id, &valid_intervals_payload(), &created_ids)
            .await
            .expect("propose");

        let out_all = do_list_planned_sessions(&db, user.id, &json!({}))
            .await
            .expect("list all");
        let parsed_all: serde_json::Value = serde_json::from_str(&out_all).unwrap();
        assert_eq!(
            parsed_all["sessions"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0),
            1
        );

        let out_planned =
            do_list_planned_sessions(&db, user.id, &json!({ "status": "planned" }))
                .await
                .expect("list planned");
        let parsed_planned: serde_json::Value = serde_json::from_str(&out_planned).unwrap();
        assert_eq!(
            parsed_planned["sessions"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0),
            0
        );

        cleanup(&path);
    }
}
