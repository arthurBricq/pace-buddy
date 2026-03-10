use chrono::{DateTime, Utc};
use domain::{Activity, ActivityLap, ActivityStream, ActivityTag};
use uuid::Uuid;

use crate::types::{StravaActivity, StravaLap, StravaStream};

pub fn strava_activity_to_domain(sa: &StravaActivity, user_id: Uuid) -> Activity {
    let start_date = DateTime::parse_from_rfc3339(&sa.start_date)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let sport_type = sa
        .sport_type
        .clone()
        .or_else(|| sa.activity_type.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    Activity {
        id: Uuid::new_v4(),
        user_id,
        strava_id: sa.id,
        name: sa.name.clone(),
        sport_type,
        start_date,
        elapsed_time: sa.elapsed_time,
        moving_time: sa.moving_time,
        distance: sa.distance,
        total_elevation_gain: sa.total_elevation_gain,
        average_speed: sa.average_speed,
        max_speed: sa.max_speed,
        average_heartrate: sa.average_heartrate,
        max_heartrate: sa.max_heartrate,
        average_cadence: sa.average_cadence,
        average_watts: sa.average_watts,
        calories: sa.calories,
        tag: ActivityTag::from_strava_workout_type(sa.workout_type),
        summary_polyline: None, // GPS data: never persisted, fetched on demand
        workout_type: sa.workout_type,
        streams_fetched_at: None,
        created_at: Utc::now(),
    }
}

pub fn strava_streams_to_domain(
    streams: Vec<StravaStream>,
    activity_id: Uuid,
) -> Vec<ActivityStream> {
    streams
        .into_iter()
        .filter_map(|s| {
            s.parsed_type().map(|st| ActivityStream {
                activity_id,
                stream_type: st,
                data_json: s.data.to_string(),
            })
        })
        .collect()
}

pub fn strava_laps_to_domain(laps: Vec<StravaLap>, activity_id: Uuid) -> Vec<ActivityLap> {
    laps.into_iter()
        .enumerate()
        .map(|(i, lap)| {
            let lap_index = lap.lap_index.or(lap.split).unwrap_or(i as i32 + 1);
            let start_date = DateTime::parse_from_rfc3339(&lap.start_date)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            ActivityLap {
                activity_id,
                lap_index,
                name: lap.name.unwrap_or_else(|| format!("Lap {lap_index}")),
                start_date,
                elapsed_time: lap.elapsed_time,
                moving_time: lap.moving_time,
                distance: lap.distance,
                average_speed: lap.average_speed,
                max_speed: lap.max_speed,
                total_elevation_gain: lap.total_elevation_gain,
                average_heartrate: lap.average_heartrate,
                max_heartrate: lap.max_heartrate,
            }
        })
        .collect()
}
