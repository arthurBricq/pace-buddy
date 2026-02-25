use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use domain::DomainError;
use serde::Deserialize;
use storage::SqliteStorage;
use storage::Storage;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(tag = "context_type", rename_all = "snake_case")]
pub enum ContextRequest {
    LastActivities { count: u32 },
    ActivityDetail { activity_id: String },
    WeeklyStats { from: String, to: String },
    TrainingRecap { training_id: String },
}

pub struct ContextResult {
    pub label: String,
    pub content: String,
}

pub async fn build_context(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    request: ContextRequest,
) -> Result<ContextResult, DomainError> {
    match request {
        ContextRequest::LastActivities { count } => build_last_activities(storage, user_id, count).await,
        ContextRequest::ActivityDetail { activity_id } => {
            let id = activity_id
                .parse::<Uuid>()
                .map_err(|e| DomainError::BadRequest(format!("Invalid activity_id: {e}")))?;
            build_activity_detail(storage, user_id, id).await
        }
        ContextRequest::WeeklyStats { from, to } => build_weekly_stats(storage, user_id, &from, &to).await,
        ContextRequest::TrainingRecap { training_id } => {
            let id = training_id
                .parse::<Uuid>()
                .map_err(|e| DomainError::BadRequest(format!("Invalid training_id: {e}")))?;
            build_training_recap(storage, user_id, id).await
        }
    }
}

async fn build_last_activities(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    count: u32,
) -> Result<ContextResult, DomainError> {
    let activities = storage.get_activities(user_id, count as i64, 0).await?;
    let label = format!("Last {} activities", activities.len());

    let mut content = format!("# Last {} Activities\n\n", activities.len());
    for a in &activities {
        let dist_km = a.distance / 1000.0;
        let duration_min = a.moving_time as f64 / 60.0;
        let pace = if a.distance > 0.0 {
            let pace_s_per_km = a.moving_time as f64 / (a.distance / 1000.0);
            let pm = pace_s_per_km as i32 / 60;
            let ps = pace_s_per_km as i32 % 60;
            format!("{}:{:02}/km", pm, ps)
        } else {
            "N/A".to_string()
        };
        let hr = a
            .average_heartrate
            .map(|h| format!(", HR {:.0}bpm", h))
            .unwrap_or_default();
        content.push_str(&format!(
            "- **{}** ({}): {:.1}km, {:.0}min, pace {}{}\n",
            a.name,
            a.start_date.format("%Y-%m-%d"),
            dist_km,
            duration_min,
            pace,
            hr,
        ));
    }

    Ok(ContextResult { label, content })
}

async fn build_activity_detail(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    activity_id: Uuid,
) -> Result<ContextResult, DomainError> {
    let a = storage.get_activity(activity_id, user_id).await?;
    let label = format!("Activity: {}", a.name);

    let dist_km = a.distance / 1000.0;
    let hours = a.moving_time / 3600;
    let mins = (a.moving_time % 3600) / 60;
    let secs = a.moving_time % 60;
    let pace = if a.distance > 0.0 {
        let pace_s = a.moving_time as f64 / (a.distance / 1000.0);
        let pm = pace_s as i32 / 60;
        let ps = pace_s as i32 % 60;
        format!("{}:{:02}/km", pm, ps)
    } else {
        "N/A".to_string()
    };

    let mut content = format!("# Activity: {}\n\n", a.name);
    content.push_str(&format!("- **Date**: {}\n", a.start_date.format("%Y-%m-%d %H:%M")));
    content.push_str(&format!("- **Sport**: {}\n", a.sport_type));
    content.push_str(&format!("- **Distance**: {:.2} km\n", dist_km));
    content.push_str(&format!("- **Duration**: {}h{:02}m{:02}s\n", hours, mins, secs));
    content.push_str(&format!("- **Pace**: {}\n", pace));
    content.push_str(&format!("- **Elevation**: {:.0} m\n", a.total_elevation_gain));
    if let Some(hr) = a.average_heartrate {
        content.push_str(&format!("- **Avg HR**: {:.0} bpm\n", hr));
    }
    if let Some(max_hr) = a.max_heartrate {
        content.push_str(&format!("- **Max HR**: {:.0} bpm\n", max_hr));
    }
    if let Some(cadence) = a.average_cadence {
        content.push_str(&format!("- **Avg Cadence**: {:.0} spm\n", cadence));
    }
    if let Some(calories) = a.calories {
        content.push_str(&format!("- **Calories**: {:.0}\n", calories));
    }
    content.push_str(&format!("- **Tag**: {:?}\n", a.tag));

    Ok(ContextResult { label, content })
}

