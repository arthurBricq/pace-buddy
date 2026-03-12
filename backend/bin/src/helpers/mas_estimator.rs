use chrono::{DateTime, Utc};
use domain::{Activity, ActivityTag, DomainError};
use serde::Serialize;
use storage::Storage;
use uuid::Uuid;

const MAX_MAS_RACE_DISTANCE_M: f64 = 50_000.0;

pub trait MasEstimator: Send + Sync {
    fn estimate(&self, races: &[Activity]) -> Option<f64>;
}

#[derive(Default)]
pub struct LastRaceEstimator;

#[derive(Debug, Clone, Serialize)]
pub struct RaceMasEstimate {
    pub date: DateTime<Utc>,
    pub mas_kmh: f64,
    pub activity_id: Uuid,
    pub activity_name: String,
    pub distance_m: f64,
    pub time_s: i32,
}

impl LastRaceEstimator {
    fn is_eligible_race(race: &Activity) -> bool {
        race.sport_type.eq_ignore_ascii_case("Run")
            && race.distance > 0.0
            && race.distance <= MAX_MAS_RACE_DISTANCE_M
            && race.moving_time > 0
    }

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
        if !Self::is_eligible_race(race) {
            return None;
        }

        let avg_speed_mps = race.distance / f64::from(race.moving_time);
        let p = Self::p_value(race.distance);
        Some((avg_speed_mps / p) * 3.6)
    }
}

impl MasEstimator for LastRaceEstimator {
    fn estimate(&self, races: &[Activity]) -> Option<f64> {
        races
            .iter()
            .filter(|a| a.tag == ActivityTag::Race)
            .filter_map(|race| Self::estimate_for_race(race).map(|mas_kmh| (race, mas_kmh)))
            .max_by(|(a, _), (b, _)| a.start_date.cmp(&b.start_date))
            .map(|(_, mas_kmh)| mas_kmh)
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
            LastRaceEstimator::estimate_for_race(race).map(|mas_kmh| RaceMasEstimate {
                date: race.start_date,
                mas_kmh,
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
    use chrono::Duration;
    use domain::ActivityTag;

    fn make_race(
        sport_type: &str,
        distance_m: f64,
        moving_time_s: i32,
        start_offset_days: i64,
    ) -> Activity {
        Activity {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            strava_id: 1,
            name: "Race".to_string(),
            sport_type: sport_type.to_string(),
            start_date: Utc::now() + Duration::days(start_offset_days),
            elapsed_time: moving_time_s,
            moving_time: moving_time_s,
            distance: distance_m,
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
        }
    }

    #[test]
    fn estimate_for_10k_is_consistent() {
        // 10k in 40min => avg 15.0 km/h, p=0.90 => MAS ~= 16.67 km/h
        let mas = LastRaceEstimator::estimate_for_race(&make_race("Run", 10_000.0, 2400, 0))
            .unwrap();

        assert!((mas - 16.6667).abs() < 0.01);
    }

    #[test]
    fn estimate_for_trail_run_is_none() {
        let mas = LastRaceEstimator::estimate_for_race(&make_race("TrailRun", 10_000.0, 3000, 0));
        assert!(mas.is_none());
    }

    #[test]
    fn estimate_for_ultra_over_50k_is_none() {
        let mas = LastRaceEstimator::estimate_for_race(&make_race("Run", 52_000.0, 15_000, 0));
        assert!(mas.is_none());
    }

    #[test]
    fn estimator_uses_latest_eligible_race() {
        let races = vec![
            make_race("Run", 10_000.0, 2400, 0),
            make_race("TrailRun", 10_000.0, 2400, 1),
            make_race("Run", 55_000.0, 14_000, 2),
        ];

        let mas = LastRaceEstimator.estimate(&races).unwrap();
        assert!((mas - 16.6667).abs() < 0.01);
    }
}
