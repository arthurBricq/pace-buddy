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
    name = "update_planned_session_status",
    description = "Transition a planned/suggested session to a new status. Use only when the \
                   user has explicitly skipped, rejected, marked-done, or accepted a prior \
                   suggestion in conversation. Acceptance is normally a UI action; only call \
                   this tool when chat makes intent unambiguous."
)]
pub(super) struct UpdatePlannedSessionStatus {
    /// Canonical training_session UUID.
    id: String,
    status: SessionStatus,
}

pub(super) async fn execute(
    storage: &SqliteStorage,
    user_id: Uuid,
    args: &Value,
) -> Result<String, DomainError> {
    let args = parse_argument::<UpdatePlannedSessionStatus>(args)?;
    let id = Uuid::parse_str(args.id.trim())
        .map_err(|e| DomainError::BadRequest(format!("Invalid id: {e}")))?;
    let status = args.status;
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
