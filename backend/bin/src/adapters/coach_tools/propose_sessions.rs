use chrono::Utc;
use coach_tool_macros::CoachTool;
use domain::{DomainError, Prescription, SessionStatus, SessionType, TrainingSession};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use storage::{SqliteStorage, Storage};
use uuid::Uuid;

#[derive(Debug, Deserialize, JsonSchema, CoachTool)]
#[serde(deny_unknown_fields)]
#[tool(
    name = "propose_sessions",
    description = "Persist one or more structured quality-session suggestions. Call ONLY for \
                   quality sessions (intervals, tempo, threshold, hill, fartlek, progression, \
                   race_pace, time_trial, strides, other_quality). Do NOT call for easy runs, \
                   long runs, recovery, or rest days — answer those in prose. Default to ONE \
                   session unless the user asks for options. The `prescription` field is \
                   REQUIRED on every item and must follow the nested schema below — a tempo \
                   session is `sets:[{repeat:1, work:{duration_s|distance_m, target}, \
                   recovery: null}]`, an interval session is `sets:[{repeat:N, \
                   work:{...,target}, recovery:{...}}]`."
)]
#[allow(dead_code)]
pub(super) struct ProposeSessions {
    /// One or more structured quality-session suggestions.
    #[schemars(length(min = 1))]
    sessions: Vec<ProposedSessionPayload>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProposedSessionPayload {
    /// Short human-readable title, e.g. '6 x 800m at 5K pace'.
    #[schemars(length(min = 1))]
    title: String,
    session_type: SessionType,
    /// Optional RFC3339 timestamp by which this suggestion is no longer relevant.
    #[serde(default)]
    expiry: Option<chrono::DateTime<Utc>>,
    /// Optional rough total duration in seconds.
    #[serde(default)]
    #[schemars(range(min = 1))]
    estimated_duration_s: Option<i64>,
    /// Optional rough total distance in meters.
    #[serde(default)]
    #[schemars(range(min = 1.0))]
    estimated_distance_m: Option<f64>,
    /// Optional one-line intent, e.g. 'VO2max stimulus without too much fatigue'.
    #[serde(default)]
    intensity_summary: Option<String>,
    /// Required structured workout. Provide at least one set with work.target.
    prescription: Prescription,
}

pub(super) async fn execute(
    storage: &SqliteStorage,
    user_id: Uuid,
    args: &Value,
    created_session_ids: &tokio::sync::Mutex<Vec<Uuid>>,
) -> Result<String, DomainError> {
    let raw_sessions = match args.get("sessions") {
        Some(Value::Array(sessions)) if !sessions.is_empty() => sessions.clone(),
        Some(Value::Array(_)) => {
            return Ok(error_response("sessions must contain at least one item"));
        }
        _ => {
            return Ok(error_response(
                "Missing sessions. Provide at least one structured quality-session suggestion.",
            ));
        }
    };
    log::info!(
        "Coach tool propose_sessions user_id={} payload_items={}",
        user_id,
        raw_sessions.len()
    );

    let mut created = Vec::new();
    let mut errors = Vec::new();
    for (index, raw) in raw_sessions.into_iter().enumerate() {
        let supplied_title = raw.get("title").and_then(Value::as_str).map(str::to_string);
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
        if item.prescription.sets.is_empty() {
            let reason = "prescription.sets must contain at least one set".to_string();
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
            session_type: item.session_type,
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

fn error_response(reason: &str) -> String {
    json!({
        "created": [],
        "errors": [{
            "index": null,
            "title": null,
            "reason": reason,
        }],
        "retry_hint": "Provide { sessions: [{ title, session_type, prescription: { sets: [{ repeat, work: { duration_s or distance_m, target: { type, ...range } }, recovery? }], warmup?, cooldown?, notes? } }] } with at least one session and at least one prescription set.",
    })
    .to_string()
}
