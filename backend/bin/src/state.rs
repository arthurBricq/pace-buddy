use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::helpers::mas_estimator::{list_race_activities, LastRaceEstimator, MasEstimator};
use auth::JwtService;
use domain::DomainError;
use llm::open_router::OpenRouterClient;
use storage::{SqliteStorage, Storage};
use strava_client::StravaClient;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum ActivitiesSyncStatus {
    Running,
    Finished,
    Failed(String),
}

pub struct AppState {
    pub storage: Arc<SqliteStorage>,
    pub strava_client: Arc<StravaClient>,
    pub jwt: Arc<JwtService>,
    pub frontend_url: String,
    pub llm_client: Option<Arc<OpenRouterClient>>,
    pub strava_webhook_verify_token: Option<String>,
    pub admin_strava_athlete_id: Option<i64>,
    pub quota_markup_ratio: f64,
    pub mas_estimator: Arc<LastRaceEstimator>,
    pub syncing_activity_users: Arc<Mutex<HashSet<Uuid>>>,
    pub activity_sync_statuses: Arc<Mutex<HashMap<Uuid, ActivitiesSyncStatus>>>,
}

impl AppState {
    /// Computes the cost in user quotas
    pub(crate) fn cost_to_user_quota(&self, real: f64) -> f64 {
        real * self.quota_markup_ratio
    }

    /// Recompute MAS from current race-tagged activities and persist it.
    /// Returns the new MAS value when one can be estimated.
    pub async fn recompute_user_mas_from_races(
        &self,
        user_id: Uuid,
    ) -> Result<Option<f64>, DomainError> {
        let races = list_race_activities(self.storage.as_ref(), user_id).await?;
        let mas_mps = MasEstimator::estimate(self.mas_estimator.as_ref(), &races);
        if let Some(mas_mps) = mas_mps {
            self.storage.update_user_mas(user_id, Some(mas_mps)).await?;
        }
        Ok(mas_mps)
    }

    pub async fn try_begin_activities_sync(&self, user_id: Uuid) -> bool {
        let mut syncing = self.syncing_activity_users.lock().await;
        if syncing.contains(&user_id) {
            return false;
        }
        syncing.insert(user_id);
        drop(syncing);

        let mut statuses = self.activity_sync_statuses.lock().await;
        statuses.insert(user_id, ActivitiesSyncStatus::Running);
        true
    }

    pub async fn mark_activities_sync_finished(&self, user_id: Uuid) {
        let mut syncing = self.syncing_activity_users.lock().await;
        syncing.remove(&user_id);
        drop(syncing);

        let mut statuses = self.activity_sync_statuses.lock().await;
        statuses.insert(user_id, ActivitiesSyncStatus::Finished);
    }

    pub async fn mark_activities_sync_failed(&self, user_id: Uuid, error: String) {
        let mut syncing = self.syncing_activity_users.lock().await;
        syncing.remove(&user_id);
        drop(syncing);

        let mut statuses = self.activity_sync_statuses.lock().await;
        statuses.insert(user_id, ActivitiesSyncStatus::Failed(error));
    }

    pub async fn get_activities_sync_status(&self, user_id: Uuid) -> (String, Option<String>) {
        {
            let syncing = self.syncing_activity_users.lock().await;
            if syncing.contains(&user_id) {
                return ("running".to_string(), None);
            }
        }

        let statuses = self.activity_sync_statuses.lock().await;
        match statuses.get(&user_id) {
            Some(ActivitiesSyncStatus::Running) => ("running".to_string(), None),
            Some(ActivitiesSyncStatus::Finished) => ("finished".to_string(), None),
            Some(ActivitiesSyncStatus::Failed(err)) => ("failed".to_string(), Some(err.clone())),
            None => ("idle".to_string(), None),
        }
    }
}
