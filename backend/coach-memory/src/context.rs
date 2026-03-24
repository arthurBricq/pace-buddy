use std::collections::BTreeMap;

use chrono::{Datelike, Duration, Utc};
use domain::{
    Activity, ActivityTag, DomainError, RunningCoachMemoryData, RunningCoachSettings,
    RunningCoachState,
};
use uuid::Uuid;

use crate::store::CoachMemoryDataStore;

pub struct CoachContextBundle {
    pub content: String,
    pub latest_seen_activity_start_date: Option<chrono::DateTime<Utc>>,
}

pub async fn build_coach_context(
    store: &(impl CoachMemoryDataStore + ?Sized),
    user_id: Uuid,
    settings: &RunningCoachSettings,
    state: &RunningCoachState,
    memory: &RunningCoachMemoryData,
) -> Result<CoachContextBundle, DomainError> {
    let volume_weeks = clamp(settings.volume_weeks, 1, 24, 8) as i64;
    let workouts_count = clamp(settings.last_workouts_count, 1, 25, 8) as usize;
    let long_runs_count = clamp(settings.last_long_runs_count, 1, 25, 6) as usize;
    let races_count = clamp(settings.last_races_count, 1, 25, 4) as usize;
    let new_activities_count = clamp(settings.new_activities_count, 1, 25, 8) as usize;

    let user = store.get_user_by_id(user_id).await?;
    let identity_profile = store.get_identity_profile(user_id).await?;
    let athlete_profile = store.get_athlete_profile(user_id).await?;

    let from = (Utc::now() - Duration::weeks(volume_weeks))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| DomainError::Internal("Invalid datetime for coach context".to_string()))?
        .and_utc();
    let to = Utc::now() + Duration::days(1);
    let activities_in_window = store.get_activities_in_range(user_id, from, to).await?;
    let activities_recent = store.get_activities(user_id, 500, 0).await?;

    let workouts: Vec<&Activity> = activities_recent
        .iter()
        .filter(|a| is_run(a) && a.tag == ActivityTag::Intervals)
        .take(workouts_count)
        .collect();
    let long_runs: Vec<&Activity> = activities_recent
        .iter()
        .filter(|a| is_run(a) && a.tag == ActivityTag::LongRun)
        .take(long_runs_count)
        .collect();
    let races: Vec<&Activity> = activities_recent
        .iter()
        .filter(|a| is_race(a))
        .take(races_count)
        .collect();

    let all_new_activities: Vec<&Activity> =
        if let Some(last_seen) = state.last_seen_activity_start_date {
            activities_recent
                .iter()
                .filter(|a| a.start_date > last_seen)
                .collect()
        } else {
            activities_recent.iter().collect()
        };
    let new_activities: Vec<&Activity> = all_new_activities
        .iter()
        .copied()
        .take(new_activities_count)
        .collect();
    let latest_seen_activity_start_date = all_new_activities
        .iter()
        .map(|a| a.start_date)
        .max()
        .or(state.last_seen_activity_start_date);

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut content = String::new();
    content.push_str("# Running Coach Grounding Context\n\n");
    content.push_str(&format!("- Current date: {}\n", today));
    content.push_str(&format!("- Athlete username: {}\n", user.username));
    if let Some(mas) = user.mas_current {
        content.push_str(&format!("- MAS: {:.1} km/h\n", mas));
    }

    content.push_str("\n## User profile\n");
    if let Some(identity) = identity_profile {
        if let Some(name) = identity.name {
            content.push_str(&format!("- Name: {}\n", name));
        }
        if let Some(age) = identity.age {
            content.push_str(&format!("- Age: {}\n", age));
        }
        if let Some(gender) = identity.gender {
            content.push_str(&format!("- Gender: {}\n", gender));
        }
    }
    if let Some(athlete) = athlete_profile {
        if let Some(goal_description) = athlete.goal_description {
            content.push_str(&format!("- Goal: {}\n", goal_description));
        }
        if let Some(goal_date) = athlete.goal_date {
            content.push_str(&format!("- Goal date: {}\n", goal_date));
        }
        if let Some(goal_distance_km) = athlete.goal_distance_km {
            content.push_str(&format!("- Goal distance: {:.1} km\n", goal_distance_km));
        }
        if let Some(goal_target_time_seconds) = athlete.goal_target_time_seconds {
            content.push_str(&format!(
                "- Goal target time: {}\n",
                format_duration(goal_target_time_seconds)
            ));
        }
        if let Some(additional_info) = athlete.additional_info {
            content.push_str(&format!("- Additional info: {}\n", additional_info));
        }
    }

    content.push_str(&format!("\n## Volume over last {} weeks\n", volume_weeks));
    let run_activities_window: Vec<&Activity> =
        activities_in_window.iter().filter(|a| is_run(a)).collect();
    let total_distance_km: f64 = run_activities_window
        .iter()
        .map(|a| a.distance)
        .sum::<f64>()
        / 1000.0;
    let total_time_s: i64 = run_activities_window
        .iter()
        .map(|a| a.moving_time as i64)
        .sum();
    content.push_str(&format!(
        "- Runs: {}\n- Distance: {:.1} km\n- Moving time: {}\n",
        run_activities_window.len(),
        total_distance_km,
        format_duration_i64(total_time_s)
    ));
    for weekly_line in summarize_weekly_volume(&run_activities_window) {
        content.push_str(&format!("- {}\n", weekly_line));
    }

    content.push_str(&format!(
        "\n## Last {} workout sessions (interval tag)\n",
        workouts.len()
    ));
    push_activity_lines(&mut content, &workouts);

    content.push_str(&format!("\n## Last {} long runs\n", long_runs.len()));
    push_activity_lines(&mut content, &long_runs);

    content.push_str(&format!("\n## Last {} races\n", races.len()));
    push_activity_lines(&mut content, &races);

    if state.last_seen_activity_start_date.is_some() {
        content.push_str(&format!(
            "\n## New activities since last exchange (showing up to {})\n",
            new_activities_count
        ));
    } else {
        content.push_str(&format!(
            "\n## Initial activities snapshot (first {} activities)\n",
            new_activities_count
        ));
    }
    push_activity_lines(&mut content, &new_activities);

    content.push_str("\n## Recent tool results\n");
    content
        .push_str("Compact summaries of recent session lookup tool usage from prior exchanges.\n");
    if memory.recent_tool_results.is_empty() {
        content.push_str("- None yet.\n");
    } else {
        for item in &memory.recent_tool_results {
            content.push_str(&format!("- {}\n", item));
        }
    }

    content.push_str("\n## Coach memory snapshot\n");
    if memory.pinned_facts.is_empty()
        && memory.active_coaching_plan.trim().is_empty()
        && memory.episodic_memory.is_empty()
        && memory.rolling_summary.trim().is_empty()
        && memory.recent_tool_results.is_empty()
    {
        content.push_str("- No memory yet.\n");
    } else {
        if !memory.pinned_facts.is_empty() {
            content.push_str("- Pinned facts:\n");
            for fact in &memory.pinned_facts {
                content.push_str(&format!("  - {}\n", fact));
            }
        }
        if !memory.active_coaching_plan.trim().is_empty() {
            content.push_str(&format!(
                "- Active coaching plan: {}\n",
                memory.active_coaching_plan
            ));
        }
        if !memory.episodic_memory.is_empty() {
            content.push_str("- Episodic memory:\n");
            for item in &memory.episodic_memory {
                content.push_str(&format!("  - {}\n", item));
            }
        }
        if !memory.rolling_summary.trim().is_empty() {
            content.push_str(&format!("- Rolling summary: {}\n", memory.rolling_summary));
        }
    }

    Ok(CoachContextBundle {
        content,
        latest_seen_activity_start_date,
    })
}

