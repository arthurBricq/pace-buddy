use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use domain::{Activity, ActivityTag, DomainError};
use serde::Deserialize;
use storage::SqliteStorage;
use storage::Storage;
use uuid::Uuid;

use crate::helpers::runner_profile_helper;

#[derive(Deserialize)]
#[serde(tag = "context_type", rename_all = "snake_case")]
pub enum ContextRequest {
    LastActivities { count: u32 },
    LastLongRuns { count: u32 },
    LastRaceEfforts { count: u32 },
    LastDaysSummary { days: u32 },
    ThisWeekVsLastWeek,
    ThisMonthVsLastMonth,
    RunnerProfilePresentation,
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
        ContextRequest::LastActivities { count } => {
            build_last_activities(storage, user_id, count).await
        }
        ContextRequest::LastLongRuns { count } => {
            build_last_long_runs(storage, user_id, count).await
        }
        ContextRequest::LastRaceEfforts { count } => {
            build_last_race_efforts(storage, user_id, count).await
        }
        ContextRequest::LastDaysSummary { days } => {
            build_last_days_summary(storage, user_id, days).await
        }
        ContextRequest::ThisWeekVsLastWeek => build_this_week_vs_last_week(storage, user_id).await,
        ContextRequest::ThisMonthVsLastMonth => {
            build_this_month_vs_last_month(storage, user_id).await
        }
        ContextRequest::RunnerProfilePresentation => {
            build_runner_profile_presentation(storage, user_id).await
        }
        ContextRequest::ActivityDetail { activity_id } => {
            let id = activity_id
                .parse::<Uuid>()
                .map_err(|e| DomainError::BadRequest(format!("Invalid activity_id: {e}")))?;
            build_activity_detail(storage, user_id, id).await
        }
        ContextRequest::WeeklyStats { from, to } => {
            build_weekly_stats(storage, user_id, &from, &to).await
        }
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

async fn build_runner_profile_presentation(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
) -> Result<ContextResult, DomainError> {
    let content = runner_profile_helper::build_runner_profile_section(storage, user_id).await?;
    Ok(ContextResult {
        label: runner_profile_helper::RUNNER_PROFILE_SECTION_TITLE.to_string(),
        content,
    })
}

async fn build_last_long_runs(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    count: u32,
) -> Result<ContextResult, DomainError> {
    let requested_count = sanitize_count(count, 50);
    let activities = storage.get_activities(user_id, 1000, 0).await?;
    let long_runs: Vec<Activity> = activities
        .into_iter()
        .filter(is_long_run_activity)
        .take(requested_count)
        .collect();

    let label = format!("Last {} long runs", long_runs.len());
    let mut content = format!("# Last {} Long Runs\n\n", long_runs.len());
    if long_runs.is_empty() {
        content.push_str("- No long runs found.\n");
    } else {
        for a in &long_runs {
            content.push_str(&format!("- {}\n", format_activity_line(a)));
        }
    }

    Ok(ContextResult { label, content })
}

async fn build_last_race_efforts(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    count: u32,
) -> Result<ContextResult, DomainError> {
    let requested_count = sanitize_count(count, 50);
    let activities = storage.get_activities(user_id, 1000, 0).await?;
    let race_efforts: Vec<Activity> = activities
        .into_iter()
        .filter(is_race_effort_activity)
        .take(requested_count)
        .collect();

    let label = format!("Last {} race efforts", race_efforts.len());
    let mut content = format!("# Last {} Race Efforts\n\n", race_efforts.len());
    if race_efforts.is_empty() {
        content.push_str("- No race efforts found.\n");
    } else {
        for a in &race_efforts {
            content.push_str(&format!("- {}\n", format_activity_line(a)));
        }
    }

    Ok(ContextResult { label, content })
}

async fn build_last_days_summary(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    days: u32,
) -> Result<ContextResult, DomainError> {
    let days = sanitize_count(days, 120) as i64;
    let today = Utc::now().date_naive();
    let start_date = today - Duration::days(days - 1);

    let activities = storage
        .get_activities_in_range(
            user_id,
            at_start_of_day_utc(start_date),
            at_start_of_day_utc(today + Duration::days(1)),
        )
        .await?;
    let summary = summarize_activities(&activities);

    let label = format!("Last {} days summary", days);
    let mut content = format!(
        "# Last {} Days Summary ({} to {})\n\n",
        days,
        start_date.format("%Y-%m-%d"),
        today.format("%Y-%m-%d"),
    );
    content.push_str(&format!(
        "- **Runs**: {} ({} quality)\n",
        summary.run_count, summary.quality_count
    ));
    content.push_str(&format!(
        "- **Distance**: {:.1}km\n",
        summary.distance_m / 1000.0
    ));
    content.push_str(&format!(
        "- **Duration**: {}h{:02}m\n\n",
        summary.moving_time_s / 3600,
        (summary.moving_time_s % 3600) / 60
    ));

    let mut day_map: BTreeMap<NaiveDate, Vec<Activity>> = BTreeMap::new();
    for a in activities {
        day_map
            .entry(a.start_date.date_naive())
            .or_default()
            .push(a);
    }

    content.push_str("## Day by day\n\n");
    for offset in 0..days {
        let day = start_date + Duration::days(offset);
        if let Some(day_activities) = day_map.get(&day) {
            let day_summary = summarize_activities(day_activities);
            content.push_str(&format!(
                "- **{}**: {} run(s), {:.1}km, {} quality (intervals {}, long runs {}, races {})\n",
                day.format("%Y-%m-%d"),
                day_summary.run_count,
                day_summary.distance_m / 1000.0,
                day_summary.quality_count,
                day_summary.intervals_count,
                day_summary.long_runs_count,
                day_summary.races_count,
            ));
        } else {
            content.push_str(&format!("- **{}**: rest day\n", day.format("%Y-%m-%d")));
        }
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
    content.push_str(&format!(
        "- **Date**: {}\n",
        a.start_date.format("%Y-%m-%d %H:%M")
    ));
    content.push_str(&format!("- **Sport**: {}\n", a.sport_type));
    content.push_str(&format!("- **Distance**: {:.2} km\n", dist_km));
    content.push_str(&format!(
        "- **Duration**: {}h{:02}m{:02}s\n",
        hours, mins, secs
    ));
    content.push_str(&format!("- **Pace**: {}\n", pace));
    content.push_str(&format!(
        "- **Elevation**: {:.0} m\n",
        a.total_elevation_gain
    ));
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
        .ok_or_else(|| DomainError::BadRequest("Invalid 'from' date".to_string()))?
        .and_utc();
    let to_date = chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|e| DomainError::BadRequest(format!("Invalid 'to' date: {e}")))?
        + Duration::days(1);
    let to_dt = to_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| DomainError::BadRequest("Invalid 'to' date".to_string()))?
        .and_utc();

