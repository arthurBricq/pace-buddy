use crate::helpers::strava_token_helper::get_valid_access_token;
use crate::state::AppState;
use domain::{Activity, DomainError};
use intervals::types::{IntervalConfig, IntervalResult};
use intervals::{
    parse_intervals_with_algorithm, AutoSpeedSegmentationAlgorithm, ManualLapIntervalAlgorithm,
};
use storage::Storage;
use strava_client::{strava_laps_to_domain, strava_streams_to_domain};

#[derive(Clone, Copy, Debug)]
pub enum IntervalAlgorithmSelection {
    SpeedBased,
    ManualLaps,
}

impl Default for IntervalAlgorithmSelection {
    fn default() -> Self {
        Self::SpeedBased
    }
}

impl IntervalAlgorithmSelection {
    pub fn from_str(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "speed_based" | "auto_speed" => Ok(Self::SpeedBased),
            "manual_laps" | "manual_lap" => Ok(Self::ManualLaps),
            other => Err(DomainError::BadRequest(format!(
                "Unknown interval algorithm '{other}'. Supported: speed_based, manual_laps"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SpeedBased => "speed_based",
            Self::ManualLaps => "manual_laps",
        }
    }
}

pub struct IntervalResolution {
    pub algorithm: IntervalAlgorithmSelection,
    pub result: IntervalResult,
}

impl AppState {
    pub async fn resolve_intervals(
        &self,
        activity: &Activity,
        requested_algorithm: Option<IntervalAlgorithmSelection>,
        mas_kmh: Option<f64>,
    ) -> Result<IntervalResolution, DomainError> {
        let cached = self.storage.get_interval_result(activity.id).await?;
        let selected_algorithm = if let Some(requested) = requested_algorithm {
            if let Some((cached_algorithm, cached_result_json)) = cached.as_ref() {
                if cached_algorithm == requested.as_str() {
                    match serde_json::from_str::<IntervalResult>(cached_result_json) {
                        Ok(cached_result) => {
                            return Ok(IntervalResolution {
                                algorithm: requested,
                                result: cached_result,
                            });
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to deserialize cached intervals for activity {}: {}. Recomputing.",
                                activity.id,
                                e
                            );
                        }
                    }
                }
            }
            requested
        } else if let Some((cached_algorithm, cached_result_json)) = cached {
            match IntervalAlgorithmSelection::from_str(&cached_algorithm) {
                Ok(cached_algorithm_selection) => {
                    match serde_json::from_str::<IntervalResult>(&cached_result_json) {
                        Ok(cached_result) => {
                            return Ok(IntervalResolution {
                                algorithm: cached_algorithm_selection,
                                result: cached_result,
                            });
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to deserialize cached intervals for activity {}: {}. Recomputing.",
                                activity.id,
                                e
                            );
                        }
                    }
                }
                Err(_) => {
                    log::warn!(
                        "Found unknown cached interval algorithm '{}' for activity {}. Recomputing with default.",
                        cached_algorithm,
                        activity.id
                    );
                }
            }
            IntervalAlgorithmSelection::default()
        } else {
            IntervalAlgorithmSelection::default()
        };

        let result = self
            .compute_intervals(activity, selected_algorithm, mas_kmh)
            .await?;

        match serde_json::to_string(&result) {
            Ok(result_json) => {
                if let Err(e) = self
                    .storage
                    .store_interval_result(activity.id, selected_algorithm.as_str(), &result_json)
                    .await
                {
                    log::error!(
                        "Failed to persist parsed intervals for activity {}: {}",
                        activity.id,
                        e
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to serialize parsed intervals for activity {}: {}",
                    activity.id,
                    e
                );
            }
        }

        Ok(IntervalResolution {
            algorithm: selected_algorithm,
            result,
        })
    }

    async fn compute_intervals(
        &self,
        activity: &Activity,
        algorithm: IntervalAlgorithmSelection,
        mas_kmh: Option<f64>,
    ) -> Result<IntervalResult, DomainError> {
        let config = IntervalConfig::default();
        let result = match algorithm {
            IntervalAlgorithmSelection::SpeedBased => {
                let mut streams = self
                    .storage
                    .get_streams(activity.id)
                    .await
                    .unwrap_or_default();
                if streams.is_empty() {
                    let access_token = get_valid_access_token(
                        &self.storage,
                        &self.strava_client,
                        activity.user_id,
                    )
                    .await?;
                    let strava_streams = self
                        .strava_client
                        .get_activity_streams(&access_token, activity.strava_id)
                        .await?;
                    streams = strava_streams_to_domain(strava_streams, activity.id);
                }
                let parser = AutoSpeedSegmentationAlgorithm;
                parse_intervals_with_algorithm(&parser, &streams, &config, mas_kmh)
            }
            IntervalAlgorithmSelection::ManualLaps => {
                let mut laps = self.storage.get_laps(activity.id).await.unwrap_or_default();
                if laps.is_empty() {
                    let access_token = get_valid_access_token(
                        &self.storage,
                        &self.strava_client,
                        activity.user_id,
                    )
                    .await?;
                    let strava_laps = self
                        .strava_client
                        .get_activity_laps(&access_token, activity.strava_id)
                        .await?;
                    laps = strava_laps_to_domain(strava_laps, activity.id);
                    if !laps.is_empty() {
                        self.storage.store_laps(&laps).await?;
                    }
                }
                let parser = ManualLapIntervalAlgorithm::new(&laps);
                parse_intervals_with_algorithm(&parser, &[], &config, mas_kmh)
            }
        };

        result.map_err(|e| DomainError::Internal(format!("Interval parsing failed: {e}")))
    }
}
