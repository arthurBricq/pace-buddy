use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    Activity, ActivityLap, ActivityStream, ActivityTag, AiChat, AiChatMessage, DomainError,
    ModelCostTier, QuotaRequest, QuotaRequestStatus, RunningStats, StravaToken, Training,
    TrainingInsight, User,
};
use uuid::Uuid;

#[async_trait]
pub trait Storage: Send + Sync {
    // Users
    async fn create_user(&self, user: &User) -> Result<(), DomainError>;
    async fn get_user_by_id(&self, id: Uuid) -> Result<User, DomainError>;
    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError>;
    async fn get_user_by_email(&self, email: &str) -> Result<User, DomainError>;
    async fn list_users(&self) -> Result<Vec<User>, DomainError>;
    async fn update_user_mas(&self, user_id: Uuid, mas_kmh: Option<f64>)
        -> Result<(), DomainError>;
    async fn upsert_model_cost_tiers(&self, tiers: &[ModelCostTier]) -> Result<(), DomainError>;
    async fn list_model_cost_tiers(&self) -> Result<Vec<ModelCostTier>, DomainError>;

    // Strava tokens
    async fn upsert_strava_token(&self, token: &StravaToken) -> Result<(), DomainError>;
    async fn get_strava_token(&self, user_id: Uuid) -> Result<StravaToken, DomainError>;
    async fn get_strava_token_by_athlete_id(
        &self,
        athlete_id: i64,
    ) -> Result<StravaToken, DomainError>;
    async fn delete_strava_data(&self, user_id: Uuid) -> Result<(), DomainError>;
    async fn delete_activity_by_strava_id(
        &self,
        strava_id: i64,
        user_id: Uuid,
    ) -> Result<(), DomainError>;

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
    async fn store_laps(&self, laps: &[ActivityLap]) -> Result<(), DomainError>;
    async fn get_laps(&self, activity_id: Uuid) -> Result<Vec<ActivityLap>, DomainError>;
    async fn store_interval_result(
        &self,
        activity_id: Uuid,
        algorithm: &str,
        result_json: &str,
    ) -> Result<(), DomainError>;
    async fn get_interval_result(
        &self,
        activity_id: Uuid,
    ) -> Result<Option<(String, String)>, DomainError>;

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
        race_distance: Option<String>,
    ) -> Result<(), DomainError>;
    async fn delete_training(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
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

    // AI Chats
    async fn create_ai_chat(&self, chat: &AiChat) -> Result<(), DomainError>;
    async fn get_ai_chat(&self, id: Uuid, user_id: Uuid) -> Result<AiChat, DomainError>;
    async fn get_ai_chat_by_source_insight(
        &self,
        user_id: Uuid,
        insight_id: Uuid,
    ) -> Result<Option<AiChat>, DomainError>;
    async fn list_ai_chats(&self, user_id: Uuid) -> Result<Vec<AiChat>, DomainError>;
    async fn update_ai_chat_title(
        &self,
        id: Uuid,
        user_id: Uuid,
        title: &str,
    ) -> Result<(), DomainError>;
    async fn delete_ai_chat(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    async fn touch_ai_chat(&self, id: Uuid) -> Result<(), DomainError>;

    // AI Chat Messages
    async fn store_ai_chat_message(&self, msg: &AiChatMessage) -> Result<(), DomainError>;
    async fn get_ai_chat_messages(&self, chat_id: Uuid) -> Result<Vec<AiChatMessage>, DomainError>;

    // Insight lookup
    async fn get_training_insight_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<TrainingInsight, DomainError>;

    // Quota
    async fn get_user_quota(&self, user_id: Uuid) -> Result<f64, DomainError>;
    async fn deduct_quota(&self, user_id: Uuid, amount: f64) -> Result<(), DomainError>;
    async fn add_quota(&self, user_id: Uuid, amount: f64) -> Result<(), DomainError>;

    // Quota requests
    async fn create_quota_request(&self, req: &QuotaRequest) -> Result<(), DomainError>;
    async fn get_quota_request(&self, id: Uuid) -> Result<QuotaRequest, DomainError>;
    async fn get_pending_quota_requests(&self) -> Result<Vec<QuotaRequest>, DomainError>;
    async fn get_user_quota_requests(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<QuotaRequest>, DomainError>;
    async fn resolve_quota_request(
        &self,
        id: Uuid,
        status: QuotaRequestStatus,
        granted_amount_usd: Option<f64>,
    ) -> Result<(), DomainError>;

    // Stats
    async fn get_running_stats(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        include_interval_count: bool,
    ) -> Result<RunningStats, DomainError>;
}
