use actix_web::{web, HttpResponse};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::helpers::strava_data_helper::fetch_streams_from_strava;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{Training, TrainingInsight};
use llm::{ChatMessage, LlmClient};

fn parse_date(s: &str) -> Result<DateTime<Utc>, String> {
    s.parse::<DateTime<Utc>>()
        .map_err(|e| format!("Invalid date '{}': {}", s, e))
}

#[derive(Deserialize)]
pub struct CreateTrainingRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub race_goal: Option<String>,
}

pub async fn create_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CreateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /trainings user={} name={}", user.user_id, body.name);

    let start_date = body
        .start_date
        .as_deref()
        .map(parse_date)
        .transpose()
        .map_err(|e| AppError(domain::DomainError::BadRequest(e)))?;
    let end_date = body
        .end_date
        .as_deref()
        .map(parse_date)
        .transpose()
        .map_err(|e| AppError(domain::DomainError::BadRequest(e)))?;

    let training = Training {
        id: Uuid::new_v4(),
        user_id: user.user_id,
        name: body.name.clone(),
        description: body.description.clone(),
        start_date,
        end_date,
        race_goal: body.race_goal.clone(),
        created_at: Utc::now(),
    };

    state.storage.create_training(&training).await?;

    Ok(HttpResponse::Created().json(training))
}

pub async fn list_trainings(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /trainings user={}", user.user_id);

    let trainings = state.storage.list_trainings(user.user_id).await?;

    Ok(HttpResponse::Ok().json(trainings))
}

pub async fn get_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("GET /trainings/{training_id} user={}", user.user_id);

    let training = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(training))
}

#[derive(Deserialize)]
pub struct UpdateTrainingRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub race_goal: Option<String>,
}

pub async fn update_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("PATCH /trainings/{training_id} user={}", user.user_id);

    let current = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    let name = body.name.clone().unwrap_or(current.name);
    let description = body.description.clone().or(current.description);
    let start_date = match &body.start_date {
        Some(s) => Some(parse_date(s).map_err(|e| AppError(domain::DomainError::BadRequest(e)))?),
        None => current.start_date,
    };
    let end_date = match &body.end_date {
        Some(s) => Some(parse_date(s).map_err(|e| AppError(domain::DomainError::BadRequest(e)))?),
        None => current.end_date,
    };
    let race_goal = body.race_goal.clone().or(current.race_goal);

    state
        .storage
        .update_training(
            training_id,
            user.user_id,
            name,
            description,
            start_date,
            end_date,
            race_goal,
        )
        .await?;

    let updated = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(updated))
}

