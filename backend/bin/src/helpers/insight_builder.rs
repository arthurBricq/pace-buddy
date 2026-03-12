use std::sync::Arc;

use chrono::{Datelike, NaiveDate, Utc};
use domain::{DomainError, Training, TrainingInsight};
use llm::{ChatMessage, LlmClient};
use storage::SqliteStorage;
use storage::Storage;
use uuid::Uuid;

use crate::helpers::runner_profile_helper;
use crate::state::AppState;

pub const DEFAULT_LLM_MODEL: &str = "google/gemini-2.5-flash";

const SYSTEM_PROMPT: &str =
    "You are an experienced running coach analyzing a runner's training plan. \
    Provide specific, actionable advice based on the data provided. \
    Use metric units (km, min/km pace). Be concise but remain precise.";

/// Everything needed to call the LLM for a training insight.
pub struct InsightContext {
    pub training: Training,
    pub system_prompt: String,
    pub user_prompt: String,
    pub display_label: String,
}

/// Gather all data and build the prompt for a training insight.
pub async fn build_insight_context(
    state: &actix_web::web::Data<AppState>,
    user_id: Uuid,
    training_id: Uuid,
    prompt_type: &str,
) -> Result<InsightContext, DomainError> {
    let training = state.storage.get_training(training_id, user_id).await?;
    let user_data = state.storage.get_user_by_id(user_id).await?;
    let mas_kmh = user_data.mas_current;
    let runner_profile_section =
        runner_profile_helper::build_runner_profile_section(&state.storage, user_id).await?;

    let training_activities = state
        .storage
        .get_training_activities(training_id, user_id)
        .await?;

    // Build interval descriptions for each activity
    let mut interval_descriptions = Vec::new();
    let mut long_run_descriptions = Vec::new();
    for activity in &training_activities {
        if activity.tag == domain::ActivityTag::LongRun {
            let dist_km = activity.distance / 1000.0;
            let duration_min = activity.moving_time as f64 / 60.0;
            let pace = if activity.distance > 0.0 {
                let pace_s = activity.moving_time as f64 / (activity.distance / 1000.0);
                let pm = pace_s as i32 / 60;
                let ps = pace_s as i32 % 60;
                format!("{}:{:02}/km", pm, ps)
            } else {
                "N/A".to_string()
            };
            long_run_descriptions.push(format!(
                "- {} ({}): {:.1}km, {:.0}min, pace {}",
                activity.name,
                activity.start_date.format("%Y-%m-%d"),
                dist_km,
                duration_min,
                pace,
            ));
        }

        match state.resolve_intervals(activity, None, mas_kmh).await {
            Ok(resolution) if resolution.result.is_interval_workout => {
                let result = resolution.result;
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
                    if let Some(rec_s) = rep.recovery_duration_s {
                        line.push_str(&format!(", {:.0}s recovery", rec_s));
                    }
                    line.push('\n');
                    desc.push_str(&line);
                }
                interval_descriptions.push(desc);
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!(
                    "Failed to resolve intervals for activity {} in insight builder: {e}",
                    activity.id
                );
            }
        }
    }

    // Weekly volume (all sports) within training date range
    let mut weekly_volume = String::new();
    if let (Some(start), Some(end)) = (training.start_date, training.end_date) {
        let range_activities = state
            .storage
            .get_activities_in_range(user_id, start, end)
            .await?;

        // (year, week) -> sport_type -> (distance, time, count)
        let mut weeks: std::collections::BTreeMap<
            (i32, u32),
            std::collections::BTreeMap<String, (f64, i64, i64)>,
        > = std::collections::BTreeMap::new();
        for a in &range_activities {
            let iso_week = a.start_date.iso_week();
            let key = (iso_week.year(), iso_week.week());
            let by_sport = weeks.entry(key).or_default();
            let entry = by_sport.entry(a.sport_type.clone()).or_insert((0.0, 0, 0));
            entry.0 += a.distance;
            entry.1 += a.moving_time as i64;
            entry.2 += 1;
        }

        for ((year, week), sports) in &weeks {
            let week_start = NaiveDate::from_isoywd_opt(*year, *week, chrono::Weekday::Mon)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
            weekly_volume.push_str(&format!("Week of {}:\n", week_start));
            // Sort sports with "Run" always first
            let mut sorted_sports: Vec<_> = sports.iter().collect();
            sorted_sports.sort_by_key(|(sport, _)| if sport.as_str() == "Run" { 0 } else { 1 });
            for (sport, (dist, time, count)) in sorted_sports {
                let hours = time / 3600;
                let mins = (time % 3600) / 60;
                let label = if count > &1 {
                    format!("{} activities", count)
                } else {
                    "1 activity".to_string()
                };
                weekly_volume.push_str(&format!(
                    "- {}: {:.1}km, {}h{:02}m, {}\n",
                    sport,
                    dist / 1000.0,
                    hours,
                    mins,
                    label,
                ));
            }
        }
    }

    let (base_label, request_instruction) = match prompt_type {
        "overview" => (
            "Critical overview of my training",
            "Give me a critical overview of my training so far. Analyze volume progression, interval quality, and readiness for the race goal.",
        ),
        "suggestions" => (
            "3 suggestions for future interval trainings",
            "Give me 3 specific suggestions for future interval training sessions. Consider my current fitness level, race goal, and training history.",
        ),
        _ => {
            return Err(DomainError::BadRequest(
                "prompt_type must be 'overview' or 'suggestions'".into(),
            ));
        }
    };
    let display_label = format!("{} — {}", training.name, base_label);

    // Build user prompt
    let today = Utc::now().format("%Y-%m-%d");
    let mut user_prompt = String::new();

    user_prompt.push_str(&format!("## Request\n{}\n\n", request_instruction));

    user_prompt.push_str("## Training Plan\n");
    user_prompt.push_str(&format!("Name: {}\n", training.name));
    if let Some(ref desc) = training.description {
        user_prompt.push_str(&format!("Description: {}\n", desc));
    }
    if let Some(ref goal) = training.race_distance {
        user_prompt.push_str(&format!("Race Distance: {}\n", goal));
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

    user_prompt.push('\n');
    user_prompt.push_str(&runner_profile_section);
    user_prompt.push('\n');

    if !weekly_volume.is_empty() {
        user_prompt.push_str("\n## Weekly Volume\n");
        user_prompt.push_str(&weekly_volume);
    }

    if !interval_descriptions.is_empty() {
        user_prompt.push_str("\n## Interval Sessions\n");
        for desc in &interval_descriptions {
            user_prompt.push_str(desc);
            user_prompt.push('\n');
        }
    }

    user_prompt.push_str("\n## Long Runs\n");
    if long_run_descriptions.is_empty() {
        user_prompt.push_str("- None recorded in this training.\n");
    } else {
        for desc in &long_run_descriptions {
            user_prompt.push_str(desc);
            user_prompt.push('\n');
        }
    }

    Ok(InsightContext {
        training,
        system_prompt: SYSTEM_PROMPT.to_string(),
        user_prompt,
        display_label,
    })
}

/// Call the LLM and persist the insight.
pub async fn generate_insight(
    storage: &Arc<SqliteStorage>,
    llm_client: &(impl LlmClient + ?Sized),
    context: &InsightContext,
    user_id: Uuid,
    prompt_type: &str,
    model: &str,
) -> Result<TrainingInsight, DomainError> {
    let messages = vec![
        ChatMessage::system(&context.system_prompt),
        ChatMessage::user(&context.user_prompt),
    ];

    let result = llm_client
        .chat_completion(model, messages, None)
        .await
        .map_err(|e| {
            log::error!("LLM call failed: {e}");
            DomainError::Internal(format!("LLM call failed: {e}"))
        })?;

    let insight = TrainingInsight {
        id: Uuid::new_v4(),
        training_id: context.training.id,
        user_id,
        prompt_type: prompt_type.to_string(),
        display_label: context.display_label.clone(),
        full_prompt: context.user_prompt.clone(),
        response: result.content,
        model: Some(model.to_string()),
        cost: Some(result.usage.cost),
        created_at: Utc::now(),
    };

    if let Err(e) = storage.store_training_insight(&insight).await {
        log::error!("Failed to persist training insight: {e}");
    }

    Ok(insight)
}