fn summarize_weekly_volume(activities: &[&Activity]) -> Vec<String> {
    let mut by_week: BTreeMap<(i32, u32), (usize, f64, i64)> = BTreeMap::new();
    for activity in activities {
        let week = activity.start_date.iso_week();
        let key = (week.year(), week.week());
        let entry = by_week.entry(key).or_insert((0, 0.0, 0));
        entry.0 += 1;
        entry.1 += activity.distance;
        entry.2 += activity.moving_time as i64;
    }

    by_week
        .iter()
        .rev()
        .take(10)
        .map(|((year, week), (count, distance_m, time_s))| {
            format!(
                "{}-W{:02}: {} runs, {:.1} km, {}",
                year,
                week,
                count,
                distance_m / 1000.0,
                format_duration_i64(*time_s)
            )
        })
        .collect()
}

fn push_activity_lines(content: &mut String, activities: &[&Activity]) {
    if activities.is_empty() {
        content.push_str("- None.\n");
        return;
    }
    for activity in activities {
        content.push_str(&format!("- {}\n", format_activity_line(activity)));
    }
}

fn format_activity_line(activity: &Activity) -> String {
    let distance_km = activity.distance / 1000.0;
    let pace = if activity.distance > 0.0 {
        let pace_s = activity.moving_time as f64 / (activity.distance / 1000.0);
        let pace_min = (pace_s / 60.0).floor() as i64;
        let pace_sec = (pace_s as i64) % 60;
        format!("{}:{:02}/km", pace_min, pace_sec)
    } else {
        "N/A".to_string()
    };
    format!(
        "{} ({}) - {:.1} km in {} at {}, elevation +{:.0} m, tag={}",
        activity.name,
        activity.start_date.format("%Y-%m-%d"),
        distance_km,
        format_duration(activity.moving_time),
        pace,
        activity.total_elevation_gain,
        activity.tag
    )
}

