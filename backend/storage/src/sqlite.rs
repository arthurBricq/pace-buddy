use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::invite_code::InviteCode;
use domain::{
    Activity, ActivityLap, ActivityStream, ActivityTag, AiChat, AiChatMessage, AthleteProfile,
    DomainError, IdentityProfile, ModelCostCategory, ModelCostTier, QuotaRequest,
    QuotaRequestStatus, RunningCoachMemory, RunningCoachMemoryData, RunningCoachMessage,
    RunningCoachSettings, RunningCoachState, RunningStats, StravaToken, Training, TrainingInsight,
    User,
};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Error as SqlxError, Row};
use uuid::Uuid;

use crate::traits::Storage;

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> Result<Self, DomainError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to connect to database: {e}")))?;

        Self::initialize_schema(&pool).await?;

        Ok(Self { pool })
    }

    async fn initialize_schema(pool: &SqlitePool) -> Result<(), DomainError> {
        // Create tables directly (no migrations for now)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT UNIQUE NOT NULL,
                display_name TEXT NOT NULL,
                email TEXT UNIQUE,
                created_at TEXT NOT NULL,
                mas_current REAL,
                quota_balance_usd REAL NOT NULL DEFAULT 0.0
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create users table: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS identity_profiles (
                user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT,
                age INTEGER,
                email TEXT,
                gender TEXT,
                height_cm REAL,
                weight_kg REAL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create identity_profiles table: {e}"))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS athlete_profiles (
                user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                goal_description TEXT,
                goal_date TEXT,
                goal_distance_km REAL,
                goal_target_time_seconds INTEGER,
                goal_sport_type TEXT,
                goal_elevation_gain_m REAL,
                additional_info TEXT,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create athlete_profiles table: {e}"))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS strava_tokens (
                user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id),
                strava_athlete_id INTEGER NOT NULL,
                access_token TEXT NOT NULL,
                refresh_token TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create strava_tokens table: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS activities (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                strava_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                sport_type TEXT NOT NULL,
                start_date TEXT NOT NULL,
                elapsed_time INTEGER NOT NULL,
                moving_time INTEGER NOT NULL,
                distance REAL NOT NULL,
                total_elevation_gain REAL NOT NULL,
                average_speed REAL NOT NULL,
                max_speed REAL NOT NULL,
                average_heartrate REAL,
                max_heartrate REAL,
                average_cadence REAL,
                average_watts REAL,
                calories REAL,
                tag TEXT NOT NULL DEFAULT 'normal',
                summary_polyline TEXT,
                workout_type INTEGER,
                streams_loaded INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                UNIQUE(user_id, strava_id)
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create activities table: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_activities_user_date ON activities(user_id, start_date DESC)"
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create activities index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS activity_streams (
                activity_id TEXT NOT NULL REFERENCES activities(id),
                stream_type TEXT NOT NULL,
                data_json TEXT NOT NULL,
                PRIMARY KEY (activity_id, stream_type)
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create activity_streams table: {e}"))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS activity_laps (
                activity_id TEXT NOT NULL REFERENCES activities(id),
                lap_index INTEGER NOT NULL,
                name TEXT NOT NULL,
                start_date TEXT NOT NULL,
                elapsed_time INTEGER NOT NULL,
                moving_time INTEGER NOT NULL,
                distance REAL NOT NULL,
                average_speed REAL NOT NULL,
                max_speed REAL NOT NULL,
                total_elevation_gain REAL NOT NULL,
                average_heartrate REAL,
                max_heartrate REAL,
                PRIMARY KEY (activity_id, lap_index)
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create activity_laps table: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS activity_interval_results (
                activity_id TEXT PRIMARY KEY NOT NULL REFERENCES activities(id),
                algorithm TEXT NOT NULL,
                result_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!(
                "Failed to create activity_interval_results table: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS trainings (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                name TEXT NOT NULL,
                description TEXT,
                start_date TEXT,
                end_date TEXT,
                race_distance TEXT,
                race_objectif TEXT,
                created_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create trainings table: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_trainings_user ON trainings(user_id)")
            .execute(pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to create trainings index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS training_insights (
                id TEXT PRIMARY KEY NOT NULL,
                training_id TEXT NOT NULL REFERENCES trainings(id) ON DELETE CASCADE,
                user_id TEXT NOT NULL REFERENCES users(id),
                prompt_type TEXT NOT NULL,
                display_label TEXT NOT NULL,
                full_prompt TEXT NOT NULL,
                response TEXT NOT NULL,
                model TEXT,
                cost REAL,
                created_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create training_insights table: {e}"))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_training_insights_training ON training_insights(training_id, user_id)"
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training_insights index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ai_chats (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                training_id TEXT,
                source_insight_id TEXT,
                source_insight_cost REAL NOT NULL DEFAULT 0.0,
                title TEXT NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create ai_chats table: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_ai_chats_user ON ai_chats(user_id, updated_at DESC)",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create ai_chats index: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_ai_chats_source_insight ON ai_chats(user_id, source_insight_id)",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!(
                "Failed to create ai_chats source insight index: {e}"
            ))
        })?;

        // Add conversation_length column if it doesn't exist (migration)
        sqlx::query("ALTER TABLE ai_chats ADD COLUMN conversation_length INTEGER")
            .execute(pool)
            .await
            .ok(); // Ignore error if column already exists
        sqlx::query(
            "ALTER TABLE ai_chats ADD COLUMN source_insight_cost REAL NOT NULL DEFAULT 0.0",
        )
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ai_chat_messages (
                id TEXT PRIMARY KEY NOT NULL,
                chat_id TEXT NOT NULL REFERENCES ai_chats(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0.0,
                context_label TEXT,
                created_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create ai_chat_messages table: {e}"))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_ai_chat_messages_chat ON ai_chat_messages(chat_id, created_at ASC)"
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create ai_chat_messages index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS running_coach_settings (
                user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                model TEXT NOT NULL,
                personality TEXT NOT NULL,
                consider_trail_runs_as_runs INTEGER NOT NULL DEFAULT 0,
                volume_weeks INTEGER NOT NULL DEFAULT 8,
                last_workouts_count INTEGER NOT NULL DEFAULT 8,
                last_long_runs_count INTEGER NOT NULL DEFAULT 6,
                last_races_count INTEGER NOT NULL DEFAULT 4,
                new_activities_count INTEGER NOT NULL DEFAULT 8,
                normalizer_every_n_messages INTEGER NOT NULL DEFAULT 6,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!(
                "Failed to create running_coach_settings table: {e}"
            ))
        })?;

        match sqlx::query(
            "ALTER TABLE running_coach_settings
             ADD COLUMN consider_trail_runs_as_runs INTEGER NOT NULL DEFAULT 0",
        )
        .execute(pool)
        .await
        {
            Ok(_) => {}
            Err(SqlxError::Database(db_err))
                if db_err.message().contains("duplicate column name") => {}
            Err(e) => {
                return Err(DomainError::Storage(format!(
                    "Failed to ensure running_coach_settings.consider_trail_runs_as_runs column: {e}"
                )));
            }
        }

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS running_coach_memory (
                user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                data_json TEXT NOT NULL,
                message_count_since_normalization INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create running_coach_memory table: {e}"))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS running_coach_state (
                user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                last_interaction_at TEXT,
                last_seen_activity_start_date TEXT,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create running_coach_state table: {e}"))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS running_coach_messages (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!(
                "Failed to create running_coach_messages table: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_running_coach_messages_user ON running_coach_messages(user_id, created_at ASC)"
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!(
                "Failed to create running_coach_messages index: {e}"
            ))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS quota_requests (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                status TEXT NOT NULL DEFAULT 'pending',
                requested_at TEXT NOT NULL,
                resolved_at TEXT,
                granted_amount_usd REAL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create quota_requests table: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS invite_codes (
                id TEXT PRIMARY KEY NOT NULL,
                code_hash TEXT UNIQUE NOT NULL,
                created_by_user_id TEXT REFERENCES users(id),
                created_for TEXT,
                created_at TEXT NOT NULL,
                expires_at TEXT,
                used_at TEXT,
                used_by_strava_athlete_id INTEGER,
                revoked_at TEXT
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create invite_codes table: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_invite_codes_created_at ON invite_codes(created_at DESC)",
        )
        .execute(pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create invite_codes index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS model_cost_tiers (
                model_id TEXT PRIMARY KEY NOT NULL,
                model_name TEXT NOT NULL,
                category TEXT NOT NULL,
                computed_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to create model_cost_tiers table: {e}"))
        })?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper: parse a DateTime<Utc> from a TEXT column stored as RFC 3339
// ---------------------------------------------------------------------------
fn parse_datetime(value: &str) -> Result<DateTime<Utc>, DomainError> {
    value
        .parse::<DateTime<Utc>>()
        .map_err(|e| DomainError::Storage(format!("Invalid datetime '{value}': {e}")))
}

fn parse_uuid(value: &str) -> Result<Uuid, DomainError> {
    value
        .parse::<Uuid>()
        .map_err(|e| DomainError::Storage(format!("Invalid UUID '{value}': {e}")))
}

fn parse_tag(value: &str) -> Result<ActivityTag, DomainError> {
    value
        .parse::<ActivityTag>()
        .map_err(|e| DomainError::Storage(format!("Invalid activity tag: {e}")))
}

// ---------------------------------------------------------------------------
// Row → domain mappers
// ---------------------------------------------------------------------------
fn row_to_user(row: &SqliteRow) -> Result<User, DomainError> {
    let id: String = row.get("id");
    let username: String = row.get("username");
    let display_name: String = row.get("display_name");
    let email: Option<String> = row.try_get("email").unwrap_or(None);
    let created_at: String = row.get("created_at");
    let mas_current: Option<f64> = row.get("mas_current");

    let quota_balance_usd: f64 = row.try_get("quota_balance_usd").unwrap_or(0.0);

    Ok(User {
        id: parse_uuid(&id)?,
        username,
        display_name,
        email,
        created_at: parse_datetime(&created_at)?,
        mas_current,
        quota_balance_usd,
    })
}

fn row_to_identity_profile(row: &SqliteRow) -> Result<IdentityProfile, DomainError> {
    let user_id: String = row.get("user_id");
    let name: Option<String> = row.get("name");
    let age: Option<i32> = row.get("age");
    let email: Option<String> = row.get("email");
    let gender: Option<String> = row.get("gender");
    let height_cm: Option<f64> = row.get("height_cm");
    let weight_kg: Option<f64> = row.get("weight_kg");
    let updated_at: String = row.get("updated_at");

    Ok(IdentityProfile {
        user_id: parse_uuid(&user_id)?,
        name,
        age,
        email,
        gender,
        height_cm,
        weight_kg,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn row_to_athlete_profile(row: &SqliteRow) -> Result<AthleteProfile, DomainError> {
    let user_id: String = row.get("user_id");
    let goal_description: Option<String> = row.get("goal_description");
    let goal_date: Option<String> = row.get("goal_date");
    let goal_distance_km: Option<f64> = row.get("goal_distance_km");
    let goal_target_time_seconds: Option<i32> = row.get("goal_target_time_seconds");
    let goal_sport_type: Option<String> = row.get("goal_sport_type");
    let goal_elevation_gain_m: Option<f64> = row.get("goal_elevation_gain_m");
    let additional_info: Option<String> = row.get("additional_info");
    let updated_at: String = row.get("updated_at");

    Ok(AthleteProfile {
        user_id: parse_uuid(&user_id)?,
        goal_description,
        goal_date,
        goal_distance_km,
        goal_target_time_seconds,
        goal_sport_type,
        goal_elevation_gain_m,
        additional_info,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn row_to_quota_request(row: &SqliteRow) -> Result<QuotaRequest, DomainError> {
    let id: String = row.get("id");
    let user_id: String = row.get("user_id");
    let status: String = row.get("status");
    let requested_at: String = row.get("requested_at");
    let resolved_at: Option<String> = row.get("resolved_at");
    let granted_amount_usd: Option<f64> = row.get("granted_amount_usd");

    Ok(QuotaRequest {
        id: parse_uuid(&id)?,
        user_id: parse_uuid(&user_id)?,
        status: QuotaRequestStatus::from_str(&status),
        requested_at: parse_datetime(&requested_at)?,
        resolved_at: resolved_at.as_deref().map(parse_datetime).transpose()?,
        granted_amount_usd,
    })
}

fn row_to_invite_code(row: &SqliteRow) -> Result<InviteCode, DomainError> {
    let id: String = row.get("id");
    let code_hash: String = row.get("code_hash");
    let created_by_user_id: Option<String> = row.get("created_by_user_id");
    let created_for: Option<String> = row.get("created_for");
    let created_at: String = row.get("created_at");
    let expires_at: Option<String> = row.get("expires_at");
    let used_at: Option<String> = row.get("used_at");
    let used_by_strava_athlete_id: Option<i64> = row.get("used_by_strava_athlete_id");
    let revoked_at: Option<String> = row.get("revoked_at");

    Ok(InviteCode {
        id: parse_uuid(&id)?,
        code_hash,
        created_by_user_id: created_by_user_id.as_deref().map(parse_uuid).transpose()?,
        created_for,
        created_at: parse_datetime(&created_at)?,
        expires_at: expires_at.as_deref().map(parse_datetime).transpose()?,
        used_at: used_at.as_deref().map(parse_datetime).transpose()?,
        used_by_strava_athlete_id,
        revoked_at: revoked_at.as_deref().map(parse_datetime).transpose()?,
    })
}

fn row_to_model_cost_tier(row: &SqliteRow) -> Result<ModelCostTier, DomainError> {
    let model_id: String = row.get("model_id");
    let model_name: String = row.get("model_name");
    let category: String = row.get("category");
    let computed_at: String = row.get("computed_at");
    let parsed_category = category
        .parse::<ModelCostCategory>()
        .map_err(|e| DomainError::Storage(e.to_string()))?;

    Ok(ModelCostTier {
        model_id,
        model_name,
        category: parsed_category,
        computed_at: parse_datetime(&computed_at)?,
    })
}

fn row_to_strava_token(row: &SqliteRow) -> Result<StravaToken, DomainError> {
    let user_id: String = row.get("user_id");
    let strava_athlete_id: i64 = row.get("strava_athlete_id");
    let access_token: String = row.get("access_token");
    let refresh_token: String = row.get("refresh_token");
    let expires_at: String = row.get("expires_at");

    Ok(StravaToken {
        user_id: parse_uuid(&user_id)?,
        strava_athlete_id,
        access_token,
        refresh_token,
        expires_at: parse_datetime(&expires_at)?,
    })
}

fn row_to_activity(row: &SqliteRow) -> Result<Activity, DomainError> {
    let id: String = row.get("id");
    let user_id: String = row.get("user_id");
    let strava_id: i64 = row.get("strava_id");
    let name: String = row.get("name");
    let sport_type: String = row.get("sport_type");
    let start_date: String = row.get("start_date");
    let elapsed_time: i32 = row.get("elapsed_time");
    let moving_time: i32 = row.get("moving_time");
    let distance: f64 = row.get("distance");
    let total_elevation_gain: f64 = row.get("total_elevation_gain");
    let average_speed: f64 = row.get("average_speed");
    let max_speed: f64 = row.get("max_speed");
    let average_heartrate: Option<f64> = row.get("average_heartrate");
    let max_heartrate: Option<f64> = row.get("max_heartrate");
    let average_cadence: Option<f64> = row.get("average_cadence");
    let average_watts: Option<f64> = row.get("average_watts");
    let calories: Option<f64> = row.get("calories");
    let tag: String = row.get("tag");
    let summary_polyline: Option<String> = row.get("summary_polyline");
    let workout_type: Option<i32> = row.get("workout_type");
    let streams_loaded_raw: i64 = row.get("streams_loaded");
    let created_at: String = row.get("created_at");

    Ok(Activity {
        id: parse_uuid(&id)?,
        user_id: parse_uuid(&user_id)?,
        strava_id,
        name,
        sport_type,
        start_date: parse_datetime(&start_date)?,
        elapsed_time,
        moving_time,
        distance,
        total_elevation_gain,
        average_speed,
        max_speed,
        average_heartrate,
        max_heartrate,
        average_cadence,
        average_watts,
        calories,
        tag: parse_tag(&tag)?,
        summary_polyline,
        workout_type,
        streams_fetched_at: if streams_loaded_raw == 0 {
            None
        } else {
            DateTime::from_timestamp(streams_loaded_raw, 0)
        },
        created_at: parse_datetime(&created_at)?,
    })
}

fn parse_stream_type(value: &str) -> Result<domain::StreamType, DomainError> {
    value
        .parse::<domain::StreamType>()
        .map_err(|e| DomainError::Storage(format!("Invalid stream type: {e}")))
}

fn row_to_training(row: &SqliteRow) -> Result<Training, DomainError> {
    let id: String = row.get("id");
    let user_id: String = row.get("user_id");
    let name: String = row.get("name");
    let description: Option<String> = row.get("description");
    let start_date: Option<String> = row.get("start_date");
    let end_date: Option<String> = row.get("end_date");
    let race_distance: Option<String> = row.get("race_distance");
    let race_objectif: Option<String> = row.try_get("race_objectif").ok();
    let created_at: String = row.get("created_at");

    Ok(Training {
        id: parse_uuid(&id)?,
        user_id: parse_uuid(&user_id)?,
        name,
        description,
        start_date: start_date.map(|s| parse_datetime(&s)).transpose()?,
        end_date: end_date.map(|s| parse_datetime(&s)).transpose()?,
        race_distance,
        race_objectif,
        created_at: parse_datetime(&created_at)?,
    })
}

fn row_to_activity_stream(row: &SqliteRow) -> Result<ActivityStream, DomainError> {
    let activity_id: String = row.get("activity_id");
    let stream_type: String = row.get("stream_type");
    let data_json: String = row.get("data_json");

    Ok(ActivityStream {
        activity_id: parse_uuid(&activity_id)?,
        stream_type: parse_stream_type(&stream_type)?,
        data_json,
    })
}

fn row_to_activity_lap(row: &SqliteRow) -> Result<ActivityLap, DomainError> {
    let activity_id: String = row.get("activity_id");
    let lap_index: i32 = row.get("lap_index");
    let name: String = row.get("name");
    let start_date: String = row.get("start_date");
    let elapsed_time: i32 = row.get("elapsed_time");
    let moving_time: i32 = row.get("moving_time");
    let distance: f64 = row.get("distance");
    let average_speed: f64 = row.get("average_speed");
    let max_speed: f64 = row.get("max_speed");
    let total_elevation_gain: f64 = row.get("total_elevation_gain");
    let average_heartrate: Option<f64> = row.get("average_heartrate");
    let max_heartrate: Option<f64> = row.get("max_heartrate");

    Ok(ActivityLap {
        activity_id: parse_uuid(&activity_id)?,
        lap_index,
        name,
        start_date: parse_datetime(&start_date)?,
        elapsed_time,
        moving_time,
        distance,
        average_speed,
        max_speed,
        total_elevation_gain,
        average_heartrate,
        max_heartrate,
    })
}

fn row_to_training_insight(row: &SqliteRow) -> Result<TrainingInsight, DomainError> {
    let id: String = row.get("id");
    let training_id: String = row.get("training_id");
    let user_id: String = row.get("user_id");
    let prompt_type: String = row.get("prompt_type");
    let display_label: String = row.get("display_label");
    let full_prompt: String = row.get("full_prompt");
    let response: String = row.get("response");
    let model: Option<String> = row.try_get("model").ok();
    let cost: Option<f64> = row.try_get("cost").ok();
    let created_at: String = row.get("created_at");

    Ok(TrainingInsight {
        id: parse_uuid(&id)?,
        training_id: parse_uuid(&training_id)?,
        user_id: parse_uuid(&user_id)?,
        prompt_type,
        display_label,
        full_prompt,
        response,
        model,
        cost,
        created_at: parse_datetime(&created_at)?,
    })
}

fn row_to_ai_chat(row: &SqliteRow) -> Result<AiChat, DomainError> {
    let id: String = row.get("id");
    let user_id: String = row.get("user_id");
    let training_id: Option<String> = row.get("training_id");
    let source_insight_id: Option<String> = row.get("source_insight_id");
    let source_insight_cost: f64 = row.try_get("source_insight_cost").unwrap_or(0.0);
    let title: String = row.get("title");
    let model: String = row.get("model");
    let conversation_length: Option<i32> = row
        .try_get::<Option<i32>, _>("conversation_length")
        .unwrap_or(None);
    let created_at: String = row.get("created_at");
    let updated_at: String = row.get("updated_at");

    Ok(AiChat {
        id: parse_uuid(&id)?,
        user_id: parse_uuid(&user_id)?,
        training_id: training_id.map(|s| parse_uuid(&s)).transpose()?,
        source_insight_id: source_insight_id.map(|s| parse_uuid(&s)).transpose()?,
        source_insight_cost,
        title,
        model,
        conversation_length: conversation_length.map(|v| v as u32),
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn row_to_ai_chat_message(row: &SqliteRow) -> Result<AiChatMessage, DomainError> {
    let id: String = row.get("id");
    let chat_id: String = row.get("chat_id");
    let role: String = row.get("role");
    let content: String = row.get("content");
    let prompt_tokens: i32 = row.get("prompt_tokens");
    let completion_tokens: i32 = row.get("completion_tokens");
    let total_tokens: i32 = row.get("total_tokens");
    let cost: f64 = row.get("cost");
    let context_label: Option<String> = row.get("context_label");
    let created_at: String = row.get("created_at");

    Ok(AiChatMessage {
        id: parse_uuid(&id)?,
        chat_id: parse_uuid(&chat_id)?,
        role,
        content,
        prompt_tokens: prompt_tokens as u32,
        completion_tokens: completion_tokens as u32,
        total_tokens: total_tokens as u32,
        cost,
        context_label,
        created_at: parse_datetime(&created_at)?,
    })
}

fn row_to_running_coach_settings(row: &SqliteRow) -> Result<RunningCoachSettings, DomainError> {
    let user_id: String = row.get("user_id");
    let model: String = row.get("model");
    let personality: String = row.get("personality");
    let consider_trail_runs_as_runs: bool = row.get("consider_trail_runs_as_runs");
    let volume_weeks: i32 = row.get("volume_weeks");
    let last_workouts_count: i32 = row.get("last_workouts_count");
    let last_long_runs_count: i32 = row.get("last_long_runs_count");
    let last_races_count: i32 = row.get("last_races_count");
    let new_activities_count: i32 = row.get("new_activities_count");
    let normalizer_every_n_messages: i32 = row.get("normalizer_every_n_messages");
    let created_at: String = row.get("created_at");
    let updated_at: String = row.get("updated_at");

    Ok(RunningCoachSettings {
        user_id: parse_uuid(&user_id)?,
        model,
        personality,
        consider_trail_runs_as_runs,
        volume_weeks,
        last_workouts_count,
        last_long_runs_count,
        last_races_count,
        new_activities_count,
        normalizer_every_n_messages,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn row_to_running_coach_memory(row: &SqliteRow) -> Result<RunningCoachMemory, DomainError> {
    let user_id: String = row.get("user_id");
    let data_json: String = row.get("data_json");
    let message_count_since_normalization: i32 = row.get("message_count_since_normalization");
    let updated_at: String = row.get("updated_at");
    let data: RunningCoachMemoryData = serde_json::from_str(&data_json).map_err(|e| {
        DomainError::Storage(format!(
            "Failed to deserialize running_coach_memory JSON: {e}"
        ))
    })?;

    Ok(RunningCoachMemory {
        user_id: parse_uuid(&user_id)?,
        data,
        message_count_since_normalization,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn row_to_running_coach_state(row: &SqliteRow) -> Result<RunningCoachState, DomainError> {
    let user_id: String = row.get("user_id");
    let last_interaction_at: Option<String> = row.get("last_interaction_at");
    let last_seen_activity_start_date: Option<String> = row.get("last_seen_activity_start_date");
    let updated_at: String = row.get("updated_at");

    Ok(RunningCoachState {
        user_id: parse_uuid(&user_id)?,
        last_interaction_at: last_interaction_at
            .as_deref()
            .map(parse_datetime)
            .transpose()?,
        last_seen_activity_start_date: last_seen_activity_start_date
            .as_deref()
            .map(parse_datetime)
            .transpose()?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn row_to_running_coach_message(row: &SqliteRow) -> Result<RunningCoachMessage, DomainError> {
    let id: String = row.get("id");
    let user_id: String = row.get("user_id");
    let role: String = row.get("role");
    let content: String = row.get("content");
    let prompt_tokens: i32 = row.get("prompt_tokens");
    let completion_tokens: i32 = row.get("completion_tokens");
    let total_tokens: i32 = row.get("total_tokens");
    let cost: f64 = row.get("cost");
    let created_at: String = row.get("created_at");

    Ok(RunningCoachMessage {
        id: parse_uuid(&id)?,
        user_id: parse_uuid(&user_id)?,
        role,
        content,
        prompt_tokens: prompt_tokens as u32,
        completion_tokens: completion_tokens as u32,
        total_tokens: total_tokens as u32,
        cost,
        created_at: parse_datetime(&created_at)?,
    })
}

fn default_running_coach_settings(user_id: Uuid) -> RunningCoachSettings {
    let now = Utc::now();
    let mut settings = RunningCoachSettings::default();
    settings.user_id = user_id;
    settings.created_at = now;
    settings.updated_at = now;
    settings
}

fn default_running_coach_memory(user_id: Uuid) -> RunningCoachMemory {
    let now = Utc::now();
    let mut memory = RunningCoachMemory::default();
    memory.user_id = user_id;
    memory.updated_at = now;
    memory
}

fn default_running_coach_state(user_id: Uuid) -> RunningCoachState {
    RunningCoachState {
        user_id,
        last_interaction_at: None,
        last_seen_activity_start_date: None,
        updated_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------
#[async_trait]
impl Storage for SqliteStorage {
    // -----------------------------------------------------------------------
    // Users
    // -----------------------------------------------------------------------
    async fn create_user(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO users (id, username, display_name, email, created_at, mas_current, quota_balance_usd) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user.id.to_string())
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.email)
        .bind(user.created_at.to_rfc3339())
        .bind(user.mas_current)
        .bind(user.quota_balance_usd)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create user: {e}")))?;

        Ok(())
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<User, DomainError> {
        let row = sqlx::query("SELECT id, username, display_name, email, created_at, mas_current, quota_balance_usd FROM users WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to get user: {e}")))?
            .ok_or_else(|| DomainError::NotFound(format!("User {id} not found")))?;

        row_to_user(&row)
    }

    async fn list_users(&self) -> Result<Vec<User>, DomainError> {
        let rows = sqlx::query("SELECT id, username, display_name, email, created_at, mas_current, quota_balance_usd FROM users ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to list users: {e}")))?;

        rows.iter().map(row_to_user).collect()
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError> {
        let row = sqlx::query(
            "SELECT id, username, display_name, email, created_at, mas_current, quota_balance_usd FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get user: {e}")))?
        .ok_or_else(|| DomainError::NotFound(format!("User '{username}' not found")))?;

        row_to_user(&row)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, DomainError> {
        let row = sqlx::query(
            "SELECT id, username, display_name, email, created_at, mas_current, quota_balance_usd FROM users WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get user by email: {e}")))?
        .ok_or_else(|| DomainError::NotFound(format!("User email '{email}' not found")))?;

        row_to_user(&row)
    }

    // -----------------------------------------------------------------------
    // Strava tokens
    // -----------------------------------------------------------------------
    async fn upsert_strava_token(&self, token: &StravaToken) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO strava_tokens (user_id, strava_athlete_id, access_token, refresh_token, expires_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                 strava_athlete_id = excluded.strava_athlete_id,
                 access_token = excluded.access_token,
                 refresh_token = excluded.refresh_token,
                 expires_at = excluded.expires_at",
        )
        .bind(token.user_id.to_string())
        .bind(token.strava_athlete_id)
        .bind(&token.access_token)
        .bind(&token.refresh_token)
        .bind(token.expires_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to upsert strava token: {e}")))?;

        Ok(())
    }

    async fn get_strava_token(&self, user_id: Uuid) -> Result<StravaToken, DomainError> {
        let row = sqlx::query(
            "SELECT user_id, strava_athlete_id, access_token, refresh_token, expires_at
             FROM strava_tokens WHERE user_id = ?",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get strava token: {e}")))?
        .ok_or_else(|| {
            DomainError::NotFound(format!("Strava token for user {user_id} not found"))
        })?;

        row_to_strava_token(&row)
    }

    async fn get_strava_token_by_athlete_id(
        &self,
        athlete_id: i64,
    ) -> Result<StravaToken, DomainError> {
        let row = sqlx::query(
            "SELECT user_id, strava_athlete_id, access_token, refresh_token, expires_at
             FROM strava_tokens WHERE strava_athlete_id = ?",
        )
        .bind(athlete_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to get strava token by athlete id: {e}"))
        })?
        .ok_or_else(|| {
            DomainError::NotFound(format!("Strava token for athlete {athlete_id} not found"))
        })?;

        row_to_strava_token(&row)
    }

    async fn delete_activity_by_strava_id(
        &self,
        strava_id: i64,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        let uid = user_id.to_string();

        // Find the activity's internal id
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM activities WHERE strava_id = ? AND user_id = ?")
                .bind(strava_id)
                .bind(&uid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    DomainError::Storage(format!("Failed to find activity by strava_id: {e}"))
                })?;

        let activity_id = match row {
            Some((id,)) => id,
            None => return Ok(()), // Activity doesn't exist locally, nothing to delete
        };

        // Delete streams/laps
        sqlx::query("DELETE FROM activity_streams WHERE activity_id = ?")
            .bind(&activity_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete streams: {e}")))?;
        sqlx::query("DELETE FROM activity_laps WHERE activity_id = ?")
            .bind(&activity_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete laps: {e}")))?;
        sqlx::query("DELETE FROM activity_interval_results WHERE activity_id = ?")
            .bind(&activity_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete interval result: {e}")))?;

        // Delete the activity
        sqlx::query("DELETE FROM activities WHERE id = ?")
            .bind(&activity_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete activity: {e}")))?;

        Ok(())
    }

    async fn delete_strava_data(&self, user_id: Uuid) -> Result<(), DomainError> {
        let uid = user_id.to_string();

        // Delete streams/laps for this user's activities
        sqlx::query(
            "DELETE FROM activity_streams WHERE activity_id IN (SELECT id FROM activities WHERE user_id = ?)",
        )
        .bind(&uid)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to delete streams: {e}")))?;
        sqlx::query(
            "DELETE FROM activity_laps WHERE activity_id IN (SELECT id FROM activities WHERE user_id = ?)",
        )
        .bind(&uid)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to delete laps: {e}")))?;
        sqlx::query(
            "DELETE FROM activity_interval_results WHERE activity_id IN (SELECT id FROM activities WHERE user_id = ?)",
        )
        .bind(&uid)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to delete interval results: {e}")))?;

        // Delete activities
        sqlx::query("DELETE FROM activities WHERE user_id = ?")
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete activities: {e}")))?;

        // Delete strava token
        sqlx::query("DELETE FROM strava_tokens WHERE user_id = ?")
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete strava token: {e}")))?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Activities
    // -----------------------------------------------------------------------
    async fn upsert_activities(&self, activities: &[Activity]) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to begin transaction: {e}")))?;

        for activity in activities {
            sqlx::query(
                "INSERT INTO activities (
                    id, user_id, strava_id, name, sport_type, start_date,
                    elapsed_time, moving_time, distance, total_elevation_gain,
                    average_speed, max_speed, average_heartrate, max_heartrate,
                    average_cadence, average_watts, calories, tag,
                    summary_polyline, workout_type, streams_loaded, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(user_id, strava_id) DO UPDATE SET
                    name = excluded.name,
                    sport_type = excluded.sport_type,
                    start_date = excluded.start_date,
                    elapsed_time = excluded.elapsed_time,
                    moving_time = excluded.moving_time,
                    distance = excluded.distance,
                    total_elevation_gain = excluded.total_elevation_gain,
                    average_speed = excluded.average_speed,
                    max_speed = excluded.max_speed,
                    average_heartrate = excluded.average_heartrate,
                    max_heartrate = excluded.max_heartrate,
                    average_cadence = excluded.average_cadence,
                    average_watts = excluded.average_watts,
                    calories = excluded.calories,
                    summary_polyline = excluded.summary_polyline,
                    workout_type = excluded.workout_type",
            )
            .bind(activity.id.to_string())
            .bind(activity.user_id.to_string())
            .bind(activity.strava_id)
            .bind(&activity.name)
            .bind(&activity.sport_type)
            .bind(activity.start_date.to_rfc3339())
            .bind(activity.elapsed_time)
            .bind(activity.moving_time)
            .bind(activity.distance)
            .bind(activity.total_elevation_gain)
            .bind(activity.average_speed)
            .bind(activity.max_speed)
            .bind(activity.average_heartrate)
            .bind(activity.max_heartrate)
            .bind(activity.average_cadence)
            .bind(activity.average_watts)
            .bind(activity.calories)
            .bind(activity.tag.to_string())
            .bind(Option::<String>::None) // Never persist GPS polyline
            .bind(activity.workout_type)
            .bind(
                activity
                    .streams_fetched_at
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0i64),
            )
            .bind(activity.created_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                DomainError::Storage(format!(
                    "Failed to upsert activity {}: {e}",
                    activity.strava_id
                ))
            })?;
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    async fn get_activities(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Activity>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, strava_id, name, sport_type, start_date,
                    elapsed_time, moving_time, distance, total_elevation_gain,
                    average_speed, max_speed, average_heartrate, max_heartrate,
                    average_cadence, average_watts, calories, tag,
                    summary_polyline, workout_type, streams_loaded, created_at
             FROM activities
             WHERE user_id = ?
             ORDER BY start_date DESC
             LIMIT ? OFFSET ?",
        )
        .bind(user_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get activities: {e}")))?;

        rows.iter().map(row_to_activity).collect()
    }

    async fn get_activity(&self, id: Uuid, user_id: Uuid) -> Result<Activity, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, strava_id, name, sport_type, start_date,
                    elapsed_time, moving_time, distance, total_elevation_gain,
                    average_speed, max_speed, average_heartrate, max_heartrate,
                    average_cadence, average_watts, calories, tag,
                    summary_polyline, workout_type, streams_loaded, created_at
             FROM activities
             WHERE id = ? AND user_id = ?",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get activity: {e}")))?
        .ok_or_else(|| DomainError::NotFound(format!("Activity {id} not found")))?;

        row_to_activity(&row)
    }

    async fn get_latest_activity_start(
        &self,
        user_id: Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, DomainError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT start_date FROM activities WHERE user_id = ? ORDER BY start_date DESC LIMIT 1",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get latest activity: {e}")))?;

        match row {
            Some((date_str,)) => Ok(Some(parse_datetime(&date_str)?)),
            None => Ok(None),
        }
    }

    async fn update_activity_tag(
        &self,
        id: Uuid,
        user_id: Uuid,
        tag: ActivityTag,
    ) -> Result<(), DomainError> {
        let result = sqlx::query("UPDATE activities SET tag = ? WHERE id = ? AND user_id = ?")
            .bind(tag.to_string())
            .bind(id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to update activity tag: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Activity {id} not found")));
        }

        Ok(())
    }

    async fn mark_streams_fetched(&self, activity_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query("UPDATE activities SET streams_loaded = ? WHERE id = ?")
            .bind(Utc::now().timestamp())
            .bind(activity_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to mark streams fetched: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!(
                "Activity {activity_id} not found"
            )));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Streams
    // -----------------------------------------------------------------------
    async fn store_streams(&self, streams: &[ActivityStream]) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to begin transaction: {e}")))?;

        let mut activity_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
        for stream in streams {
            // Never persist GPS data
            if stream.stream_type == domain::StreamType::LatLng {
                continue;
            }
            activity_ids.insert(stream.activity_id);
            sqlx::query(
                "INSERT INTO activity_streams (activity_id, stream_type, data_json)
                 VALUES (?, ?, ?)
                 ON CONFLICT(activity_id, stream_type) DO UPDATE SET
                     data_json = excluded.data_json",
            )
            .bind(stream.activity_id.to_string())
            .bind(stream.stream_type.to_string())
            .bind(&stream.data_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to store stream: {e}")))?;
        }

        for activity_id in activity_ids {
            sqlx::query("DELETE FROM activity_interval_results WHERE activity_id = ?")
                .bind(activity_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    DomainError::Storage(format!(
                        "Failed to invalidate parsed intervals after stream update: {e}"
                    ))
                })?;
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    async fn get_streams(&self, activity_id: Uuid) -> Result<Vec<ActivityStream>, DomainError> {
        let rows = sqlx::query(
            "SELECT activity_id, stream_type, data_json
             FROM activity_streams
             WHERE activity_id = ?",
        )
        .bind(activity_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get streams: {e}")))?;

        rows.iter().map(row_to_activity_stream).collect()
    }

    async fn store_laps(&self, laps: &[ActivityLap]) -> Result<(), DomainError> {
        if laps.is_empty() {
            return Ok(());
        }

        let activity_id = laps[0].activity_id.to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to begin transaction: {e}")))?;

        sqlx::query("DELETE FROM activity_laps WHERE activity_id = ?")
            .bind(&activity_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to clear existing laps: {e}")))?;
        sqlx::query("DELETE FROM activity_interval_results WHERE activity_id = ?")
            .bind(&activity_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                DomainError::Storage(format!(
                    "Failed to invalidate parsed intervals after lap update: {e}"
                ))
            })?;

        for lap in laps {
            sqlx::query(
                "INSERT INTO activity_laps (
                    activity_id, lap_index, name, start_date,
                    elapsed_time, moving_time, distance,
                    average_speed, max_speed, total_elevation_gain,
                    average_heartrate, max_heartrate
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(lap.activity_id.to_string())
            .bind(lap.lap_index)
            .bind(&lap.name)
            .bind(lap.start_date.to_rfc3339())
            .bind(lap.elapsed_time)
            .bind(lap.moving_time)
            .bind(lap.distance)
            .bind(lap.average_speed)
            .bind(lap.max_speed)
            .bind(lap.total_elevation_gain)
            .bind(lap.average_heartrate)
            .bind(lap.max_heartrate)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to store lap: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    async fn get_laps(&self, activity_id: Uuid) -> Result<Vec<ActivityLap>, DomainError> {
        let rows = sqlx::query(
            "SELECT activity_id, lap_index, name, start_date,
                    elapsed_time, moving_time, distance,
                    average_speed, max_speed, total_elevation_gain,
                    average_heartrate, max_heartrate
             FROM activity_laps
             WHERE activity_id = ?
             ORDER BY lap_index ASC",
        )
        .bind(activity_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get laps: {e}")))?;

        rows.iter().map(row_to_activity_lap).collect()
    }

    async fn store_interval_result(
        &self,
        activity_id: Uuid,
        algorithm: &str,
        result_json: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO activity_interval_results (activity_id, algorithm, result_json, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(activity_id) DO UPDATE SET
                algorithm = excluded.algorithm,
                result_json = excluded.result_json,
                updated_at = excluded.updated_at",
        )
        .bind(activity_id.to_string())
        .bind(algorithm)
        .bind(result_json)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to store interval result: {e}")))?;

        Ok(())
    }

    async fn get_interval_result(
        &self,
        activity_id: Uuid,
    ) -> Result<Option<(String, String)>, DomainError> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT algorithm, result_json
             FROM activity_interval_results
             WHERE activity_id = ?",
        )
        .bind(activity_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get interval result: {e}")))
    }

    // -----------------------------------------------------------------------
    // Trainings
    // -----------------------------------------------------------------------
    async fn create_training(&self, training: &Training) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO trainings (id, user_id, name, description, start_date, end_date, race_distance, race_objectif, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(training.id.to_string())
        .bind(training.user_id.to_string())
        .bind(&training.name)
        .bind(&training.description)
        .bind(training.start_date.map(|d| d.to_rfc3339()))
        .bind(training.end_date.map(|d| d.to_rfc3339()))
        .bind(&training.race_distance)
        .bind(&training.race_objectif)
        .bind(training.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training: {e}")))?;

        Ok(())
    }

    async fn get_training(&self, id: Uuid, user_id: Uuid) -> Result<Training, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, name, description, start_date, end_date, race_distance, race_objectif, created_at
             FROM trainings
             WHERE id = ? AND user_id = ?",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get training: {e}")))?
        .ok_or_else(|| DomainError::NotFound(format!("Training {id} not found")))?;

        row_to_training(&row)
    }

    async fn list_trainings(&self, user_id: Uuid) -> Result<Vec<Training>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, name, description, start_date, end_date, race_distance, race_objectif, created_at
             FROM trainings
             WHERE user_id = ?
             ORDER BY created_at DESC",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to list trainings: {e}")))?;

        rows.iter().map(row_to_training).collect()
    }

    async fn update_training(
        &self,
        id: Uuid,
        user_id: Uuid,
        name: String,
        description: Option<String>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        race_distance: Option<String>,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE trainings SET name = ?, description = ?, start_date = ?, end_date = ?, race_distance = ? WHERE id = ? AND user_id = ?",
        )
        .bind(&name)
        .bind(&description)
        .bind(start_date.map(|d| d.to_rfc3339()))
        .bind(end_date.map(|d| d.to_rfc3339()))
        .bind(&race_distance)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to update training: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Training {id} not found")));
        }

        Ok(())
    }

    async fn delete_training(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query("DELETE FROM trainings WHERE id = ? AND user_id = ?")
            .bind(id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete training: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Training {id} not found")));
        }

        Ok(())
    }

    async fn get_training_activities(
        &self,
        training_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Activity>, DomainError> {
        let training = self.get_training(training_id, user_id).await?;

        let rows = sqlx::query(
            "SELECT a.id, a.user_id, a.strava_id, a.name, a.sport_type, a.start_date,
                    a.elapsed_time, a.moving_time, a.distance, a.total_elevation_gain,
                    a.average_speed, a.max_speed, a.average_heartrate, a.max_heartrate,
                    a.average_cadence, a.average_watts, a.calories, a.tag, a.summary_polyline,
                    a.workout_type, a.streams_loaded, a.created_at
             FROM activities a
             WHERE a.user_id = ?
               AND a.tag IN ('intervals', 'long_run')
               AND (? IS NULL OR a.start_date >= ?)
               AND (? IS NULL OR a.start_date < ?)
             ORDER BY a.start_date DESC",
        )
        .bind(user_id.to_string())
        .bind(training.start_date.map(|d| d.to_rfc3339()))
        .bind(training.start_date.map(|d| d.to_rfc3339()))
        .bind(training.end_date.map(|d| d.to_rfc3339()))
        .bind(training.end_date.map(|d| d.to_rfc3339()))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get training activities: {e}")))?;

        rows.iter().map(row_to_activity).collect()
    }

    async fn get_activity_trainings(
        &self,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Training>, DomainError> {
        let activity = self.get_activity(activity_id, user_id).await?;
        if !matches!(activity.tag, ActivityTag::Intervals | ActivityTag::LongRun) {
            return Ok(vec![]);
        }

        let rows = sqlx::query(
            "SELECT t.id, t.user_id, t.name, t.description, t.start_date, t.end_date, t.race_distance, t.race_objectif, t.created_at
             FROM trainings t
             WHERE t.user_id = ?
               AND (? IS NULL OR t.start_date IS NULL OR ? >= t.start_date)
               AND (? IS NULL OR t.end_date IS NULL OR ? < t.end_date)
             ORDER BY t.created_at DESC",
        )
        .bind(user_id.to_string())
        .bind(Some(activity.start_date.to_rfc3339()))
        .bind(Some(activity.start_date.to_rfc3339()))
        .bind(Some(activity.start_date.to_rfc3339()))
        .bind(Some(activity.start_date.to_rfc3339()))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to get activity trainings: {e}"))
        })?;

        rows.iter().map(row_to_training).collect()
    }

    async fn get_activities_in_range(
        &self,
        user_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Activity>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, strava_id, name, sport_type, start_date,
                    elapsed_time, moving_time, distance, total_elevation_gain,
                    average_speed, max_speed, average_heartrate, max_heartrate,
                    average_cadence, average_watts, calories, tag,
                    summary_polyline, workout_type, streams_loaded, created_at
             FROM activities
             WHERE user_id = ? AND start_date >= ? AND start_date < ?
             ORDER BY start_date ASC",
        )
        .bind(user_id.to_string())
        .bind(from.to_rfc3339())
        .bind(to.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get activities in range: {e}")))?;

        rows.iter().map(row_to_activity).collect()
    }

    // -----------------------------------------------------------------------
    // Training Insights
    // -----------------------------------------------------------------------
    async fn store_training_insight(&self, insight: &TrainingInsight) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO training_insights (id, training_id, user_id, prompt_type, display_label, full_prompt, response, model, cost, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(insight.id.to_string())
        .bind(insight.training_id.to_string())
        .bind(insight.user_id.to_string())
        .bind(&insight.prompt_type)
        .bind(&insight.display_label)
        .bind(&insight.full_prompt)
        .bind(&insight.response)
        .bind(insight.model.as_deref())
        .bind(insight.cost)
        .bind(insight.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to store training insight: {e}")))?;

        Ok(())
    }

    async fn get_training_insights(
        &self,
        training_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TrainingInsight>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, training_id, user_id, prompt_type, display_label, full_prompt, response, model, cost, created_at
             FROM training_insights
             WHERE training_id = ? AND user_id = ?
             ORDER BY created_at DESC",
        )
        .bind(training_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get training insights: {e}")))?;

        rows.iter().map(row_to_training_insight).collect()
    }

    // -----------------------------------------------------------------------
    // User MAS
    // -----------------------------------------------------------------------
    async fn update_user_mas(
        &self,
        user_id: Uuid,
        mas_kmh: Option<f64>,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE users SET mas_current = ? WHERE id = ?")
            .bind(mas_kmh)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to update user MAS: {e}")))?;

        Ok(())
    }

    async fn get_identity_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<IdentityProfile>, DomainError> {
        let row = sqlx::query(
            "SELECT user_id, name, age, email, gender, height_cm, weight_kg, updated_at
             FROM identity_profiles
             WHERE user_id = ?",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get identity profile: {e}")))?;

        row.map(|r| row_to_identity_profile(&r)).transpose()
    }

    async fn upsert_identity_profile(&self, profile: &IdentityProfile) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO identity_profiles (
                user_id, name, age, email, gender, height_cm, weight_kg, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                name = excluded.name,
                age = excluded.age,
                email = excluded.email,
                gender = excluded.gender,
                height_cm = excluded.height_cm,
                weight_kg = excluded.weight_kg,
                updated_at = excluded.updated_at",
        )
        .bind(profile.user_id.to_string())
        .bind(profile.name.as_deref())
        .bind(profile.age)
        .bind(profile.email.as_deref())
        .bind(profile.gender.as_deref())
        .bind(profile.height_cm)
        .bind(profile.weight_kg)
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to upsert identity profile: {e}")))?;

        Ok(())
    }

    async fn get_athlete_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AthleteProfile>, DomainError> {
        let row = sqlx::query(
            "SELECT user_id, goal_description, goal_date, goal_distance_km, goal_target_time_seconds,
                    goal_sport_type, goal_elevation_gain_m, additional_info, updated_at
             FROM athlete_profiles
             WHERE user_id = ?",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get athlete profile: {e}")))?;

        row.map(|r| row_to_athlete_profile(&r)).transpose()
    }

    async fn upsert_athlete_profile(&self, profile: &AthleteProfile) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO athlete_profiles (
                user_id, goal_description, goal_date, goal_distance_km, goal_target_time_seconds,
                goal_sport_type, goal_elevation_gain_m, additional_info, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                goal_description = excluded.goal_description,
                goal_date = excluded.goal_date,
                goal_distance_km = excluded.goal_distance_km,
                goal_target_time_seconds = excluded.goal_target_time_seconds,
                goal_sport_type = excluded.goal_sport_type,
                goal_elevation_gain_m = excluded.goal_elevation_gain_m,
                additional_info = excluded.additional_info,
                updated_at = excluded.updated_at",
        )
        .bind(profile.user_id.to_string())
        .bind(profile.goal_description.as_deref())
        .bind(profile.goal_date.as_deref())
        .bind(profile.goal_distance_km)
        .bind(profile.goal_target_time_seconds)
        .bind(profile.goal_sport_type.as_deref())
        .bind(profile.goal_elevation_gain_m)
        .bind(profile.additional_info.as_deref())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to upsert athlete profile: {e}")))?;

        Ok(())
    }

    async fn upsert_model_cost_tiers(&self, tiers: &[ModelCostTier]) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to start transaction: {e}")))?;

        sqlx::query("DELETE FROM model_cost_tiers")
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to clear model cost tiers: {e}")))?;

        for tier in tiers {
            sqlx::query(
                "INSERT INTO model_cost_tiers (model_id, model_name, category, computed_at) VALUES (?, ?, ?, ?)",
            )
            .bind(&tier.model_id)
            .bind(&tier.model_name)
            .bind(tier.category.as_str())
            .bind(tier.computed_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to upsert model cost tier: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    async fn list_model_cost_tiers(&self) -> Result<Vec<ModelCostTier>, DomainError> {
        let rows = sqlx::query(
            "SELECT model_id, model_name, category, computed_at
             FROM model_cost_tiers
             ORDER BY category, model_name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to list model cost tiers: {e}")))?;

        rows.iter().map(row_to_model_cost_tier).collect()
    }

    // -----------------------------------------------------------------------
    // AI Chats
    // -----------------------------------------------------------------------
    async fn create_ai_chat(&self, chat: &AiChat) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO ai_chats (id, user_id, training_id, source_insight_id, source_insight_cost, title, model, conversation_length, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(chat.id.to_string())
        .bind(chat.user_id.to_string())
        .bind(chat.training_id.map(|id| id.to_string()))
        .bind(chat.source_insight_id.map(|id| id.to_string()))
        .bind(chat.source_insight_cost)
        .bind(&chat.title)
        .bind(&chat.model)
        .bind(chat.conversation_length.map(|v| v as i32))
        .bind(chat.created_at.to_rfc3339())
        .bind(chat.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create ai chat: {e}")))?;

        Ok(())
    }

    async fn get_ai_chat(&self, id: Uuid, user_id: Uuid) -> Result<AiChat, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, training_id, source_insight_id, source_insight_cost, title, model, conversation_length, created_at, updated_at
             FROM ai_chats WHERE id = ? AND user_id = ?",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get ai chat: {e}")))?
        .ok_or_else(|| DomainError::NotFound(format!("AI chat {id} not found")))?;

        row_to_ai_chat(&row)
    }

    async fn get_ai_chat_by_source_insight(
        &self,
        user_id: Uuid,
        insight_id: Uuid,
    ) -> Result<Option<AiChat>, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, training_id, source_insight_id, source_insight_cost, title, model, conversation_length, created_at, updated_at
             FROM ai_chats
             WHERE user_id = ? AND source_insight_id = ?
             ORDER BY updated_at DESC
             LIMIT 1",
        )
        .bind(user_id.to_string())
        .bind(insight_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to get ai chat by source insight: {e}"))
        })?;

        row.map(|r| row_to_ai_chat(&r)).transpose()
    }

    async fn list_ai_chats(&self, user_id: Uuid) -> Result<Vec<AiChat>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, training_id, source_insight_id, source_insight_cost, title, model, conversation_length, created_at, updated_at
             FROM ai_chats WHERE user_id = ? ORDER BY updated_at DESC",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to list ai chats: {e}")))?;

        rows.iter().map(row_to_ai_chat).collect()
    }

    async fn delete_ai_chat(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query("DELETE FROM ai_chats WHERE id = ? AND user_id = ?")
            .bind(id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to delete ai chat: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("AI chat {id} not found")));
        }

        Ok(())
    }

    async fn update_ai_chat_title(
        &self,
        id: Uuid,
        user_id: Uuid,
        title: &str,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE ai_chats SET title = ?, updated_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(title)
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to update ai chat title: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("AI chat {id} not found")));
        }

        Ok(())
    }

    async fn touch_ai_chat(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE ai_chats SET updated_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to touch ai chat: {e}")))?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // AI Chat Messages
    // -----------------------------------------------------------------------
    async fn store_ai_chat_message(&self, msg: &AiChatMessage) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO ai_chat_messages (id, chat_id, role, content, prompt_tokens, completion_tokens, total_tokens, cost, context_label, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(msg.id.to_string())
        .bind(msg.chat_id.to_string())
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(msg.prompt_tokens as i32)
        .bind(msg.completion_tokens as i32)
        .bind(msg.total_tokens as i32)
        .bind(msg.cost)
        .bind(&msg.context_label)
        .bind(msg.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to store ai chat message: {e}")))?;

        Ok(())
    }

    async fn get_ai_chat_messages(&self, chat_id: Uuid) -> Result<Vec<AiChatMessage>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, chat_id, role, content, prompt_tokens, completion_tokens, total_tokens, cost, context_label, created_at
             FROM ai_chat_messages WHERE chat_id = ? ORDER BY created_at ASC",
        )
        .bind(chat_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get ai chat messages: {e}")))?;

        rows.iter().map(row_to_ai_chat_message).collect()
    }

    // -----------------------------------------------------------------------
    // Running Coach
    // -----------------------------------------------------------------------
    async fn get_or_create_running_coach_settings(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachSettings, DomainError> {
        let row = sqlx::query(
            "SELECT user_id, model, personality, consider_trail_runs_as_runs,
                    volume_weeks, last_workouts_count,
                    last_long_runs_count, last_races_count, new_activities_count,
                    normalizer_every_n_messages, created_at, updated_at
             FROM running_coach_settings
             WHERE user_id = ?",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get running coach settings: {e}")))?;

        if let Some(row) = row {
            return row_to_running_coach_settings(&row);
        }

        let settings = default_running_coach_settings(user_id);
        self.upsert_running_coach_settings(&settings).await?;
        Ok(settings)
    }

    async fn upsert_running_coach_settings(
        &self,
        settings: &RunningCoachSettings,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO running_coach_settings (
                user_id, model, personality, consider_trail_runs_as_runs,
                volume_weeks, last_workouts_count,
                last_long_runs_count, last_races_count, new_activities_count,
                normalizer_every_n_messages, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                model = excluded.model,
                personality = excluded.personality,
                consider_trail_runs_as_runs = excluded.consider_trail_runs_as_runs,
                volume_weeks = excluded.volume_weeks,
                last_workouts_count = excluded.last_workouts_count,
                last_long_runs_count = excluded.last_long_runs_count,
                last_races_count = excluded.last_races_count,
                new_activities_count = excluded.new_activities_count,
                normalizer_every_n_messages = excluded.normalizer_every_n_messages,
                updated_at = excluded.updated_at",
        )
        .bind(settings.user_id.to_string())
        .bind(&settings.model)
        .bind(&settings.personality)
        .bind(settings.consider_trail_runs_as_runs)
        .bind(settings.volume_weeks)
        .bind(settings.last_workouts_count)
        .bind(settings.last_long_runs_count)
        .bind(settings.last_races_count)
        .bind(settings.new_activities_count)
        .bind(settings.normalizer_every_n_messages)
        .bind(settings.created_at.to_rfc3339())
        .bind(settings.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to upsert running coach settings: {e}"))
        })?;

        Ok(())
    }

    async fn get_or_create_running_coach_memory(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachMemory, DomainError> {
        let row = sqlx::query(
            "SELECT user_id, data_json, message_count_since_normalization, updated_at
             FROM running_coach_memory
             WHERE user_id = ?",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get running coach memory: {e}")))?;

        if let Some(row) = row {
            return row_to_running_coach_memory(&row);
        }

        let memory = default_running_coach_memory(user_id);
        self.upsert_running_coach_memory(&memory).await?;
        Ok(memory)
    }

    async fn upsert_running_coach_memory(
        &self,
        memory: &RunningCoachMemory,
    ) -> Result<(), DomainError> {
        let data_json = serde_json::to_string(&memory.data).map_err(|e| {
            DomainError::Storage(format!("Failed to serialize running coach memory: {e}"))
        })?;

        sqlx::query(
            "INSERT INTO running_coach_memory (
                user_id, data_json, message_count_since_normalization, updated_at
            ) VALUES (?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                data_json = excluded.data_json,
                message_count_since_normalization = excluded.message_count_since_normalization,
                updated_at = excluded.updated_at",
        )
        .bind(memory.user_id.to_string())
        .bind(data_json)
        .bind(memory.message_count_since_normalization)
        .bind(memory.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to upsert running coach memory: {e}")))?;

        Ok(())
    }

    async fn get_or_create_running_coach_state(
        &self,
        user_id: Uuid,
    ) -> Result<RunningCoachState, DomainError> {
        let row = sqlx::query(
            "SELECT user_id, last_interaction_at, last_seen_activity_start_date, updated_at
             FROM running_coach_state
             WHERE user_id = ?",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get running coach state: {e}")))?;

        if let Some(row) = row {
            return row_to_running_coach_state(&row);
        }

        let state = default_running_coach_state(user_id);
        self.upsert_running_coach_state(&state).await?;
        Ok(state)
    }

    async fn upsert_running_coach_state(
        &self,
        state: &RunningCoachState,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO running_coach_state (
                user_id, last_interaction_at, last_seen_activity_start_date, updated_at
            ) VALUES (?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                last_interaction_at = excluded.last_interaction_at,
                last_seen_activity_start_date = excluded.last_seen_activity_start_date,
                updated_at = excluded.updated_at",
        )
        .bind(state.user_id.to_string())
        .bind(state.last_interaction_at.map(|v| v.to_rfc3339()))
        .bind(state.last_seen_activity_start_date.map(|v| v.to_rfc3339()))
        .bind(state.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to upsert running coach state: {e}")))?;

        Ok(())
    }

    async fn list_running_coach_messages(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RunningCoachMessage>, DomainError> {
        let safe_limit = limit.max(1).min(500);
        let rows = sqlx::query(
            "SELECT id, user_id, role, content, prompt_tokens, completion_tokens, total_tokens, cost, created_at
             FROM running_coach_messages
             WHERE user_id = ?
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(user_id.to_string())
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to list running coach messages: {e}")))?;

        let mut messages: Vec<RunningCoachMessage> = rows
            .iter()
            .map(row_to_running_coach_message)
            .collect::<Result<Vec<_>, _>>()?;
        messages.reverse();
        Ok(messages)
    }

    async fn store_running_coach_message(
        &self,
        msg: &RunningCoachMessage,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO running_coach_messages (
                id, user_id, role, content, prompt_tokens, completion_tokens, total_tokens, cost, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(msg.id.to_string())
        .bind(msg.user_id.to_string())
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(msg.prompt_tokens as i32)
        .bind(msg.completion_tokens as i32)
        .bind(msg.total_tokens as i32)
        .bind(msg.cost)
        .bind(msg.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to store running coach message: {e}")))?;

        Ok(())
    }

    async fn clear_running_coach_data(&self, user_id: Uuid) -> Result<(), DomainError> {
        let user_id = user_id.to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to start transaction: {e}")))?;

        sqlx::query("DELETE FROM running_coach_messages WHERE user_id = ?")
            .bind(&user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                DomainError::Storage(format!("Failed to clear running coach messages: {e}"))
            })?;

        sqlx::query("DELETE FROM running_coach_memory WHERE user_id = ?")
            .bind(&user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                DomainError::Storage(format!("Failed to clear running coach memory: {e}"))
            })?;

        sqlx::query("DELETE FROM running_coach_state WHERE user_id = ?")
            .bind(&user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                DomainError::Storage(format!("Failed to clear running coach state: {e}"))
            })?;

        sqlx::query("DELETE FROM running_coach_settings WHERE user_id = ?")
            .bind(&user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                DomainError::Storage(format!("Failed to clear running coach settings: {e}"))
            })?;

        tx.commit().await.map_err(|e| {
            DomainError::Storage(format!("Failed to commit running coach reset: {e}"))
        })?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Insight lookup
    // -----------------------------------------------------------------------
    async fn get_training_insight_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<TrainingInsight, DomainError> {
        let row = sqlx::query(
            "SELECT id, training_id, user_id, prompt_type, display_label, full_prompt, response, model, cost, created_at
             FROM training_insights WHERE id = ? AND user_id = ?",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get training insight: {e}")))?
        .ok_or_else(|| DomainError::NotFound(format!("Training insight {id} not found")))?;

        row_to_training_insight(&row)
    }

    // -----------------------------------------------------------------------
    // Stats
    // -----------------------------------------------------------------------
    async fn get_running_stats(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        include_interval_count: bool,
    ) -> Result<RunningStats, DomainError> {
        let mut sql = String::from(
            "SELECT COALESCE(SUM(distance), 0.0) as total_distance,
                    COALESCE(SUM(moving_time), 0) as total_time,
                    COALESCE(SUM(total_elevation_gain), 0.0) as total_elevation,
                    COUNT(*) as activity_count
             FROM activities
             WHERE user_id = ? AND sport_type = 'Run'",
        );
        let mut binds: Vec<String> = vec![user_id.to_string()];

        if let Some(ref f) = from {
            sql.push_str(" AND start_date >= ?");
            binds.push(f.to_rfc3339());
        }
        if let Some(ref t) = to {
            sql.push_str(" AND start_date < ?");
            binds.push(t.to_rfc3339());
        }

        let mut query = sqlx::query(&sql);
        for b in &binds {
            query = query.bind(b);
        }

        let row = query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to get running stats: {e}")))?;

        let total_distance: f64 = row.get("total_distance");
        let total_time: i64 = row.get("total_time");
        let total_elevation: f64 = row.get("total_elevation");
        let activity_count: i32 = row.get("activity_count");

        let avg_speed_mps = if total_time > 0 {
            Some(total_distance / total_time as f64)
        } else {
            None
        };

        let interval_count = if include_interval_count {
            let mut isql = String::from(
                "SELECT COUNT(*) as cnt FROM activities WHERE user_id = ? AND sport_type = 'Run' AND tag = 'intervals'",
            );
            let mut ibinds: Vec<String> = vec![user_id.to_string()];

            if let Some(ref f) = from {
                isql.push_str(" AND start_date >= ?");
                ibinds.push(f.to_rfc3339());
            }
            if let Some(ref t) = to {
                isql.push_str(" AND start_date < ?");
                ibinds.push(t.to_rfc3339());
            }

            let mut iquery = sqlx::query(&isql);
            for b in &ibinds {
                iquery = iquery.bind(b);
            }

            let irow = iquery
                .fetch_one(&self.pool)
                .await
                .map_err(|e| DomainError::Storage(format!("Failed to get interval count: {e}")))?;

            let cnt: i32 = irow.get("cnt");
            Some(cnt as i64)
        } else {
            None
        };

        Ok(RunningStats {
            total_distance_m: total_distance,
            total_time_s: total_time,
            total_elevation_m: total_elevation,
            avg_speed_mps,
            activity_count: activity_count as i64,
            interval_count,
        })
    }

    // -----------------------------------------------------------------------
    // Quota
    // -----------------------------------------------------------------------

    async fn get_user_quota(&self, user_id: Uuid) -> Result<f64, DomainError> {
        let row = sqlx::query("SELECT quota_balance_usd FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::NotFound(format!("User not found: {e}")))?;

        Ok(row.get("quota_balance_usd"))
    }

    async fn deduct_quota(&self, user_id: Uuid, amount: f64) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE users SET quota_balance_usd = quota_balance_usd - ? WHERE id = ? AND quota_balance_usd >= ?"
        )
        .bind(amount)
        .bind(user_id.to_string())
        .bind(amount)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to deduct quota: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::QuotaExhausted(
                "Insufficient quota balance".into(),
            ));
        }
        Ok(())
    }

    async fn add_quota(&self, user_id: Uuid, amount: f64) -> Result<(), DomainError> {
        sqlx::query("UPDATE users SET quota_balance_usd = quota_balance_usd + ? WHERE id = ?")
            .bind(amount)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to add quota: {e}")))?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Quota requests
    // -----------------------------------------------------------------------

    async fn create_quota_request(&self, req: &QuotaRequest) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO quota_requests (id, user_id, status, requested_at, resolved_at, granted_amount_usd)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(req.id.to_string())
        .bind(req.user_id.to_string())
        .bind(req.status.as_str())
        .bind(req.requested_at.to_rfc3339())
        .bind(req.resolved_at.map(|dt| dt.to_rfc3339()))
        .bind(req.granted_amount_usd)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create quota request: {e}")))?;

        Ok(())
    }

    async fn get_quota_request(&self, id: Uuid) -> Result<QuotaRequest, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, status, requested_at, resolved_at, granted_amount_usd
             FROM quota_requests WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|_| DomainError::NotFound("Quota request not found".into()))?;

        row_to_quota_request(&row)
    }

    async fn get_pending_quota_requests(&self) -> Result<Vec<QuotaRequest>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, status, requested_at, resolved_at, granted_amount_usd
             FROM quota_requests WHERE status = 'pending' ORDER BY requested_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get quota requests: {e}")))?;

        rows.iter().map(row_to_quota_request).collect()
    }

    async fn get_user_quota_requests(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<QuotaRequest>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, status, requested_at, resolved_at, granted_amount_usd
             FROM quota_requests WHERE user_id = ? ORDER BY requested_at DESC",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get user quota requests: {e}")))?;

        rows.iter().map(row_to_quota_request).collect()
    }

    async fn resolve_quota_request(
        &self,
        id: Uuid,
        status: QuotaRequestStatus,
        granted_amount_usd: Option<f64>,
    ) -> Result<(), DomainError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE quota_requests SET status = ?, resolved_at = ?, granted_amount_usd = ? WHERE id = ?"
        )
        .bind(status.as_str())
        .bind(&now)
        .bind(granted_amount_usd)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to resolve quota request: {e}")))?;

        Ok(())
    }

    async fn create_invite_code(&self, invite_code: &InviteCode) -> Result<(), DomainError> {
        let created_by_user_id: Option<String> = invite_code
            .created_by_user_id
            .as_ref()
            .map(|id| id.to_string());
        let expires_at: Option<String> = invite_code.expires_at.as_ref().map(|dt| dt.to_rfc3339());
        let used_at: Option<String> = invite_code.used_at.as_ref().map(|dt| dt.to_rfc3339());
        let revoked_at: Option<String> = invite_code.revoked_at.as_ref().map(|dt| dt.to_rfc3339());

        sqlx::query(
            "INSERT INTO invite_codes (
                id, code_hash, created_by_user_id, created_for, created_at, expires_at, used_at,
                used_by_strava_athlete_id, revoked_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(invite_code.id.to_string())
        .bind(&invite_code.code_hash)
        .bind(created_by_user_id)
        .bind(&invite_code.created_for)
        .bind(invite_code.created_at.to_rfc3339())
        .bind(expires_at)
        .bind(used_at)
        .bind(invite_code.used_by_strava_athlete_id)
        .bind(revoked_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create invite code: {e}")))?;

        Ok(())
    }

    async fn list_invite_codes(&self) -> Result<Vec<InviteCode>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, code_hash, created_by_user_id, created_for, created_at, expires_at, used_at, used_by_strava_athlete_id, revoked_at
             FROM invite_codes
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to list invite codes: {e}")))?;

        rows.iter().map(row_to_invite_code).collect()
    }

    async fn get_invite_code_by_hash(&self, code_hash: &str) -> Result<InviteCode, DomainError> {
        let row = sqlx::query(
            "SELECT id, code_hash, created_by_user_id, created_for, created_at, expires_at, used_at, used_by_strava_athlete_id, revoked_at
             FROM invite_codes
             WHERE code_hash = ?",
        )
        .bind(code_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get invite code: {e}")))?
        .ok_or_else(|| DomainError::NotFound("Invite code not found".into()))?;

        row_to_invite_code(&row)
    }

    async fn revoke_invite_code(&self, id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE invite_codes
             SET revoked_at = COALESCE(revoked_at, ?)
             WHERE id = ? AND used_at IS NULL",
        )
        .bind(&now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to revoke invite code: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::BadRequest(
                "Invite code cannot be revoked (already used or missing)".into(),
            ));
        }

        Ok(())
    }

    async fn consume_invite_code(
        &self,
        code_hash: &str,
        used_by_strava_athlete_id: i64,
    ) -> Result<(), DomainError> {
        let now = Utc::now();
        let now_rfc3339 = now.to_rfc3339();

        let result = sqlx::query(
            "UPDATE invite_codes
             SET used_at = ?, used_by_strava_athlete_id = ?
             WHERE code_hash = ?
               AND used_at IS NULL
               AND revoked_at IS NULL
               AND (expires_at IS NULL OR expires_at > ?)",
        )
        .bind(&now_rfc3339)
        .bind(used_by_strava_athlete_id)
        .bind(code_hash)
        .bind(&now_rfc3339)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to consume invite code: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::BadRequest(
                "Invite code is invalid, expired, revoked, or already used".into(),
            ));
        }

        Ok(())
    }
}

