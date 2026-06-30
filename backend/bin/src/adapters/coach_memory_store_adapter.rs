use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use coach_memory::CoachMemoryDataStore;
use domain::{
    Activity, AthleteProfile, DomainError, IdentityProfile, RunningCoachMemory,
    RunningCoachMessage, RunningCoachSettings, RunningCoachState, TrainingSession, User,
};
use storage::{SqliteStorage, Storage};
use uuid::Uuid;

pub struct CoachMemoryStorageAdapter {
    storage: Arc<SqliteStorage>,
}

impl CoachMemoryStorageAdapter {
    pub fn new(storage: Arc<SqliteStorage>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl CoachMemoryDataStore for CoachMemoryStorageAdapter {
    async fn get_or_create_running_coach_settings(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachSettings, DomainError> {
        self.storage
            .get_or_create_running_coach_settings(user_id)
            .await
    }

    async fn get_or_create_running_coach_memory(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachMemory, DomainError> {
        self.storage
            .get_or_create_running_coach_memory(user_id)
            .await
    }

    async fn get_or_create_running_coach_state(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachState, DomainError> {
        self.storage
            .get_or_create_running_coach_state(user_id)
            .await
    }

    async fn upsert_running_coach_state(
        &self,
        state: &RunningCoachState,
    ) -> Result<(), DomainError> {
        self.storage.upsert_running_coach_state(state).await
    }

    async fn upsert_running_coach_memory(
        &self,
        memory: &RunningCoachMemory,
    ) -> Result<(), DomainError> {
        self.storage.upsert_running_coach_memory(memory).await
    }

    async fn store_running_coach_message(
        &self,
        msg: &RunningCoachMessage,
    ) -> Result<(), DomainError> {
        self.storage.store_running_coach_message(msg).await
    }

    async fn delete_running_coach_message(&self, message_id: Uuid) -> Result<(), DomainError> {
        self.storage.delete_running_coach_message(message_id).await
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, DomainError> {
        self.storage.get_user_by_id(user_id).await
    }

    async fn get_identity_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<IdentityProfile>, DomainError> {
        self.storage.get_identity_profile(user_id).await
    }

    async fn get_athlete_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AthleteProfile>, DomainError> {
        self.storage.get_athlete_profile(user_id).await
    }

    async fn get_activities_in_range(
        &self,
        user_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Activity>, DomainError> {
        self.storage
            .get_activities_in_range(user_id, from, to)
            .await
    }

    async fn get_activities(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Activity>, DomainError> {
        self.storage.get_activities(user_id, limit, offset).await
    }

    async fn list_running_coach_messages(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RunningCoachMessage>, DomainError> {
        self.storage
            .list_running_coach_messages(user_id, limit)
            .await
    }

    async fn list_training_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TrainingSession>, DomainError> {
        self.storage.list_training_sessions(user_id, None).await
    }
}
