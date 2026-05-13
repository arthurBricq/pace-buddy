use coach_tool_macros::CoachTool;
use domain::{ActivityTag, DomainError};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use storage::Storage;
use uuid::Uuid;

use super::{
    build_last_session_matches, parse_argument, trim_optional_string, AppCoachToolExecutor,
};

#[derive(Debug, Deserialize, JsonSchema, CoachTool)]
#[serde(deny_unknown_fields)]
#[tool(
    name = "get_last_sessions",
    description = "Get the most recent sessions, optionally filtered by sport type or activity \
                   tag, with additional metadata such as distance, pace and elevation gain."
)]
pub(super) struct GetLastSessions {
    /// How many recent sessions to return. Default 1.
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
    let args = parse_argument::<GetLastSessions>(args)?;
    let limit = args.limit.unwrap_or(1).clamp(1, 20) as usize;
    let sport_type = trim_optional_string(args.sport_type);
    let tag = args.tag;
    let settings = executor
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

    let activities = executor
        .state
        .storage
        .get_activities(user_id, 500, 0)
        .await?;
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
