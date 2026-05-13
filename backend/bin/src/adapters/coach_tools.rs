use actix_web::web;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use coach_memory::CoachToolExecutor;
use domain::{
    coach_sport_type_matches_filter, Activity, ActivityTag, DomainError, RunningCoachSettings,
};
use llm::{ToolCall, ToolDefinition};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::helpers::formatting::format_pace_from_activity;
use crate::state::AppState;

mod get_last_sessions;
mod get_session_detail;
mod get_sessions_in_time_range;
mod list_planned_sessions;
mod propose_sessions;
mod search_sessions;
mod update_planned_session_status;

use get_last_sessions::GetLastSessions;
use get_session_detail::GetSessionDetail;
use get_sessions_in_time_range::GetSessionsInTimeRange;
use list_planned_sessions::ListPlannedSessions;
use propose_sessions::ProposeSessions;
use search_sessions::SearchSessions;
use update_planned_session_status::UpdatePlannedSessionStatus;

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
            tool_definition::<SearchSessions>(),
            tool_definition::<GetLastSessions>(),
            tool_definition::<GetSessionsInTimeRange>(),
            tool_definition::<GetSessionDetail>(),
            tool_definition::<ListPlannedSessions>(),
            tool_definition::<ProposeSessions>(),
            tool_definition::<UpdatePlannedSessionStatus>(),
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
            <SearchSessions as CoachToolArgs>::NAME => {
                search_sessions::execute(self, user_id, &call.arguments).await
            }
            <GetLastSessions as CoachToolArgs>::NAME => {
                get_last_sessions::execute(self, user_id, &call.arguments).await
            }
            <GetSessionsInTimeRange as CoachToolArgs>::NAME => {
                get_sessions_in_time_range::execute(self, user_id, &call.arguments).await
            }
            <GetSessionDetail as CoachToolArgs>::NAME => {
                get_session_detail::execute(self, user_id, &call.arguments).await
            }
            <ListPlannedSessions as CoachToolArgs>::NAME => {
                list_planned_sessions::execute(
                    self.state.storage.as_ref(),
                    user_id,
                    &call.arguments,
                )
                .await
            }
            <ProposeSessions as CoachToolArgs>::NAME => {
                propose_sessions::execute(
                    self.state.storage.as_ref(),
                    user_id,
                    &call.arguments,
                    &self.created_session_ids,
                )
                .await
            }
            <UpdatePlannedSessionStatus as CoachToolArgs>::NAME => {
                update_planned_session_status::execute(
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
                        <SearchSessions as CoachToolArgs>::NAME,
                        <GetLastSessions as CoachToolArgs>::NAME,
                        <GetSessionsInTimeRange as CoachToolArgs>::NAME,
                        <GetSessionDetail as CoachToolArgs>::NAME,
                        <ListPlannedSessions as CoachToolArgs>::NAME,
                        <ProposeSessions as CoachToolArgs>::NAME,
                        <UpdatePlannedSessionStatus as CoachToolArgs>::NAME
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

fn trim_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_argument<T>(args: &Value) -> Result<T, DomainError>
where
    T: CoachToolArgs + for<'de> Deserialize<'de>,
{
    serde_json::from_value::<T>(args.clone())
        .map_err(|e| DomainError::BadRequest(format!("Invalid {} args: {e}", T::NAME)))
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
        <SearchSessions as CoachToolArgs>::NAME => {
            summarize_match_tool(SearchSessions::NAME, &call.arguments, &payload)
        }
        <GetLastSessions as CoachToolArgs>::NAME => {
            summarize_match_tool(GetLastSessions::NAME, &call.arguments, &payload)
        }
        <GetSessionsInTimeRange as CoachToolArgs>::NAME => {
            summarize_match_tool(GetSessionsInTimeRange::NAME, &call.arguments, &payload)
        }
        <GetSessionDetail as CoachToolArgs>::NAME => {
            summarize_session_detail_tool(&call.arguments, &payload)
        }
        <ListPlannedSessions as CoachToolArgs>::NAME => {
            summarize_list_planned_sessions(&call.arguments, &payload)
        }
        <ProposeSessions as CoachToolArgs>::NAME => summarize_propose_sessions(&payload),
        <UpdatePlannedSessionStatus as CoachToolArgs>::NAME => {
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
        format!("list_planned_sessions(status={status_arg}) -> 0 sessions")
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
        .filter_map(|c| {
            c.get("title")
                .and_then(Value::as_str)
                .map(|s| format!("'{s}'"))
        })
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

pub(crate) trait CoachToolArgs: JsonSchema {
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
}

fn tool_definition<T: CoachToolArgs>() -> ToolDefinition {
    ToolDefinition {
        name: T::NAME.to_string(),
        description: T::DESCRIPTION.to_string(),
        parameters: tool_parameters::<T>(),
    }
}

fn tool_parameters<T: JsonSchema>() -> Value {
    let mut schema =
        serde_json::to_value(schema_for!(T)).expect("generated tool schema should serialize");
    if let Value::Object(root) = &mut schema {
        root.remove("$schema");
        root.remove("title");
    }
    schema
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
        <SearchSessions as CoachToolArgs>::NAME => format!(
            "search_sessions(query='{}')",
            arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        <GetLastSessions as CoachToolArgs>::NAME => {
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
        <GetSessionsInTimeRange as CoachToolArgs>::NAME => format!(
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
        list_planned_sessions::execute as do_list_planned_sessions,
        propose_sessions::{execute as do_propose_sessions, ProposeSessions},
        tool_parameters,
        update_planned_session_status::execute as do_update_planned_session_status,
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

    fn schema_required_contains(schema: &serde_json::Value, field: &str) -> bool {
        schema
            .get("required")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|required| required.iter().any(|value| value.as_str() == Some(field)))
    }

    fn schema_definitions(
        schema: &serde_json::Value,
    ) -> &serde_json::Map<String, serde_json::Value> {
        schema
            .get("$defs")
            .or_else(|| schema.get("definitions"))
            .and_then(serde_json::Value::as_object)
            .expect("schema definitions")
    }

    fn resolve_schema_ref<'a>(
        root: &'a serde_json::Value,
        schema: &'a serde_json::Value,
    ) -> &'a serde_json::Value {
        if let Some(all_of) = schema.get("allOf").and_then(serde_json::Value::as_array) {
            if all_of.len() == 1 {
                return resolve_schema_ref(root, &all_of[0]);
            }
        }
        let Some(reference) = schema.get("$ref").and_then(serde_json::Value::as_str) else {
            return schema;
        };
        let name = reference
            .strip_prefix("#/$defs/")
            .or_else(|| reference.strip_prefix("#/definitions/"))
            .expect("local schema ref");
        schema_definitions(root)
            .get(name)
            .expect("referenced schema")
    }

    #[test]
    fn propose_sessions_schema_requires_sessions_and_denies_extra_fields() {
        let schema = tool_parameters::<ProposeSessions>();
        assert_eq!(schema.get("$schema"), None);
        assert_eq!(schema.get("title"), None);
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema_required_contains(&schema, "sessions"));

        let sessions = &schema["properties"]["sessions"];
        assert_eq!(sessions["minItems"], 1);

        let item = resolve_schema_ref(&schema, &sessions["items"]);
        assert_eq!(item["additionalProperties"], false);
        assert!(schema_required_contains(item, "title"));
        assert!(schema_required_contains(item, "session_type"));
        assert!(schema_required_contains(item, "prescription"));

        let prescription = resolve_schema_ref(&schema, &item["properties"]["prescription"]);
        assert_eq!(prescription["additionalProperties"], false);
        assert!(schema_required_contains(prescription, "sets"));
        assert_eq!(prescription["properties"]["sets"]["minItems"], 1);
    }

    #[test]
    fn propose_sessions_schema_exposes_snake_case_session_type_enum() {
        let schema = tool_parameters::<ProposeSessions>();
        let sessions = &schema["properties"]["sessions"];
        let item = resolve_schema_ref(&schema, &sessions["items"]);
        let session_type = resolve_schema_ref(&schema, &item["properties"]["session_type"]);
        let enum_values = session_type["enum"].as_array().expect("session type enum");

        for expected in [
            "intervals",
            "tempo",
            "threshold",
            "hill",
            "fartlek",
            "progression",
            "race_pace",
            "time_trial",
            "strides",
            "other_quality",
        ] {
            assert!(
                enum_values
                    .iter()
                    .any(|value| value.as_str() == Some(expected)),
                "missing enum value {expected}"
            );
        }
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
                    "prescription": {
                        "sets": [{
                            "repeat": 1,
                            "work": {
                                "duration_s": 1200,
                                "target": { "type": "pace", "min_s_per_km": 260, "max_s_per_km": 270 }
                            }
                        }]
                    }
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
    async fn propose_sessions_empty_prescription_is_surfaced_as_error() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        let payload = json!({
            "sessions": [{
                "title": "Empty prescription",
                "session_type": "tempo",
                "prescription": {}
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
            reason.contains("sets"),
            "expected reason to mention 'sets', got: {reason}"
        );

        cleanup(&path);
    }

    #[tokio::test]
    async fn propose_sessions_empty_sets_is_surfaced_as_error() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        let payload = json!({
            "sessions": [{
                "title": "No work",
                "session_type": "tempo",
                "prescription": { "sets": [] }
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
            reason.contains("at least one set"),
            "expected reason to mention non-empty sets, got: {reason}"
        );

        cleanup(&path);
    }

    #[tokio::test]
    async fn propose_sessions_with_empty_payload_returns_validation_error() {
        let (db, user, path) = fresh_storage_and_user().await;
        let created_ids = tokio::sync::Mutex::new(Vec::new());

        let out = do_propose_sessions(&db, user.id, &json!({}), &created_ids)
            .await
            .expect("propose");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            parsed
                .get("created")
                .and_then(|v| v.as_array())
                .unwrap()
                .len(),
            0
        );
        let errors = parsed["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0]["reason"]
            .as_str()
            .unwrap()
            .contains("Missing sessions"));

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

        let out_planned = do_list_planned_sessions(&db, user.id, &json!({ "status": "planned" }))
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

    // End-to-end proof that OpenRouter accepts the schemars-generated schema
    // payload for the full set of coach tools. Kept ignored because it requires
    // network access and a real API key at `backend/openrouter_key`.
    #[tokio::test]
    #[ignore = "live integration test: requires network and backend/openrouter_key"]
    async fn openrouter_accepts_full_coach_tool_schema_payload() {
        use super::{
            tool_definition, CoachToolArgs, GetLastSessions, GetSessionDetail,
            GetSessionsInTimeRange, ListPlannedSessions, ProposeSessions, SearchSessions,
            UpdatePlannedSessionStatus,
        };
        use llm::open_router::OpenRouterClient;
        use llm::{ChatMessage, LlmClient, ToolChoice};
        use std::path::PathBuf;

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let key_path = manifest_dir
            .parent()
            .expect("bin crate must be inside backend workspace")
            .join("openrouter_key");
        let api_key = std::fs::read_to_string(&key_path)
            .unwrap_or_else(|e| panic!("Failed to read OpenRouter key at {:?}: {e}", key_path))
            .trim()
            .to_string();
        assert!(
            !api_key.is_empty(),
            "OpenRouter key file exists but is empty: {:?}",
            key_path
        );

        let tools = vec![
            tool_definition::<SearchSessions>(),
            tool_definition::<GetLastSessions>(),
            tool_definition::<GetSessionsInTimeRange>(),
            tool_definition::<GetSessionDetail>(),
            tool_definition::<ListPlannedSessions>(),
            tool_definition::<ProposeSessions>(),
            tool_definition::<UpdatePlannedSessionStatus>(),
        ];

        let forced_name = <SearchSessions as CoachToolArgs>::NAME;
        let client = OpenRouterClient::new(api_key);

        let result = client
            .chat_completion_with_tools(
                "openai/gpt-5-mini",
                vec![
                    ChatMessage::system(
                        "You are running an integration smoke test for tool schema acceptance. \
                         Call only the tool you are forced to call, with minimal valid arguments.",
                    ),
                    ChatMessage::user(
                        "Search the user's sessions for 'tempo' and return at most 3 candidates.",
                    ),
                ],
                tools,
                Some(ToolChoice::Function {
                    name: forced_name.to_string(),
                }),
                Some(false),
                None,
            )
            .await
            .expect(
                "OpenRouter should accept the schemars-generated coach tool schema payload \
                 and honour the forced tool_choice",
            );

        assert!(
            !result.tool_calls.is_empty(),
            "Expected at least one tool call in live response, got none. \
             finish_reason={:?}, content={:?}",
            result.finish_reason,
            result.content,
        );
        assert_eq!(
            result.tool_calls[0].name, forced_name,
            "Expected the forced tool to be called"
        );
        assert!(
            result.tool_calls[0].arguments_parse_error.is_none(),
            "Expected tool arguments to be valid JSON, got parse error: {:?}",
            result.tool_calls[0].arguments_parse_error
        );
        assert!(
            result.tool_calls[0]
                .arguments
                .get("query")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|q| !q.trim().is_empty()),
            "Expected non-empty `query` argument from forced search_sessions call, got: {:?}",
            result.tool_calls[0].arguments
        );
        assert_eq!(
            result.finish_reason.as_deref(),
            Some("tool_calls"),
            "Expected finish_reason=tool_calls for forced function-call request"
        );
    }
}
