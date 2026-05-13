use coach_tool_macros::CoachTool;
use domain::DomainError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use storage::Storage;
use uuid::Uuid;

use super::{build_search_matches, parse_argument, serialize_match, AppCoachToolExecutor};

#[derive(Debug, Deserialize, JsonSchema, CoachTool)]
#[serde(deny_unknown_fields)]
#[tool(
    name = "search_sessions",
    description = "Search sessions by text query and return candidate activities with canonical \
                   activity_id UUID."
)]
pub(super) struct SearchSessions {
    /// What to search for (date, activity name, tag, race, interval, etc.).
    query: String,
    /// Max number of candidate sessions to return. Default 5.
    #[schemars(range(min = 1, max = 20))]
    limit: Option<u64>,
}

pub(super) async fn execute(
    executor: &AppCoachToolExecutor,
    user_id: Uuid,
    args: &Value,
) -> Result<String, DomainError> {
    let args = parse_argument::<SearchSessions>(args)?;
    let query = args.query.trim();
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

    let limit = args.limit.unwrap_or(5).clamp(1, 20) as usize;
    log::info!(
        "Coach tool search_sessions user_id={} query='{}' limit={}",
        user_id,
        query,
        limit
    );

    if let Ok(activity_id) = Uuid::parse_str(query) {
        if let Ok(activity) = executor
            .state
            .storage
            .get_activity(activity_id, user_id)
            .await
        {
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

    let activities = executor
        .state
        .storage
        .get_activities(user_id, 500, 0)
        .await?;
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
