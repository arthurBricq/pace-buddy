use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use auth::{JwtService, WebAuthnService};
use llm::open_router::OpenRouterClient;
use storage::SqliteStorage;
use tokio::sync::Mutex;
use strava_client::StravaClient;
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
    pub webauthn: Arc<WebAuthnService>,
    pub jwt: Arc<JwtService>,
    pub frontend_url: String,
    pub llm_client: Option<Arc<OpenRouterClient>>,
    pub strava_webhook_verify_token: Option<String>,
    pub admin_strava_athlete_id: Option<i64>,
    pub quota_markup_ratio: f64,
    pub syncing_activity_users: Arc<Mutex<HashSet<Uuid>>>,
    pub activity_sync_statuses: Arc<Mutex<HashMap<Uuid, ActivitiesSyncStatus>>>,
}

impl AppState {
    /// Computes the cost in user quotas
    pub(crate) fn cost_to_user_quota(&self, real: f64) -> f64 {
        real * self.quota_markup_ratio
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
