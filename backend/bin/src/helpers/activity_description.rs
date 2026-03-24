use std::str::FromStr;

use domain::{Activity, ActivityLap, ActivityStream, ActivityTag, DomainError, StreamType};
use storage::Storage;
use strava_client::{strava_laps_to_domain, strava_streams_to_domain};
use uuid::Uuid;

use crate::helpers::strava_token_helper::get_valid_access_token;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityDescriptionMode {
    Auto,
    Intervals,
    Race,
    LongRun,
    Normal,
}

impl ActivityDescriptionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Intervals => "intervals",
            Self::Race => "race",
            Self::LongRun => "long_run",
            Self::Normal => "normal",
        }
    }
}

impl FromStr for ActivityDescriptionMode {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "intervals" => Ok(Self::Intervals),
            "race" => Ok(Self::Race),
            "long_run" | "longrun" => Ok(Self::LongRun),
            "normal" => Ok(Self::Normal),
            other => Err(DomainError::BadRequest(format!(
                "Unknown activity description mode '{other}'. Supported: auto, intervals, race, long_run, normal"
            ))),
        }
    }
}

pub async fn build_activity_description(
    state: &AppState,
    user_id: Uuid,
    activity_id: Uuid,
    requested_mode: ActivityDescriptionMode,
) -> Result<String, DomainError> {
    let activity = state.storage.get_activity(activity_id, user_id).await?;
    let selected_mode = resolve_mode(&activity, requested_mode);

    let mut notes: Vec<String> = Vec::new();

    let laps = match ensure_laps(state, &activity).await {
        Ok(v) => v,
        Err(err) => {
            notes.push(format!("Laps unavailable ({err})"));
            Vec::new()
        }
    };

    let streams = match ensure_streams(state, &activity).await {
        Ok(v) => v,
        Err(err) => {
            notes.push(format!("Streams unavailable ({err})"));
            Vec::new()
        }
    };

    let mas_kmh = state.storage.get_user_by_id(user_id).await?.mas_current;

    let mode_analysis =
        build_mode_analysis(state, &activity, selected_mode, mas_kmh, &laps, &streams, &mut notes)
            .await;

    Ok(render_description(
        &activity,
        selected_mode,
        &mode_analysis,
        &notes,
    ))
}

fn resolve_mode(activity: &Activity, requested_mode: ActivityDescriptionMode) -> ActivityDescriptionMode {
    if requested_mode != ActivityDescriptionMode::Auto {
        return requested_mode;
    }

    match activity.tag {
        ActivityTag::Intervals => ActivityDescriptionMode::Intervals,
        ActivityTag::Race => ActivityDescriptionMode::Race,
        ActivityTag::LongRun => ActivityDescriptionMode::LongRun,
        ActivityTag::Normal => match activity.workout_type {
            Some(3) => ActivityDescriptionMode::Intervals,
            Some(1) => ActivityDescriptionMode::Race,
            Some(2) => ActivityDescriptionMode::LongRun,
            _ => ActivityDescriptionMode::Normal,
        },
    }
}

async fn ensure_streams(state: &AppState, activity: &Activity) -> Result<Vec<ActivityStream>, DomainError> {
    let mut streams = state.storage.get_streams(activity.id).await.unwrap_or_default();
    if !streams.is_empty() {
        return Ok(streams);
    }

    let access_token = get_valid_access_token(&state.storage, &state.strava_client, activity.user_id).await?;
    let strava_streams = state
        .strava_client
        .get_activity_streams(&access_token, activity.strava_id)
        .await?;
    streams = strava_streams_to_domain(strava_streams, activity.id);
    if !streams.is_empty() {
        state.storage.store_streams(&streams).await?;
    }
    state.storage.mark_streams_fetched(activity.id).await?;

    Ok(streams)
}

async fn ensure_laps(state: &AppState, activity: &Activity) -> Result<Vec<ActivityLap>, DomainError> {
    let mut laps = state.storage.get_laps(activity.id).await.unwrap_or_default();
    if !laps.is_empty() {
        return Ok(laps);
    }

    let access_token = get_valid_access_token(&state.storage, &state.strava_client, activity.user_id).await?;
    let strava_laps = state
        .strava_client
        .get_activity_laps(&access_token, activity.strava_id)
        .await?;
    laps = strava_laps_to_domain(strava_laps, activity.id);
    if !laps.is_empty() {
        state.storage.store_laps(&laps).await?;
    }

    Ok(laps)
}

