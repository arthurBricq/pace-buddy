use coach_tool_macros::CoachTool;
use domain::DomainError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::helpers::activity_description::{build_activity_description, ActivityDescriptionMode};

use super::{parse_argument, AppCoachToolExecutor};

#[derive(Debug, Deserialize, JsonSchema, CoachTool)]
#[serde(deny_unknown_fields)]
#[tool(
    name = "get_session_detail",
    description = "Get high-fidelity markdown description for one activity, addressed by \
                   canonical activity_id UUID."
)]
pub(super) struct GetSessionDetail {
    /// Canonical internal activity UUID.
    activity_id: String,
    /// Optional rendering mode. Default auto.
    detail_mode: Option<DetailModeArg>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum DetailModeArg {
    Auto,
    Intervals,
    Race,
    LongRun,
    Normal,
}

impl From<DetailModeArg> for ActivityDescriptionMode {
    fn from(value: DetailModeArg) -> Self {
        match value {
            DetailModeArg::Auto => Self::Auto,
            DetailModeArg::Intervals => Self::Intervals,
            DetailModeArg::Race => Self::Race,
            DetailModeArg::LongRun => Self::LongRun,
            DetailModeArg::Normal => Self::Normal,
        }
    }
}

pub(super) async fn execute(
    executor: &AppCoachToolExecutor,
    user_id: Uuid,
    args: &Value,
) -> Result<String, DomainError> {
    let args = parse_argument::<GetSessionDetail>(args)?;
    let activity_id = Uuid::parse_str(args.activity_id.trim())
        .map_err(|e| DomainError::BadRequest(format!("Invalid activity_id: {e}")))?;

    let mode = args
        .detail_mode
        .map(ActivityDescriptionMode::from)
        .unwrap_or(ActivityDescriptionMode::Auto);
    log::info!(
        "Coach tool get_session_detail user_id={} activity_id={} detail_mode={}",
        user_id,
        activity_id,
        mode.as_str()
    );

    let description =
        build_activity_description(executor.state.get_ref(), user_id, activity_id, mode).await?;
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