async fn build_weekly_stats(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    from: &str,
    to: &str,
) -> Result<ContextResult, DomainError> {
    let from_dt = chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|e| DomainError::BadRequest(format!("Invalid 'from' date: {e}")))?
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let to_dt = chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|e| DomainError::BadRequest(format!("Invalid 'to' date: {e}")))?
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc();

    let activities = storage.get_activities_in_range(user_id, from_dt, to_dt).await?;
    let label = format!("Weekly stats {} to {}", from, to);

    // Group by (year, week) -> sport_type -> (distance, time, count)
    let mut weeks: std::collections::BTreeMap<
        (i32, u32),
        std::collections::BTreeMap<String, (f64, i64, i64)>,
    > = std::collections::BTreeMap::new();
    for a in &activities {
        let iso_week = a.start_date.iso_week();
        let key = (iso_week.year(), iso_week.week());
        let by_sport = weeks.entry(key).or_default();
        let entry = by_sport.entry(a.sport_type.clone()).or_insert((0.0, 0, 0));
        entry.0 += a.distance;
        entry.1 += a.moving_time as i64;
        entry.2 += 1;
    }

    let mut content = format!("# Weekly Stats ({} to {})\n\n", from, to);
    for ((year, week), sports) in &weeks {
        let week_start = NaiveDate::from_isoywd_opt(*year, *week, chrono::Weekday::Mon)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
        content.push_str(&format!("**Week of {}**:\n", week_start));
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
            content.push_str(&format!(
                "- {}: {:.1}km, {}h{:02}m, {}\n",
                sport,
                dist / 1000.0,
                hours,
                mins,
                label,
            ));
        }
        content.push('\n');
    }

    Ok(ContextResult { label, content })
}

async fn build_training_recap(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    training_id: Uuid,
) -> Result<ContextResult, DomainError> {
    let training = storage.get_training(training_id, user_id).await?;
    let activities = storage.get_training_activities(training_id, user_id).await?;
    let label = format!("Training: {}", training.name);

    let mut content = format!("# Training: {}\n\n", training.name);
    if let Some(desc) = &training.description {
        content.push_str(&format!("**Description**: {}\n", desc));
    }
    if let Some(start) = training.start_date {
        content.push_str(&format!("**Start**: {}\n", start.format("%Y-%m-%d")));
    }
    if let Some(end) = training.end_date {
        content.push_str(&format!("**End**: {}\n", end.format("%Y-%m-%d")));
    }
    if let Some(goal) = &training.race_goal {
        content.push_str(&format!("**Race goal**: {}\n", goal));
    }
    if let Some(obj) = &training.race_objectif {
        content.push_str(&format!("**Race objective**: {}\n", obj));
    }

    content.push_str(&format!("\n## Activities ({} total)\n\n", activities.len()));
    for a in &activities {
        let dist_km = a.distance / 1000.0;
        let duration_min = a.moving_time as f64 / 60.0;
        let pace = if a.distance > 0.0 {
            let pace_s = a.moving_time as f64 / (a.distance / 1000.0);
            let pm = pace_s as i32 / 60;
            let ps = pace_s as i32 % 60;
            format!("{}:{:02}/km", pm, ps)
        } else {
            "N/A".to_string()
        };
        let hr = a
            .average_heartrate
            .map(|h| format!(", HR {:.0}bpm", h))
            .unwrap_or_default();
        content.push_str(&format!(
            "- **{}** ({}, {}): {:.1}km, {:.0}min, {}{}\n",
            a.name,
            a.sport_type,
            a.start_date.format("%Y-%m-%d"),
            dist_km,
            duration_min,
            pace,
            hr,
        ));
    }

    Ok(ContextResult { label, content })
}
