use async_trait::async_trait;
use domain::{Activity, ActivityStream, ActivityTag, DomainError, StravaToken, User};
use uuid::Uuid;

#[async_trait]
pub trait Storage: Send + Sync {
    // Users
    async fn create_user(&self, user: &User) -> Result<(), DomainError>;
    async fn get_user_by_id(&self, id: Uuid) -> Result<User, DomainError>;
    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError>;

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
    async fn update_activity_tag(
        &self,
        id: Uuid,
        user_id: Uuid,
        tag: ActivityTag,
    ) -> Result<(), DomainError>;
    async fn mark_streams_loaded(&self, activity_id: Uuid) -> Result<(), DomainError>;

    // Streams
    async fn store_streams(&self, streams: &[ActivityStream]) -> Result<(), DomainError>;
    async fn get_streams(&self, activity_id: Uuid) -> Result<Vec<ActivityStream>, DomainError>;
}
