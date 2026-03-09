use chrono::{DateTime, Utc};
use domain::{Activity, ActivityTag, DomainError};
use serde::Serialize;
use storage::Storage;
use uuid::Uuid;

pub trait MasEstimator: Send + Sync {
    fn estimate(&self, races: &[Activity]) -> Option<f64>;
}

#[derive(Default)]
pub struct LastRaceEstimator;

#[derive(Debug, Clone, Serialize)]
pub struct RaceMasEstimate {
    pub date: DateTime<Utc>,
    pub mas_ms: f64,
    pub mas_kmh: f64,
    pub activity_id: Uuid,
    pub activity_name: String,
    pub distance_m: f64,
    pub time_s: i32,
}

impl LastRaceEstimator {
    fn p_value(distance_m: f64) -> f64 {
        if (1500.0..=3000.0).contains(&distance_m) {
            0.98
        } else if (distance_m - 5000.0).abs() < 100.0 {
            0.92
        } else if (distance_m - 10000.0).abs() < 100.0 {
            0.90
        } else if (distance_m - 21100.0).abs() < 500.0 {
            0.85
        } else if (distance_m - 42200.0).abs() < 1000.0 {
            0.80
        } else if distance_m < 5000.0 {
            0.95
        } else if distance_m < 10000.0 {
            0.91
        } else if distance_m < 21100.0 {
            0.875
        } else if distance_m < 42200.0 {
            0.825
        } else {
            0.75
        }
    }

    pub fn estimate_for_race(race: &Activity) -> Option<f64> {
        if race.distance <= 0.0 || race.moving_time <= 0 {
            return None;
        }

        let avg_speed_mps = race.distance / f64::from(race.moving_time);
        let p = Self::p_value(race.distance);
        Some(avg_speed_mps / p)
    }
}

impl MasEstimator for LastRaceEstimator {
    fn estimate(&self, races: &[Activity]) -> Option<f64> {
        races
            .iter()
            .filter(|a| a.tag == ActivityTag::Race)
            .max_by_key(|a| a.start_date)
            .and_then(Self::estimate_for_race)
    }
}

pub async fn list_race_activities(
    storage: &impl Storage,
    user_id: Uuid,
) -> Result<Vec<Activity>, DomainError> {
    let mut races = Vec::new();
    let mut offset = 0;
    let limit = 200;

    loop {
        let page = storage.get_activities(user_id, limit, offset).await?;
        let page_len = page.len();
        if page_len == 0 {
            break;
        }

        races.extend(page.into_iter().filter(|a| a.tag == ActivityTag::Race));
        if page_len < limit as usize {
            break;
        }
        offset += limit;
    }

    Ok(races)
}

pub fn build_race_mas_estimates(races: &[Activity]) -> Vec<RaceMasEstimate> {
    let mut out: Vec<RaceMasEstimate> = races
        .iter()
        .filter_map(|race| {
            LastRaceEstimator::estimate_for_race(race).map(|mas_ms| RaceMasEstimate {
                date: race.start_date,
                mas_ms,
                mas_kmh: mas_ms * 3.6,
                activity_id: race.id,
                activity_name: race.name.clone(),
                distance_m: race.distance,
                time_s: race.moving_time,
            })
        })
        .collect();

    out.sort_by_key(|item| item.date);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_for_10k_is_consistent() {
        // 10k in 40min => avg 4.1667 m/s, p=0.90 => MAS ~= 4.63 m/s
        let mas = LastRaceEstimator::estimate_for_race(&Activity {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            strava_id: 1,
            name: "Race".to_string(),
            sport_type: "Run".to_string(),
            start_date: Utc::now(),
            elapsed_time: 2400,
            moving_time: 2400,
            distance: 10_000.0,
            total_elevation_gain: 0.0,
            average_speed: 0.0,
            max_speed: 0.0,
            average_heartrate: None,
            max_heartrate: None,
            average_cadence: None,
            average_watts: None,
            calories: None,
            tag: ActivityTag::Race,
            summary_polyline: None,
            workout_type: None,
            streams_fetched_at: None,
            created_at: Utc::now(),
        })
        .unwrap();

        assert!((mas - 4.6296).abs() < 0.01);
    }
}