pub async fn delete_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("DELETE /trainings/{training_id} user={}", user.user_id);

    state
        .storage
        .delete_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn add_activity_to_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let (training_id, activity_id) = path.into_inner();
    log::info!(
        "POST /trainings/{training_id}/activities/{activity_id} user={}",
        user.user_id
    );

    state
        .storage
        .add_activity_to_training(training_id, activity_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn remove_activity_from_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let (training_id, activity_id) = path.into_inner();
    log::info!(
        "DELETE /trainings/{training_id}/activities/{activity_id} user={}",
        user.user_id
    );

    state
        .storage
        .remove_activity_from_training(training_id, activity_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn get_training_activities(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!(
        "GET /trainings/{training_id}/activities user={}",
        user.user_id
    );

    let activities = state
        .storage
        .get_training_activities(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(activities))
}

pub async fn get_activity_trainings(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
    log::info!(
        "GET /activities/{activity_id}/trainings user={}",
        user.user_id
    );

    let trainings = state
        .storage
        .get_activity_trainings(activity_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(trainings))
}

// ---------------------------------------------------------------------------
// Training insight (LLM)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct InsightRequest {
    pub prompt_type: String,
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct InsightResponse {
    pub id: String,
    pub display_label: String,
    pub full_prompt: String,
    pub response: String,
}

const DEFAULT_LLM_MODEL: &str = "google/gemini-2.5-flash";

pub async fn training_insight(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<InsightRequest>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!(
        "POST /trainings/{training_id}/insight user={} type={}",
        user.user_id,
        body.prompt_type
    );

    let llm_client = state
        .llm_client
        .as_ref()
        .ok_or_else(|| AppError(domain::DomainError::Internal("LLM not configured".into())))?;

    // Load training (verifies ownership)
    let training = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    // Load user for MAS
    let user_data = state.storage.get_user_by_id(user.user_id).await?;
    let mas_kmh = user_data.mas_current.map(|mps| mps * 3.6);

    // Load training activities (interval sessions)
    let training_activities = state
        .storage
        .get_training_activities(training_id, user.user_id)
        .await?;

    // Build interval descriptions for each activity
    let config = intervals::types::IntervalConfig::default();
    let mut interval_descriptions = Vec::new();
    for activity in &training_activities {
        // Try cached streams first, fall back to Strava
        let mut streams = state
            .storage
            .get_streams(activity.id)
            .await
            .unwrap_or_default();
        if streams.is_empty() {
            streams = match fetch_streams_from_strava(&state, activity).await {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Failed to fetch streams for activity {}: {e}", activity.id);
                    continue;
                }
            };
        }
        if streams.is_empty() {
            continue;
        }
        match intervals::parse_intervals(&streams, &config, mas_kmh) {
            Ok(result) if result.is_interval_workout => {
                let mut desc = format!(
                    "### {} ({})\n{} reps:\n",
                    activity.name,
                    activity.start_date.format("%Y-%m-%d"),
                    result.reps.len()
                );
                for rep in &result.reps {
                    let pace_min = (rep.avg_pace_s_per_km / 60.0).floor() as i32;
                    let pace_sec = (rep.avg_pace_s_per_km % 60.0).round() as i32;
                    let mut line = format!(
                        "- Rep {}: {:.0}m in {:.0}s, pace {}:{:02}/km",
                        rep.rep_index + 1,
                        rep.distance_m,
                        rep.duration_s,
                        pace_min,
                        pace_sec,
                    );
                    if let Some(pct) = rep.pct_mas {
                        line.push_str(&format!(", {:.0}% MAS", pct * 100.0));
                    }
                    if let Some(ref recovery) = rep.recovery {
                        let style = rep
                            .recovery_style
                            .map(|s| format!("{:?}", s))
                            .unwrap_or_else(|| "Unknown".into());
                        line.push_str(&format!(
                            ", recovery: {} ({:.0}s)",
                            style, recovery.duration_s
                        ));
                    }
                    line.push('\n');
                    desc.push_str(&line);
                }
                interval_descriptions.push(desc);
            }
            _ => {}
        }
    }

    // Weekly running volume within training date range
    let mut weekly_volume = String::new();
    if let (Some(start), Some(end)) = (training.start_date, training.end_date) {
        let range_activities = state
            .storage
            .get_activities_in_range(user.user_id, start, end)
            .await?;

        // Group by ISO week
        let mut weeks: std::collections::BTreeMap<(i32, u32), (f64, i64, i64)> =
            std::collections::BTreeMap::new();
        for a in &range_activities {
            if a.sport_type != "Run" {
                continue;
            }
            let iso_week = a.start_date.iso_week();
            let key = (iso_week.year(), iso_week.week());
            let entry = weeks.entry(key).or_insert((0.0, 0, 0));
            entry.0 += a.distance;
            entry.1 += a.moving_time as i64;
            entry.2 += 1;
        }

        for ((year, week), (dist, time, count)) in &weeks {
            let week_start = NaiveDate::from_isoywd_opt(*year, *week, chrono::Weekday::Mon)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
            let hours = time / 3600;
            let mins = (time % 3600) / 60;
            weekly_volume.push_str(&format!(
                "Week of {}: {:.1}km, {}h{:02}m, {} runs\n",
                week_start,
                dist / 1000.0,
                hours,
                mins,
                count
            ));
        }
    }

    let (display_label, request_instruction) = match body.prompt_type.as_str() {
        "overview" => (
            "Critical overview of my training",
            "Give me a critical overview of my training so far. Analyze volume progression, interval quality, and readiness for the race goal.",
        ),
        "suggestions" => (
            "3 suggestions for future interval trainings",
            "Give me 3 specific suggestions for future interval training sessions. Consider my current fitness level, race goal, and training history.",
        ),
        _ => {
            return Err(AppError(domain::DomainError::BadRequest(
                "prompt_type must be 'overview' or 'suggestions'".into(),
            )));
        }
    };

    // Build user prompt — request first, then context
    let today = Utc::now().format("%Y-%m-%d");
    let mut user_prompt = String::new();

    user_prompt.push_str(&format!("## Request\n{}\n\n", request_instruction));

    user_prompt.push_str("## Training Plan\n");
    user_prompt.push_str(&format!("Name: {}\n", training.name));
    if let Some(ref desc) = training.description {
        user_prompt.push_str(&format!("Description: {}\n", desc));
    }
    if let Some(ref goal) = training.race_goal {
        user_prompt.push_str(&format!("Race Goal: {}\n", goal));
    }
    if let Some(start) = training.start_date {
        user_prompt.push_str(&format!("Start: {}\n", start.format("%Y-%m-%d")));
    }
    if let Some(end) = training.end_date {
        user_prompt.push_str(&format!("End: {}\n", end.format("%Y-%m-%d")));
    }
    user_prompt.push_str(&format!("Today's date: {}\n", today));
    if let Some(mas) = mas_kmh {
        user_prompt.push_str(&format!("Runner's MAS: {:.1} km/h\n", mas));
    }

    if !weekly_volume.is_empty() {
        user_prompt.push_str("\n## Weekly Running Volume\n");
        user_prompt.push_str(&weekly_volume);
    }

    if !interval_descriptions.is_empty() {
        user_prompt.push_str("\n## Interval Sessions\n");
        for desc in &interval_descriptions {
            user_prompt.push_str(desc);
            user_prompt.push('\n');
        }
    }

    let system_prompt = "You are an experienced running coach analyzing a runner's training plan. \
        Provide specific, actionable advice based on the data provided. \
        Use metric units (km, min/km pace). Be concise but remain precise.";

    let messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt),
    ];

    let model = body.model.as_deref().unwrap_or(DEFAULT_LLM_MODEL);
    let result = llm_client
        .chat_completion(model, messages, None)
        .await
        .map_err(|e| {
            log::error!("LLM call failed: {e}");
            AppError(domain::DomainError::Internal(format!(
                "LLM call failed: {e}"
            )))
        })?;

    let insight_id = Uuid::new_v4();
    let insight = TrainingInsight {
        id: insight_id,
        training_id,
        user_id: user.user_id,
        prompt_type: body.prompt_type.clone(),
        display_label: display_label.to_string(),
        full_prompt: user_prompt.clone(),
        response: result.content.clone(),
        model: Some(model.to_string()),
        cost: Some(result.usage.cost),
        created_at: Utc::now(),
    };

    if let Err(e) = state.storage.store_training_insight(&insight).await {
        log::error!("Failed to persist training insight: {e}");
    }

    Ok(HttpResponse::Ok().json(InsightResponse {
        id: insight_id.to_string(),
        display_label: display_label.to_string(),
        full_prompt: user_prompt,
        response: result.content,
    }))
}

// ---------------------------------------------------------------------------
// List training insights
// ---------------------------------------------------------------------------

pub async fn list_training_insights(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::debug!(
        "GET /trainings/{training_id}/insights user={}",
        user.user_id
    );

    // Verify training ownership
    let _training = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    let insights = state
        .storage
        .get_training_insights(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(insights))
}
