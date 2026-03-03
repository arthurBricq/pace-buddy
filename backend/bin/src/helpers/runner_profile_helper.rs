use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Datelike, Utc};
use domain::{Activity, ActivityTag, DomainError};
use storage::SqliteStorage;
use storage::Storage;
use uuid::Uuid;

pub const RUNNER_PROFILE_SECTION_TITLE: &str = "Runner profile and general presentation";

pub async fn build_runner_profile_section(
    storage: &Arc<SqliteStorage>,
    user_id: Uuid,
) -> Result<String, DomainError> {
    let from = chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
        .ok_or_else(|| DomainError::Internal("Invalid lower-bound date".to_string()))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| DomainError::Internal("Invalid lower-bound datetime".to_string()))?
        .and_utc();
    let to = Utc::now() + chrono::Duration::days(1);

    let activities = storage.get_activities_in_range(user_id, from, to).await?;
    let run_activities: Vec<&Activity> = activities.iter().filter(|a| is_run(a)).collect();

    let mut distance_by_month: BTreeMap<(i32, u32), f64> = BTreeMap::new();
    let mut cumulative_distance_m = 0.0;
    let mut races: Vec<&Activity> = Vec::new();

    for activity in &run_activities {
        let date = activity.start_date.date_naive();
        *distance_by_month
            .entry((date.year(), date.month()))
            .or_insert(0.0) += activity.distance;
        cumulative_distance_m += activity.distance;

        if is_race_effort(activity) {
            races.push(activity);
        }
    }

    let mut section = String::new();
    section.push_str(&format!("## {RUNNER_PROFILE_SECTION_TITLE}\n\n"));
    section.push_str("### Last 12 months running distance\n");
    for (year, month) in last_twelve_months_keys() {
        let dist_m = distance_by_month.get(&(year, month)).copied().unwrap_or(0.0);
        section.push_str(&format!("- {:04}-{:02}: {:.1} km\n", year, month, dist_m / 1000.0));
    }

    section.push_str("\n### Cumulative running distance (all time)\n");
    section.push_str(&format!("- {:.1} km\n", cumulative_distance_m / 1000.0));

    section.push_str("\n### All races done\n");
    if races.is_empty() {
        section.push_str("- No races recorded yet.\n");
    } else {
        races.sort_by_key(|a| a.start_date);
        for race in races.iter().rev() {
            section.push_str(&format!(
                "- {}: {:.1} km in {} ({})\n",
                race.start_date.format("%Y-%m-%d"),
                race.distance / 1000.0,
                format_duration_hms(race.moving_time),
                race.name
            ));
        }
    }

    Ok(section)
}

fn is_run(activity: &Activity) -> bool {
    activity.sport_type == "Run"
}

fn is_race_effort(activity: &Activity) -> bool {
    is_run(activity) && (activity.tag == ActivityTag::Race || activity.workout_type == Some(1))
}

fn last_twelve_months_keys() -> Vec<(i32, u32)> {
    let today = Utc::now().date_naive();
    let mut year = today.year();
    let mut month = today.month();
    let mut out = Vec::with_capacity(12);

    for _ in 0..12 {
        out.push((year, month));
        if month == 1 {
            year -= 1;
            month = 12;
        } else {
            month -= 1;
        }
    }
    out.reverse();
    out
}

fn format_duration_hms(total_seconds: i32) -> String {
    let secs = total_seconds.max(0) as i64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h}:{m:02}:{s:02}")
}
