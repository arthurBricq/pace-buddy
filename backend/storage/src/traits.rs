use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{Activity, ActivityStream, ActivityTag, DomainError, RunningStats, StravaToken, Training, TrainingInsight, User};
use uuid::Uuid;

#[async_trait]
pub trait Storage: Send + Sync {
    // Users
    async fn create_user(&self, user: &User) -> Result<(), DomainError>;
    async fn get_user_by_id(&self, id: Uuid) -> Result<User, DomainError>;
    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError>;
    async fn list_users(&self) -> Result<Vec<User>, DomainError>;
    async fn update_user_mas(&self, user_id: Uuid, mas_mps: Option<f64>) -> Result<(), DomainError>;

    // Passkeys - store as JSON text
    async fn store_passkey(&self, user_id: Uuid, passkey_json: &str) -> Result<(), DomainError>;
    async fn get_passkeys_for_user(&self, user_id: Uuid) -> Result<Vec<String>, DomainError>;

    // Strava tokens
    async fn upsert_strava_token(&self, token: &StravaToken) -> Result<(), DomainError>;
    async fn get_strava_token(&self, user_id: Uuid) -> Result<StravaToken, DomainError>;

    // Activities
    async fn upsert_activities(&self, activities: &[Activity]) -> Result<(), DomainError>;
    async fn get_activities(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Activity>, DomainError>;
    async fn get_activity(&self, id: Uuid, user_id: Uuid) -> Result<Activity, DomainError>;
    async fn get_latest_activity_start(
        &self,
        user_id: Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, DomainError>;
    async fn update_activity_tag(
        &self,
        id: Uuid,
        user_id: Uuid,
        tag: ActivityTag,
    ) -> Result<(), DomainError>;
    async fn mark_streams_fetched(&self, activity_id: Uuid) -> Result<(), DomainError>;

    // Streams
    async fn store_streams(&self, streams: &[ActivityStream]) -> Result<(), DomainError>;
    async fn get_streams(&self, activity_id: Uuid) -> Result<Vec<ActivityStream>, DomainError>;

    // Trainings
    async fn create_training(&self, training: &Training) -> Result<(), DomainError>;
    async fn get_training(&self, id: Uuid, user_id: Uuid) -> Result<Training, DomainError>;
    async fn list_trainings(&self, user_id: Uuid) -> Result<Vec<Training>, DomainError>;
    async fn update_training(
        &self,
        id: Uuid,
        user_id: Uuid,
        name: String,
        description: Option<String>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        race_goal: Option<String>,
    ) -> Result<(), DomainError>;
    async fn delete_training(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    async fn add_activity_to_training(
        &self,
        training_id: Uuid,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError>;
    async fn remove_activity_from_training(
        &self,
        training_id: Uuid,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError>;
    async fn get_training_activities(
        &self,
        training_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Activity>, DomainError>;
    async fn get_activity_trainings(
        &self,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Training>, DomainError>;

    async fn get_activities_in_range(
        &self,
        user_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Activity>, DomainError>;

    // Training insights
    async fn store_training_insight(&self, insight: &TrainingInsight) -> Result<(), DomainError>;
    async fn get_training_insights(
        &self,
        training_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TrainingInsight>, DomainError>;

    // Stats
    async fn get_running_stats(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        include_interval_count: bool,
    ) -> Result<RunningStats, DomainError>;
}