async fn build_mode_analysis(
    state: &AppState,
    activity: &Activity,
    mode: ActivityDescriptionMode,
    mas_kmh: Option<f64>,
    laps: &[ActivityLap],
    streams: &[ActivityStream],
    notes: &mut Vec<String>,
) -> String {
    match mode {
        ActivityDescriptionMode::Intervals => {
            match state.resolve_intervals(activity, None, mas_kmh).await {
                Ok(resolution) => {
                    let result = resolution.result;
                    let mut out = String::new();
                    out.push_str(&format!(
                        "- Mode: intervals\n- Algorithm: {}\n- Interval workout detected: {}\n- Interval score: {:.2}\n",
                        resolution.algorithm.as_str(),
                        if result.is_interval_workout { "yes" } else { "no" },
                        result.interval_score
                    ));
                    if result.reps.is_empty() {
                        out.push_str("- Repetitions: none detected\n");
                        return out;
                    }

                    out.push_str(&format!("- Repetitions: {}\n", result.reps.len()));
                    out.push_str("\n### Repetition Details\n");
                    for rep in result.reps {
                        let mut line = format!(
                            "- Rep {}: {:.0}m in {:.0}s, pace {}",
                            rep.rep_index + 1,
                            rep.distance_m,
                            rep.duration_s,
                            pace_to_text(rep.avg_pace_s_per_km),
                        );
                        if let Some(pct) = rep.pct_mas {
                            line.push_str(&format!(", {:.0}% MAS", pct * 100.0));
                        }
                        if let Some(recovery_s) = rep.recovery_duration_s {
                            line.push_str(&format!(", {:.0}s recovery", recovery_s));
                        }
                        line.push('\n');
                        out.push_str(&line);
                    }
                    out
                }
                Err(err) => {
                    notes.push(format!("Interval parsing unavailable ({err})"));
                    "- Mode: intervals\n- Interval details unavailable; falling back to basic metrics.".to_string()
                }
            }
        }
        ActivityDescriptionMode::Race => {
            let mut out = String::new();
            out.push_str("- Mode: race\n");

            if let Some(split) = compute_half_split(streams) {
                out.push_str(&format!(
                    "- Estimated first half pace: {}\n- Estimated second half pace: {}\n- Split trend: {}\n",
                    pace_to_text(split.first_half_pace_s_per_km),
                    pace_to_text(split.second_half_pace_s_per_km),
                    split.trend
                ));
            } else {
                out.push_str("- Half-split estimation unavailable (missing time/distance streams)\n");
            }

            if let Some(lap_summary) = summarize_lap_pacing(laps) {
                out.push_str(&format!(
                    "- Lap pacing ({} laps): avg {}, fastest {}, slowest {}\n",
                    lap_summary.count,
                    pace_to_text(lap_summary.avg_pace_s_per_km),
                    pace_to_text(lap_summary.fastest_pace_s_per_km),
                    pace_to_text(lap_summary.slowest_pace_s_per_km)
                ));
            } else {
                out.push_str("- Lap pacing unavailable\n");
            }

            out
        }
        ActivityDescriptionMode::LongRun => {
            let mut out = String::new();
            out.push_str("- Mode: long_run\n");
            if let Some(split) = compute_half_split(streams) {
                out.push_str(&format!(
                    "- First half pace: {}\n- Second half pace: {}\n",
                    pace_to_text(split.first_half_pace_s_per_km),
                    pace_to_text(split.second_half_pace_s_per_km)
                ));
            }

            if let Some(drift) = compute_long_run_drift(streams) {
                out.push_str(&format!(
                    "- Pace drift (2nd vs 1st half): {:+.1}%\n- HR drift (2nd - 1st half): {:+.1} bpm\n",
                    drift.pace_drift_pct,
                    drift.hr_drift_bpm
                ));
            } else {
                out.push_str("- Drift metrics unavailable (missing heartrate/velocity streams)\n");
            }

            if let Some(lap_summary) = summarize_lap_pacing(laps) {
                out.push_str(&format!(
                    "- Lap consistency: avg {}, fastest {}, slowest {}\n",
                    pace_to_text(lap_summary.avg_pace_s_per_km),
                    pace_to_text(lap_summary.fastest_pace_s_per_km),
                    pace_to_text(lap_summary.slowest_pace_s_per_km)
                ));
            }

            out
        }
        ActivityDescriptionMode::Normal | ActivityDescriptionMode::Auto => {
            let mut out = String::new();
            out.push_str("- Mode: normal\n");
            if let Some(split) = compute_half_split(streams) {
                out.push_str(&format!(
                    "- First half pace: {}\n- Second half pace: {}\n",
                    pace_to_text(split.first_half_pace_s_per_km),
                    pace_to_text(split.second_half_pace_s_per_km)
                ));
            } else {
                out.push_str("- Split detail unavailable (missing time/distance streams)\n");
            }

            if let Some(lap_summary) = summarize_lap_pacing(laps) {
                out.push_str(&format!(
                    "- Key splits from laps ({}): avg {}, fastest {}, slowest {}\n",
                    lap_summary.count,
                    pace_to_text(lap_summary.avg_pace_s_per_km),
                    pace_to_text(lap_summary.fastest_pace_s_per_km),
                    pace_to_text(lap_summary.slowest_pace_s_per_km)
                ));
            }

            out
        }
    }
}