fn format_duration(seconds: i32) -> String {
    format_duration_i64(seconds.max(0) as i64)
}

fn format_duration_i64(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else {
        format!("{m}m{s:02}s")
    }
}

fn is_run(activity: &Activity) -> bool {
    activity.sport_type == "Run"
}

fn is_race(activity: &Activity) -> bool {
    is_run(activity) && (activity.tag == ActivityTag::Race || activity.workout_type == Some(1))
}

fn clamp(value: i32, min: i32, max: i32, fallback: i32) -> i32 {
    let val = if value <= 0 { fallback } else { value };
    val.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::Duration;
    use domain::{
        Activity, ActivityTag, AthleteProfile, IdentityProfile, RunningCoachMemory,
        RunningCoachMemoryData, RunningCoachMessage, RunningCoachSettings, RunningCoachState, User,
    };
    use uuid::Uuid;

    use super::{build_coach_context, CoachMemoryDataStore};

    struct FakeStore {
        user: User,
        identity_profile: Option<IdentityProfile>,
        athlete_profile: Option<AthleteProfile>,
        activities_in_range: Vec<Activity>,
        activities_recent: Vec<Activity>,
        coach_messages: Vec<RunningCoachMessage>,
    }

    #[async_trait]
    impl CoachMemoryDataStore for FakeStore {
        async fn get_or_create_running_coach_settings(
            &self,
            _user_id: Uuid,
        ) -> Result<RunningCoachSettings, domain::DomainError> {
            Err(domain::DomainError::Internal(
                "unused in this test".to_string(),
            ))
        }

        async fn get_or_create_running_coach_memory(
            &self,
            _user_id: Uuid,
        ) -> Result<RunningCoachMemory, domain::DomainError> {
            Err(domain::DomainError::Internal(
                "unused in this test".to_string(),
            ))
        }

        async fn get_or_create_running_coach_state(
            &self,
            _user_id: Uuid,
        ) -> Result<RunningCoachState, domain::DomainError> {
            Err(domain::DomainError::Internal(
                "unused in this test".to_string(),
            ))
        }

        async fn upsert_running_coach_state(
            &self,
            _state: &RunningCoachState,
        ) -> Result<(), domain::DomainError> {
            Err(domain::DomainError::Internal(
                "unused in this test".to_string(),
            ))
        }

        async fn upsert_running_coach_memory(
            &self,
            _memory: &RunningCoachMemory,
        ) -> Result<(), domain::DomainError> {
            Err(domain::DomainError::Internal(
                "unused in this test".to_string(),
            ))
        }

        async fn store_running_coach_message(
            &self,
            _msg: &RunningCoachMessage,
        ) -> Result<(), domain::DomainError> {
            Err(domain::DomainError::Internal(
                "unused in this test".to_string(),
            ))
        }

        async fn get_user_by_id(&self, _user_id: Uuid) -> Result<User, domain::DomainError> {
            Ok(self.user.clone())
        }

        async fn get_identity_profile(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<IdentityProfile>, domain::DomainError> {
            Ok(self.identity_profile.clone())
        }

        async fn get_athlete_profile(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<AthleteProfile>, domain::DomainError> {
            Ok(self.athlete_profile.clone())
        }

        async fn get_activities_in_range(
            &self,
            _user_id: Uuid,
            _from: chrono::DateTime<chrono::Utc>,
            _to: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Activity>, domain::DomainError> {
            Ok(self.activities_in_range.clone())
        }

        async fn get_activities(
            &self,
            _user_id: Uuid,
            _limit: i64,
            _offset: i64,
        ) -> Result<Vec<Activity>, domain::DomainError> {
            Ok(self.activities_recent.clone())
        }

        async fn list_running_coach_messages(
            &self,
            _user_id: Uuid,
            _limit: i64,
        ) -> Result<Vec<RunningCoachMessage>, domain::DomainError> {
            Ok(self.coach_messages.clone())
        }
    }

    #[tokio::test]
    async fn includes_recent_tool_results_and_tracks_latest_new_activity() {
        let user_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let settings = RunningCoachSettings {
            user_id,
            ..RunningCoachSettings::default()
        };
        let state = RunningCoachState {
            user_id,
            last_interaction_at: None,
            last_seen_activity_start_date: Some(now - Duration::days(5)),
            updated_at: now,
        };

        let recent_activities = vec![
            fake_activity(
                user_id,
                "Easy Run",
                now - Duration::days(1),
                ActivityTag::Normal,
            ),
            fake_activity(
                user_id,
                "Intervals",
                now - Duration::days(3),
                ActivityTag::Intervals,
            ),
            fake_activity(
                user_id,
                "Long Run",
                now - Duration::days(7),
                ActivityTag::LongRun,
            ),
        ];

        let store = FakeStore {
            user: User {
                id: user_id,
                username: "runner".to_string(),
                display_name: "Runner".to_string(),
                email: None,
                created_at: now,
                mas_current: Some(17.4),
                quota_balance_usd: 1.0,
            },
            identity_profile: None,
            athlete_profile: None,
            activities_in_range: recent_activities.clone(),
            activities_recent: recent_activities,
            coach_messages: Vec::new(),
        };

        let memory = RunningCoachMemoryData {
            recent_tool_results: vec![
                "get_last_sessions -> 1 match: Easy Run on 2026-03-23".to_string()
            ],
            ..RunningCoachMemoryData::default()
        };

        let bundle = build_coach_context(&store, user_id, &settings, &state, &memory)
            .await
            .expect("context should build");

        assert!(bundle.content.contains("## Recent tool results"));
        assert!(bundle
            .content
            .contains("get_last_sessions -> 1 match: Easy Run on 2026-03-23"));
        assert!(bundle.content.contains("elevation +100 m"));
        assert_eq!(
            bundle.latest_seen_activity_start_date,
            Some(now - Duration::days(1))
        );
    }

    fn fake_activity(
        user_id: Uuid,
        name: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        tag: ActivityTag,
    ) -> Activity {
        Activity {
            id: Uuid::new_v4(),
            user_id,
            strava_id: 1,
            name: name.to_string(),
            sport_type: "Run".to_string(),
            start_date,
            elapsed_time: 3600,
            moving_time: 3540,
            distance: 10_000.0,
            total_elevation_gain: 100.0,
            average_speed: 2.8,
            max_speed: 4.2,
            average_heartrate: Some(145.0),
            max_heartrate: Some(176.0),
            average_cadence: Some(170.0),
            average_watts: None,
            calories: None,
            tag,
            summary_polyline: None,
            workout_type: None,
            streams_fetched_at: None,
            created_at: chrono::Utc::now(),
        }
    }
}
