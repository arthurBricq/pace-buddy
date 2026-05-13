use coach_tool_macros::CoachTool;
use domain::{DomainError, SessionStatus};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use storage::{SqliteStorage, Storage};
use uuid::Uuid;

use super::parse_argument;

#[derive(Debug, Deserialize, JsonSchema, CoachTool)]
#[serde(deny_unknown_fields)]
#[tool(
    name = "list_planned_sessions",
    description = "List the user's planned/suggested quality sessions (training_sessions table), \
                   newest first. Call this before proposing a new session to avoid \
                   double-proposing something already on the agenda."
)]
pub(super) struct ListPlannedSessions {
    /// Optional status filter. Omit to return all statuses.
    status: Option<SessionStatus>,
}

pub(super) async fn execute(
    storage: &SqliteStorage,
    user_id: Uuid,
    args: &Value,
) -> Result<String, DomainError> {
    let args = parse_argument::<ListPlannedSessions>(args)?;
    let status = args.status;
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
