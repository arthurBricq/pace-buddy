use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{Activity, ActivityStream, ActivityTag, DomainError, RunningStats, StravaToken, Training, TrainingInsight, User};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::Row;
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

        // Create tables directly (no migrations for now)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT UNIQUE NOT NULL,
                display_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                mas_current REAL
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create users table: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS passkeys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL REFERENCES users(id),
                passkey_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create passkeys table: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS strava_tokens (
                user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id),
                strava_athlete_id INTEGER NOT NULL,
                access_token TEXT NOT NULL,
                refresh_token TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )"
        )
        .execute(&pool)
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
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create activities table: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_activities_user_date ON activities(user_id, start_date DESC)"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create activities index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS activity_streams (
                activity_id TEXT NOT NULL REFERENCES activities(id),
                stream_type TEXT NOT NULL,
                data_json TEXT NOT NULL,
                PRIMARY KEY (activity_id, stream_type)
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create activity_streams table: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS trainings (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                name TEXT NOT NULL,
                description TEXT,
                start_date TEXT,
                end_date TEXT,
                race_goal TEXT,
                created_at TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create trainings table: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_trainings_user ON trainings(user_id)"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create trainings index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS training_activities (
                training_id TEXT NOT NULL REFERENCES trainings(id) ON DELETE CASCADE,
                activity_id TEXT NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
                PRIMARY KEY (training_id, activity_id)
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training_activities table: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_training_activities_training ON training_activities(training_id)"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training_activities index: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_training_activities_activity ON training_activities(activity_id)"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training_activities index: {e}")))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS training_insights (
                id TEXT PRIMARY KEY NOT NULL,
                training_id TEXT NOT NULL REFERENCES trainings(id) ON DELETE CASCADE,
                user_id TEXT NOT NULL REFERENCES users(id),
                prompt_type TEXT NOT NULL,
                display_label TEXT NOT NULL,
                full_prompt TEXT NOT NULL,
                response TEXT NOT NULL,
                created_at TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training_insights table: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_training_insights_training ON training_insights(training_id, user_id)"
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training_insights index: {e}")))?;

        Ok(Self { pool })
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
    let created_at: String = row.get("created_at");
    let mas_current: Option<f64> = row.get("mas_current");

    Ok(User {
        id: parse_uuid(&id)?,
        username,
        display_name,
        created_at: parse_datetime(&created_at)?,
        mas_current,
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
    let race_goal: Option<String> = row.get("race_goal");
    let created_at: String = row.get("created_at");

    Ok(Training {
        id: parse_uuid(&id)?,
        user_id: parse_uuid(&user_id)?,
        name,
        description,
        start_date: start_date.map(|s| parse_datetime(&s)).transpose()?,
        end_date: end_date.map(|s| parse_datetime(&s)).transpose()?,
        race_goal,
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

fn row_to_training_insight(row: &SqliteRow) -> Result<TrainingInsight, DomainError> {
    let id: String = row.get("id");
    let training_id: String = row.get("training_id");
    let user_id: String = row.get("user_id");
    let prompt_type: String = row.get("prompt_type");
    let display_label: String = row.get("display_label");
    let full_prompt: String = row.get("full_prompt");
    let response: String = row.get("response");
    let created_at: String = row.get("created_at");

    Ok(TrainingInsight {
        id: parse_uuid(&id)?,
        training_id: parse_uuid(&training_id)?,
        user_id: parse_uuid(&user_id)?,
        prompt_type,
        display_label,
        full_prompt,
        response,
        created_at: parse_datetime(&created_at)?,
    })
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
            "INSERT INTO users (id, username, display_name, created_at, mas_current) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(user.id.to_string())
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(user.created_at.to_rfc3339())
        .bind(user.mas_current)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create user: {e}")))?;

        Ok(())
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<User, DomainError> {
        let row = sqlx::query("SELECT id, username, display_name, created_at, mas_current FROM users WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to get user: {e}")))?
            .ok_or_else(|| DomainError::NotFound(format!("User {id} not found")))?;

        row_to_user(&row)
    }

    async fn list_users(&self) -> Result<Vec<User>, DomainError> {
        let rows = sqlx::query("SELECT id, username, display_name, created_at, mas_current FROM users ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to list users: {e}")))?;

        rows.iter().map(row_to_user).collect()
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError> {
        let row = sqlx::query(
            "SELECT id, username, display_name, created_at, mas_current FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to get user: {e}")))?
        .ok_or_else(|| DomainError::NotFound(format!("User '{username}' not found")))?;

        row_to_user(&row)
    }

    // -----------------------------------------------------------------------
    // Passkeys
    // -----------------------------------------------------------------------
    async fn store_passkey(&self, user_id: Uuid, passkey_json: &str) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO passkeys (user_id, passkey_json) VALUES (?, ?)")
            .bind(user_id.to_string())
            .bind(passkey_json)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to store passkey: {e}")))?;

        Ok(())
    }

    async fn get_passkeys_for_user(&self, user_id: Uuid) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query("SELECT passkey_json FROM passkeys WHERE user_id = ?")
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to get passkeys: {e}")))?;

        let passkeys: Vec<String> = rows.iter().map(|r| r.get("passkey_json")).collect();
        Ok(passkeys)
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
            .bind(activity.streams_fetched_at.map(|dt| dt.timestamp()).unwrap_or(0i64))
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
        let result =
            sqlx::query("UPDATE activities SET streams_loaded = ? WHERE id = ?")
                .bind(Utc::now().timestamp())
                .bind(activity_id.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    DomainError::Storage(format!("Failed to mark streams fetched: {e}"))
                })?;

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

        for stream in streams {
            // Never persist GPS data
            if stream.stream_type == domain::StreamType::LatLng {
                continue;
            }
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

    // -----------------------------------------------------------------------
    // Trainings
    // -----------------------------------------------------------------------
    async fn create_training(&self, training: &Training) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO trainings (id, user_id, name, description, start_date, end_date, race_goal, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(training.id.to_string())
        .bind(training.user_id.to_string())
        .bind(&training.name)
        .bind(&training.description)
        .bind(training.start_date.map(|d| d.to_rfc3339()))
        .bind(training.end_date.map(|d| d.to_rfc3339()))
        .bind(&training.race_goal)
        .bind(training.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training: {e}")))?;

        Ok(())
    }

    async fn get_training(&self, id: Uuid, user_id: Uuid) -> Result<Training, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, name, description, start_date, end_date, race_goal, created_at
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
            "SELECT id, user_id, name, description, start_date, end_date, race_goal, created_at
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
        race_goal: Option<String>,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE trainings SET name = ?, description = ?, start_date = ?, end_date = ?, race_goal = ? WHERE id = ? AND user_id = ?",
        )
        .bind(&name)
        .bind(&description)
        .bind(start_date.map(|d| d.to_rfc3339()))
        .bind(end_date.map(|d| d.to_rfc3339()))
        .bind(&race_goal)
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

    async fn add_activity_to_training(
        &self,
        training_id: Uuid,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        // Verify training belongs to user
        let _training = self.get_training(training_id, user_id).await?;

        // Verify activity belongs to user and is tagged as intervals
        let activity = self.get_activity(activity_id, user_id).await?;
        if activity.tag != ActivityTag::Intervals {
            return Err(DomainError::BadRequest(
                "Only interval-tagged activities can be added to trainings".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO training_activities (training_id, activity_id)
             VALUES (?, ?)
             ON CONFLICT(training_id, activity_id) DO NOTHING",
        )
        .bind(training_id.to_string())
        .bind(activity_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to add activity to training: {e}"))
        })?;

        Ok(())
    }

    async fn remove_activity_from_training(
        &self,
        training_id: Uuid,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        // Verify training belongs to user
        let _training = self.get_training(training_id, user_id).await?;

        let result = sqlx::query(
            "DELETE FROM training_activities
             WHERE training_id = ? AND activity_id = ?",
        )
        .bind(training_id.to_string())
        .bind(activity_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to remove activity from training: {e}"))
        })?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(
                "Activity not found in training".into(),
            ));
        }

        Ok(())
    }

    async fn get_training_activities(
        &self,
        training_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Activity>, DomainError> {
        // Verify training belongs to user
        let _training = self.get_training(training_id, user_id).await?;

        let rows = sqlx::query(
            "SELECT a.id, a.user_id, a.strava_id, a.name, a.sport_type, a.start_date,
                    a.elapsed_time, a.moving_time, a.distance, a.total_elevation_gain,
                    a.average_speed, a.max_speed, a.average_heartrate, a.max_heartrate,
                    a.average_cadence, a.average_watts, a.calories, a.tag, a.summary_polyline,
                    a.workout_type, a.streams_loaded, a.created_at
             FROM activities a
             INNER JOIN training_activities ta ON a.id = ta.activity_id
             WHERE ta.training_id = ? AND a.user_id = ?
             ORDER BY a.start_date DESC",
        )
        .bind(training_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Storage(format!("Failed to get training activities: {e}"))
        })?;

        rows.iter().map(row_to_activity).collect()
    }

    async fn get_activity_trainings(
        &self,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Training>, DomainError> {
        // Verify activity belongs to user
        let _activity = self.get_activity(activity_id, user_id).await?;

        let rows = sqlx::query(
            "SELECT t.id, t.user_id, t.name, t.description, t.start_date, t.end_date, t.race_goal, t.created_at
             FROM trainings t
             INNER JOIN training_activities ta ON t.id = ta.training_id
             WHERE ta.activity_id = ? AND t.user_id = ?
             ORDER BY t.created_at DESC",
        )
        .bind(activity_id.to_string())
        .bind(user_id.to_string())
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
            "INSERT INTO training_insights (id, training_id, user_id, prompt_type, display_label, full_prompt, response, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(insight.id.to_string())
        .bind(insight.training_id.to_string())
        .bind(insight.user_id.to_string())
        .bind(&insight.prompt_type)
        .bind(&insight.display_label)
        .bind(&insight.full_prompt)
        .bind(&insight.response)
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
            "SELECT id, training_id, user_id, prompt_type, display_label, full_prompt, response, created_at
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
    async fn update_user_mas(&self, user_id: Uuid, mas_mps: Option<f64>) -> Result<(), DomainError> {
        sqlx::query("UPDATE users SET mas_current = ? WHERE id = ?")
            .bind(mas_mps)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to update user MAS: {e}")))?;

        Ok(())
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
}
