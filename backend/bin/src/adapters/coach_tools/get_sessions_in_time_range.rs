use coach_tool_macros::CoachTool;
use domain::{ActivityTag, DomainError};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use storage::Storage;
use uuid::Uuid;

use super::{
    build_time_range_matches, parse_argument, parse_yyyy_mm_dd, trim_optional_string,
    AppCoachToolExecutor,
};

#[derive(Debug, Deserialize, JsonSchema, CoachTool)]
#[serde(deny_unknown_fields)]
#[tool(
    name = "get_sessions_in_time_range",
    description = "Get sessions whose start date falls within an inclusive date range, \
                   optionally filtered by sport type or activity tag."
)]
pub(super) struct GetSessionsInTimeRange {
    /// Inclusive UTC start date in YYYY-MM-DD format.
    start_date: String,
    /// Inclusive UTC end date in YYYY-MM-DD format.
    end_date: String,
    /// Max number of sessions to return. Default 10.
    #[schemars(range(min = 1, max = 20))]
    limit: Option<u64>,
    /// Optional sport type filter, for example 'Run'.
    sport_type: Option<String>,
    /// Optional activity tag filter.
    tag: Option<ActivityTag>,
}

pub(super) async fn execute(
    executor: &AppCoachToolExecutor,
    user_id: Uuid,
    args: &Value,
) -> Result<String, DomainError> {
    let args = parse_argument::<GetSessionsInTimeRange>(args)?;
    let start_date = parse_yyyy_mm_dd(&args.start_date, "start_date")?;
    let end_date = parse_yyyy_mm_dd(&args.end_date, "end_date")?;
    if end_date < start_date {
        return Err(DomainError::BadRequest(
            "end_date must be greater than or equal to start_date".to_string(),
        ));
    }

    let limit = args.limit.unwrap_or(10).clamp(1, 20) as usize;
    let sport_type = trim_optional_string(args.sport_type);
    let tag = args.tag;
    let settings = executor
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

    let activities = executor
        .state
        .storage
        .get_activities(user_id, 500, 0)
        .await?;
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
