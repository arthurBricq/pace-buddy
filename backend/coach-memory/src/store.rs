use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    Activity, AthleteProfile, DomainError, IdentityProfile, RunningCoachMemory,
    RunningCoachMessage, RunningCoachSettings, RunningCoachState, TrainingSession, User,
};
use uuid::Uuid;

#[async_trait]
pub trait CoachMemoryDataStore: Send + Sync {
    async fn get_or_create_running_coach_settings(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachSettings, DomainError>;
    async fn get_or_create_running_coach_memory(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachMemory, DomainError>;
    async fn get_or_create_running_coach_state(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachState, DomainError>;
    async fn upsert_running_coach_state(
        &self,
        state: &RunningCoachState,
    ) -> Result<(), DomainError>;
    async fn upsert_running_coach_memory(
        &self,
        memory: &RunningCoachMemory,
    ) -> Result<(), DomainError>;
    async fn store_running_coach_message(
        &self,
        msg: &RunningCoachMessage,
    ) -> Result<(), DomainError>;
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, DomainError>;
    async fn get_identity_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<IdentityProfile>, DomainError>;
    async fn get_athlete_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AthleteProfile>, DomainError>;
    async fn get_activities_in_range(
        &self,
        user_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Activity>, DomainError>;
    async fn get_activities(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Activity>, DomainError>;
    async fn list_running_coach_messages(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RunningCoachMessage>, DomainError>;
    /// Return all `TrainingSession` rows for the user, any status. The coach
    /// context filters down to suggested + planned in Rust; we don't push the
    /// filter into storage because (a) the typical row count is small, and
    /// (b) future context features may want skipped/done too.
    async fn list_training_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TrainingSession>, DomainError>;
}
