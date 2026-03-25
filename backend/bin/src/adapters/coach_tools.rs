use std::str::FromStr;

use actix_web::web;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use coach_memory::CoachToolExecutor;
use domain::{Activity, ActivityTag, DomainError};
use llm::{ToolCall, ToolDefinition};
use serde_json::{json, Value};
use storage::Storage;
use uuid::Uuid;

use crate::helpers::activity_description::{build_activity_description, ActivityDescriptionMode};
use crate::helpers::formatting::format_pace_from_activity;
use crate::state::AppState;

pub struct AppCoachToolExecutor {
    state: web::Data<AppState>,
}

impl AppCoachToolExecutor {
    pub fn new(state: web::Data<AppState>) -> Self {
        Self { state }
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
                        "get_session_detail"
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
        log::info!(
            "Coach tool get_last_sessions user_id={} limit={} sport_type={} tag={}",
            user_id,
            limit,
            sport_type.as_deref().unwrap_or("any"),
            tag.map(|v| v.to_string())
                .unwrap_or_else(|| "any".to_string())
        );

        let activities = self.state.storage.get_activities(user_id, 500, 0).await?;
        let matches = build_last_session_matches(&activities, limit, sport_type.as_deref(), tag);
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
        log::info!(
            "Coach tool get_sessions_in_time_range user_id={} start_date={} end_date={} limit={} sport_type={} tag={}",
            user_id,
            start_date,
            end_date,
            limit,
            sport_type.as_deref().unwrap_or("any"),
            tag.map(|v| v.to_string()).unwrap_or_else(|| "any".to_string())
        );

        let activities = self.state.storage.get_activities(user_id, 500, 0).await?;
        let matches = build_time_range_matches(
            &activities,
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
    activity: &Activity,
    sport_type: Option<&str>,
    tag: Option<ActivityTag>,
) -> bool {
    let sport_matches = sport_type
        .map(|expected| activity.sport_type.eq_ignore_ascii_case(expected))
        .unwrap_or(true);
    let tag_matches = tag.map(|expected| activity.tag == expected).unwrap_or(true);
    sport_matches && tag_matches
}

fn build_last_session_matches(
    activities: &[Activity],
    limit: usize,
    sport_type: Option<&str>,
    tag: Option<ActivityTag>,
) -> Vec<Value> {
    let mut filtered: Vec<&Activity> = activities
        .iter()
        .filter(|activity| activity_matches_filters(activity, sport_type, tag))
        .collect();
    filtered.sort_by(|a, b| b.start_date.cmp(&a.start_date));

    filtered
        .into_iter()
        .take(limit)
        .map(|activity| serialize_match(activity, 0))
        .collect()
}

fn build_time_range_matches(
    activities: &[Activity],
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
        .filter(|activity| activity_matches_filters(activity, sport_type, tag))
        .collect();
    filtered.sort_by(|a, b| b.start_date.cmp(&a.start_date));

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
        _ => None,
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
    use domain::{Activity, ActivityTag};
    use llm::ToolCall;
    use uuid::Uuid;

    fn sample_activity(name: &str, date_offset_days: i64, tag: ActivityTag) -> Activity {
        Activity {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            strava_id: 1,
            name: name.to_string(),
            sport_type: "Run".to_string(),
            start_date: Utc::now() - Duration::days(date_offset_days),
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

        let matches =
            build_last_session_matches(&activities, 2, Some("Run"), Some(ActivityTag::Race));

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
            sample_activity("Mar 01", 23, ActivityTag::Normal),
            sample_activity("Mar 03", 21, ActivityTag::Intervals),
            sample_activity("Mar 05", 19, ActivityTag::Normal),
        ];

        let start = NaiveDate::from_ymd_opt(2026, 3, 3).expect("valid date");
        let end = NaiveDate::from_ymd_opt(2026, 3, 5).expect("valid date");
        let matches = build_time_range_matches(&activities, start, end, 10, Some("Run"), None);

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
}