    let activities = storage
        .get_activities_in_range(user_id, from_dt, to_dt)
        .await?;
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

async fn build_this_week_vs_last_week(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
) -> Result<ContextResult, DomainError> {
    let today = Utc::now().date_naive();
    let this_week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let days_elapsed = (today - this_week_start).num_days() + 1;
    let this_week_end = this_week_start + Duration::days(days_elapsed);
    let last_week_start = this_week_start - Duration::days(7);
    let last_week_end = last_week_start + Duration::days(days_elapsed);

    let this_week = storage
        .get_activities_in_range(
            user_id,
            at_start_of_day_utc(this_week_start),
            at_start_of_day_utc(this_week_end),
        )
        .await?;
    let last_week = storage
        .get_activities_in_range(
            user_id,
            at_start_of_day_utc(last_week_start),
            at_start_of_day_utc(last_week_end),
        )
        .await?;

    let this_summary = summarize_activities(&this_week);
    let last_summary = summarize_activities(&last_week);

    let label = "This week vs last week".to_string();
    let mut content = String::from("# This Week vs Last Week\n\n");
    content.push_str(&format!(
        "- **This week window**: {} to {}\n",
        this_week_start.format("%Y-%m-%d"),
        (this_week_end - Duration::days(1)).format("%Y-%m-%d")
    ));
    content.push_str(&format!(
        "- **Last week window**: {} to {}\n\n",
        last_week_start.format("%Y-%m-%d"),
        (last_week_end - Duration::days(1)).format("%Y-%m-%d")
    ));

    content.push_str("## This week\n");
    content.push_str(&format_period_summary(&this_summary));
    content.push_str("\n## Last week\n");
    content.push_str(&format_period_summary(&last_summary));
    content.push_str("\n## Delta (this - last)\n");
    content.push_str(&format!(
        "- **Distance**: {:+.1}km\n",
        (this_summary.distance_m - last_summary.distance_m) / 1000.0
    ));
    content.push_str(&format!(
        "- **Runs**: {:+}\n",
        this_summary.run_count - last_summary.run_count
    ));
    content.push_str(&format!(
        "- **Quality sessions**: {:+}\n",
        this_summary.quality_count - last_summary.quality_count
    ));

    Ok(ContextResult { label, content })
}

async fn build_this_month_vs_last_month(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
) -> Result<ContextResult, DomainError> {
    let today = Utc::now().date_naive();
    let this_month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .ok_or_else(|| DomainError::Internal("Invalid current month start".to_string()))?;
    let days_elapsed = (today - this_month_start).num_days() + 1;
    let this_month_end = this_month_start + Duration::days(days_elapsed);

    let (prev_month_year, prev_month) = if today.month() == 1 {
        (today.year() - 1, 12)
    } else {
        (today.year(), today.month() - 1)
    };
    let prev_month_start = NaiveDate::from_ymd_opt(prev_month_year, prev_month, 1)
        .ok_or_else(|| DomainError::Internal("Invalid previous month start".to_string()))?;
    let prev_month_days = days_in_month(prev_month_year, prev_month) as i64;
    let prev_window_days = days_elapsed.min(prev_month_days);
    let prev_month_end = prev_month_start + Duration::days(prev_window_days);

    let this_month = storage
        .get_activities_in_range(
            user_id,
            at_start_of_day_utc(this_month_start),
            at_start_of_day_utc(this_month_end),
        )
        .await?;
    let prev_month_window = storage
        .get_activities_in_range(
            user_id,
            at_start_of_day_utc(prev_month_start),
            at_start_of_day_utc(prev_month_end),
        )
        .await?;

    let this_summary = summarize_activities(&this_month);
    let prev_summary = summarize_activities(&prev_month_window);

    let label = "This month vs last month".to_string();
    let mut content = String::from("# This Month vs Last Month\n\n");
    content.push_str(&format!(
        "- **This month window**: {} to {}\n",
        this_month_start.format("%Y-%m-%d"),
        (this_month_end - Duration::days(1)).format("%Y-%m-%d")
    ));
    content.push_str(&format!(
        "- **Last month window (same number of days)**: {} to {}\n\n",
        prev_month_start.format("%Y-%m-%d"),
        (prev_month_end - Duration::days(1)).format("%Y-%m-%d")
    ));

    content.push_str("## This month\n");
    content.push_str(&format_period_summary(&this_summary));
    content.push_str("\n## Last month\n");
    content.push_str(&format_period_summary(&prev_summary));
    content.push_str("\n## Delta (this - last)\n");
    content.push_str(&format!(
        "- **Distance**: {:+.1}km\n",
        (this_summary.distance_m - prev_summary.distance_m) / 1000.0
    ));
    content.push_str(&format!(
        "- **Runs**: {:+}\n",
        this_summary.run_count - prev_summary.run_count
    ));
    content.push_str(&format!(
        "- **Quality sessions**: {:+}\n",
        this_summary.quality_count - prev_summary.quality_count
    ));

    Ok(ContextResult { label, content })
}

async fn build_training_recap(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
    training_id: Uuid,
) -> Result<ContextResult, DomainError> {
    let training = storage.get_training(training_id, user_id).await?;
    let activities = storage
        .get_training_activities(training_id, user_id)
        .await?;
    let long_runs: Vec<&domain::Activity> = activities
        .iter()
        .filter(|a| a.tag == domain::ActivityTag::LongRun)
        .collect();
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
    if let Some(goal) = &training.race_distance {
        content.push_str(&format!("**Race Distance**: {}\n", goal));
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

    content.push_str(&format!("\n## Long Runs ({} total)\n\n", long_runs.len()));
    if long_runs.is_empty() {
        content.push_str("- No long runs in this training.\n");
    } else {
        for a in long_runs {
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
            content.push_str(&format!(
                "- **{}** ({}): {:.1}km, {:.0}min, {}\n",
                a.name,
                a.start_date.format("%Y-%m-%d"),
                dist_km,
                duration_min,
                pace,
            ));
        }
    }

    Ok(ContextResult { label, content })
}

fn sanitize_count(value: u32, max: u32) -> usize {
    value.clamp(1, max) as usize
}

fn at_start_of_day_utc(date: NaiveDate) -> chrono::DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .expect("valid day boundary")
        .and_utc()
}

fn is_run_activity(activity: &Activity) -> bool {
    activity.sport_type == "Run"
}

fn is_long_run_activity(activity: &Activity) -> bool {
    is_run_activity(activity)
        && (activity.tag == ActivityTag::LongRun || activity.workout_type == Some(2))
}

fn is_race_effort_activity(activity: &Activity) -> bool {
    is_run_activity(activity)
        && (activity.tag == ActivityTag::Race || activity.workout_type == Some(1))
}

fn is_quality_activity(activity: &Activity) -> bool {
    is_run_activity(activity)
        && matches!(
            activity.tag,
            ActivityTag::Intervals | ActivityTag::LongRun | ActivityTag::Race
        )
}

fn format_pace(distance_m: f64, moving_time_s: i32) -> String {
    if distance_m <= 0.0 {
        return "N/A".to_string();
    }
    let pace_s = moving_time_s as f64 / (distance_m / 1000.0);
    let pm = pace_s as i32 / 60;
    let ps = pace_s as i32 % 60;
    format!("{pm}:{ps:02}/km")
}

fn format_activity_line(activity: &Activity) -> String {
    let hr = activity
        .average_heartrate
        .map(|h| format!(", HR {:.0}bpm", h))
        .unwrap_or_default();
    format!(
        "**{}** ({}): {:.1}km, {:.0}min, pace {}{}, tag {:?}",
        activity.name,
        activity.start_date.format("%Y-%m-%d"),
        activity.distance / 1000.0,
        activity.moving_time as f64 / 60.0,
        format_pace(activity.distance, activity.moving_time),
        hr,
        activity.tag,
    )
}

#[derive(Default)]
struct ActivitySummary {
    run_count: i64,
    distance_m: f64,
    moving_time_s: i64,
    quality_count: i64,
    intervals_count: i64,
    long_runs_count: i64,
    races_count: i64,
}

fn summarize_activities(activities: &[Activity]) -> ActivitySummary {
    let mut summary = ActivitySummary::default();
    for activity in activities {
        if !is_run_activity(activity) {
            continue;
        }
        summary.run_count += 1;
        summary.distance_m += activity.distance;
        summary.moving_time_s += activity.moving_time as i64;

        if is_quality_activity(activity) {
            summary.quality_count += 1;
        }
        match activity.tag {
            ActivityTag::Intervals => summary.intervals_count += 1,
            ActivityTag::LongRun => summary.long_runs_count += 1,
            ActivityTag::Race => summary.races_count += 1,
            ActivityTag::Normal => {}
        }
    }
    summary
}

fn format_period_summary(summary: &ActivitySummary) -> String {
    format!(
        "- **Runs**: {} ({} quality)\n- **Distance**: {:.1}km\n- **Duration**: {}h{:02}m\n- **Quality split**: intervals {}, long runs {}, races {}\n",
        summary.run_count,
        summary.quality_count,
        summary.distance_m / 1000.0,
        summary.moving_time_s / 3600,
        (summary.moving_time_s % 3600) / 60,
        summary.intervals_count,
        summary.long_runs_count,
        summary.races_count,
    )
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_of_next = NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid month");
    (first_of_next - Duration::days(1)).day()
}