fn render_description(
    activity: &Activity,
    selected_mode: ActivityDescriptionMode,
    mode_analysis: &str,
    notes: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("# Session\n\n");
    out.push_str("## Identity\n");
    out.push_str(&format!("- Activity ID: {}\n", activity.id));
    out.push_str(&format!("- Strava ID: {}\n", activity.strava_id));
    out.push_str(&format!("- Name: {}\n", activity.name));
    out.push_str(&format!(
        "- Start date: {}\n",
        activity.start_date.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    out.push_str(&format!("- Sport: {}\n", activity.sport_type));
    out.push_str(&format!("- Tag: {}\n", activity.tag));
    out.push_str(&format!("- Selected mode: {}\n\n", selected_mode.as_str()));

    out.push_str("## Core Metrics\n");
    out.push_str(&format!("- Distance: {:.2} km\n", activity.distance / 1000.0));
    out.push_str(&format!("- Moving time: {}\n", format_duration(activity.moving_time)));
    out.push_str(&format!("- Elapsed time: {}\n", format_duration(activity.elapsed_time)));
    out.push_str(&format!(
        "- Average pace: {}\n",
        pace_to_text(activity_pace_s_per_km(activity))
    ));
    out.push_str(&format!(
        "- Average speed: {:.2} km/h\n",
        activity.average_speed * 3.6
    ));
    out.push_str(&format!("- Max speed: {:.2} km/h\n", activity.max_speed * 3.6));
    out.push_str(&format!(
        "- Elevation gain: {:.0} m\n",
        activity.total_elevation_gain
    ));
    if let Some(hr) = activity.average_heartrate {
        out.push_str(&format!("- Avg HR: {:.0} bpm\n", hr));
    }
    if let Some(max_hr) = activity.max_heartrate {
        out.push_str(&format!("- Max HR: {:.0} bpm\n", max_hr));
    }
    if let Some(cadence) = activity.average_cadence {
        out.push_str(&format!("- Avg cadence: {:.0} spm\n", cadence));
    }
    if let Some(watts) = activity.average_watts {
        out.push_str(&format!("- Avg power: {:.0} W\n", watts));
    }
    if let Some(calories) = activity.calories {
        out.push_str(&format!("- Calories: {:.0}\n", calories));
    }
    out.push('\n');

    out.push_str("## Mode Analysis\n");
    out.push_str(mode_analysis.trim());
    out.push_str("\n\n");

    out.push_str("## Notes/Data Quality\n");
    if notes.is_empty() {
        out.push_str("- No known data quality issue detected while building this description.\n");
    } else {
        for note in notes {
            out.push_str(&format!("- {note}\n"));
        }
    }

    out
}

fn activity_pace_s_per_km(activity: &Activity) -> f64 {
    if activity.distance > 0.0 {
        activity.moving_time as f64 / (activity.distance / 1000.0)
    } else {
        0.0
    }
}

fn format_duration(total_seconds: i32) -> String {
    let seconds = total_seconds.max(0);
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else {
        format!("{m}m{s:02}s")
    }
}

fn pace_to_text(seconds_per_km: f64) -> String {
    if !seconds_per_km.is_finite() || seconds_per_km <= 0.0 {
        return "N/A".to_string();
    }
    let minutes = (seconds_per_km / 60.0).floor() as i64;
    let seconds = (seconds_per_km.round() as i64) % 60;
    format!("{minutes}:{seconds:02}/km")
}

#[derive(Debug, Clone, Copy)]
struct HalfSplit {
    first_half_pace_s_per_km: f64,
    second_half_pace_s_per_km: f64,
    trend: &'static str,
}

fn compute_half_split(streams: &[ActivityStream]) -> Option<HalfSplit> {
    let distances = stream_numbers(streams, StreamType::Distance)?;
    let times = stream_numbers(streams, StreamType::Time)?;

    if distances.len() < 2 || distances.len() != times.len() {
        return None;
    }

    let start_distance = *distances.first()?;
    let end_distance = *distances.last()?;
    let start_time = *times.first()?;
    let end_time = *times.last()?;

    let total_distance = end_distance - start_distance;
    let total_time = end_time - start_time;
    if total_distance <= 0.0 || total_time <= 0.0 {
        return None;
    }

    let half_distance_abs = start_distance + total_distance / 2.0;
    let mut crossing_index: Option<usize> = None;
    for (idx, d) in distances.iter().enumerate().skip(1) {
        if *d >= half_distance_abs {
            crossing_index = Some(idx);
            break;
        }
    }
    let idx = crossing_index?;
    let d1 = distances[idx - 1];
    let d2 = distances[idx];
    let t1 = times[idx - 1];
    let t2 = times[idx];
    let interpolation = if (d2 - d1).abs() < f64::EPSILON {
        0.0
    } else {
        ((half_distance_abs - d1) / (d2 - d1)).clamp(0.0, 1.0)
    };
    let half_time_abs = t1 + (t2 - t1) * interpolation;

    let first_half_time = half_time_abs - start_time;
    let second_half_time = end_time - half_time_abs;
    let half_distance_km = (total_distance / 2.0) / 1000.0;
    if half_distance_km <= 0.0 {
        return None;
    }

    let first_pace = first_half_time / half_distance_km;
    let second_pace = second_half_time / half_distance_km;
    let diff = second_pace - first_pace;
    let trend = if diff < -3.0 {
        "negative split"
    } else if diff > 3.0 {
        "positive split"
    } else {
        "even split"
    };

    Some(HalfSplit {
        first_half_pace_s_per_km: first_pace,
        second_half_pace_s_per_km: second_pace,
        trend,
    })
}

#[derive(Debug, Clone, Copy)]
struct LongRunDrift {
    pace_drift_pct: f64,
    hr_drift_bpm: f64,
}

fn compute_long_run_drift(streams: &[ActivityStream]) -> Option<LongRunDrift> {
    let heartrate = stream_numbers(streams, StreamType::HeartRate)?;
    let velocity = stream_numbers(streams, StreamType::VelocitySmooth)?;
    let len = heartrate.len().min(velocity.len());
    if len < 10 {
        return None;
    }

    let half = len / 2;
    let (hr1, hr2) = (avg(&heartrate[..half])?, avg(&heartrate[half..len])?);
    let (v1, v2) = (avg(&velocity[..half])?, avg(&velocity[half..len])?);
    if v1 <= 0.0 || v2 <= 0.0 {
        return None;
    }

    let p1 = 1000.0 / v1;
    let p2 = 1000.0 / v2;
    let pace_drift_pct = ((p2 - p1) / p1) * 100.0;
    let hr_drift_bpm = hr2 - hr1;

    Some(LongRunDrift {
        pace_drift_pct,
        hr_drift_bpm,
    })
}

#[derive(Debug, Clone, Copy)]
struct LapPacingSummary {
    count: usize,
    avg_pace_s_per_km: f64,
    fastest_pace_s_per_km: f64,
    slowest_pace_s_per_km: f64,
}

fn summarize_lap_pacing(laps: &[ActivityLap]) -> Option<LapPacingSummary> {
    if laps.is_empty() {
        return None;
    }
    let mut paces: Vec<f64> = laps
        .iter()
        .filter_map(|lap| {
            if lap.distance > 0.0 && lap.moving_time > 0 {
                Some(lap.moving_time as f64 / (lap.distance / 1000.0))
            } else {
                None
            }
        })
        .collect();
    if paces.is_empty() {
        return None;
    }

    paces.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let avg = avg(&paces)?;
    Some(LapPacingSummary {
        count: paces.len(),
        avg_pace_s_per_km: avg,
        fastest_pace_s_per_km: paces[0],
        slowest_pace_s_per_km: *paces.last()?,
    })
}

fn stream_numbers(streams: &[ActivityStream], stream_type: StreamType) -> Option<Vec<f64>> {
    let stream = streams.iter().find(|s| s.stream_type == stream_type)?;
    let value = serde_json::from_str::<serde_json::Value>(&stream.data_json).ok()?;
    let items = value.as_array()?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        if let Some(v) = item.as_f64() {
            out.push(v);
            continue;
        }
        if let Some(v) = item.as_i64() {
            out.push(v as f64);
            continue;
        }
        if let Some(v) = item.as_u64() {
            out.push(v as f64);
            continue;
        }
        return None;
    }
    Some(out)
}

fn avg(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().sum();
    Some(sum / values.len() as f64)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use domain::{Activity, ActivityTag};

    use super::{compute_half_split, render_description, resolve_mode, ActivityDescriptionMode};

    fn sample_activity(tag: ActivityTag, workout_type: Option<i32>) -> Activity {
        Activity {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            strava_id: 42,
            name: "Morning Run".to_string(),
            sport_type: "Run".to_string(),
            start_date: Utc::now(),
            elapsed_time: 3600,
            moving_time: 3540,
            distance: 10000.0,
            total_elevation_gain: 120.0,
            average_speed: 2.82,
            max_speed: 4.8,
            average_heartrate: Some(152.0),
            max_heartrate: Some(177.0),
            average_cadence: Some(170.0),
            average_watts: Some(240.0),
            calories: Some(720.0),
            tag,
            summary_polyline: None,
            workout_type,
            streams_fetched_at: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn auto_mode_prefers_tag_then_workout_type() {
        let intervals = sample_activity(ActivityTag::Intervals, Some(0));
        assert_eq!(
            resolve_mode(&intervals, ActivityDescriptionMode::Auto),
            ActivityDescriptionMode::Intervals
        );

        let workout_based_race = sample_activity(ActivityTag::Normal, Some(1));
        assert_eq!(
            resolve_mode(&workout_based_race, ActivityDescriptionMode::Auto),
            ActivityDescriptionMode::Race
        );

        let normal = sample_activity(ActivityTag::Normal, None);
        assert_eq!(
            resolve_mode(&normal, ActivityDescriptionMode::Auto),
            ActivityDescriptionMode::Normal
        );
    }

    #[test]
    fn rendered_description_has_required_sections() {
        let activity = sample_activity(ActivityTag::Normal, None);
        let text = render_description(
            &activity,
            ActivityDescriptionMode::Normal,
            "- Mode: normal\n- Split detail unavailable",
            &[],
        );

        assert!(text.contains("# Session"));
        assert!(text.contains("## Identity"));
        assert!(text.contains("## Core Metrics"));
        assert!(text.contains("## Mode Analysis"));
        assert!(text.contains("## Notes/Data Quality"));
        assert!(text.contains("Selected mode: normal"));
    }

    #[test]
    fn half_split_detects_positive_split() {
        let activity_id = uuid::Uuid::new_v4();
        let streams = vec![
            domain::ActivityStream {
                activity_id,
                stream_type: domain::StreamType::Distance,
                data_json: "[0, 1000, 2000, 3000, 4000]".to_string(),
            },
            domain::ActivityStream {
                activity_id,
                stream_type: domain::StreamType::Time,
                data_json: "[0, 290, 600, 930, 1280]".to_string(),
            },
        ];

        let split = compute_half_split(&streams).expect("expected split");
        assert!(split.second_half_pace_s_per_km > split.first_half_pace_s_per_km);
        assert_eq!(split.trend, "positive split");
    }

    #[test]
    fn rendered_description_handles_missing_optional_metrics() {
        let mut activity = sample_activity(ActivityTag::Normal, None);
        activity.distance = 0.0;
        activity.average_heartrate = None;
        activity.max_heartrate = None;
        activity.average_cadence = None;
        activity.average_watts = None;
        activity.calories = None;

        let text = render_description(
            &activity,
            ActivityDescriptionMode::Normal,
            "- Mode: normal\n- Split detail unavailable",
            &[],
        );

        assert!(text.contains("Average pace: N/A"));
        assert!(text.contains("## Core Metrics"));
        assert!(!text.contains("Avg HR:"));
        assert!(!text.contains("Avg cadence:"));
        assert!(!text.contains("Avg power:"));
    }
}