impl SqliteStorage {
    pub async fn purge_old_streams(&self, max_age_days: i64) -> Result<u64, DomainError> {
        let cutoff = Utc::now().timestamp() - max_age_days * 86_400;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to begin transaction: {e}")))?;

        let res = sqlx::query(
            "DELETE FROM activity_streams
             WHERE activity_id IN (
                 SELECT id FROM activities
                 WHERE streams_loaded != 0 AND streams_loaded < ?
             )",
        )
        .bind(cutoff)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to purge old streams: {e}")))?;

        sqlx::query(
            "UPDATE activities
             SET streams_loaded = 0
             WHERE streams_loaded != 0 AND streams_loaded < ?",
        )
        .bind(cutoff)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to mark expired streams: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to commit purge: {e}")))?;

        Ok(res.rows_affected())
    }

    /// Development helper: remove all application data and rebuild schema.
    pub async fn delete_all_data(&self) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to begin transaction: {e}")))?;

        // Drop child tables first, then parent tables.
        for table in [
            "running_coach_messages",
            "running_coach_state",
            "running_coach_memory",
            "running_coach_settings",
            "ai_chat_messages",
            "ai_chats",
            "training_insights",
            "activity_interval_results",
            "activity_laps",
            "activity_streams",
            "activities",
            "trainings",
            "quota_requests",
            "invite_codes",
            "model_cost_tiers",
            "strava_tokens",
            "users",
        ] {
            let query = format!("DROP TABLE IF EXISTS {table}");
            sqlx::query(&query)
                .execute(&mut *tx)
                .await
                .map_err(|e| DomainError::Storage(format!("Failed to drop {table}: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to commit delete-all-data: {e}")))?;

        Self::initialize_schema(&self.pool).await?;

        Ok(())
    }
}
