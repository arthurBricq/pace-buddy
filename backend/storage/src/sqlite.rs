use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{Activity, ActivityStream, ActivityTag, DomainError, StravaToken, Training, User};
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

        // Run migrations: execute each statement separately since SQLite
        // does not support multiple statements in a single sqlx::query() call.
        let migration_sql = include_str!("migrations/001_initial.sql");
        for statement in migration_sql.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed)
                    .execute(&pool)
                    .await
                    .map_err(|e| {
                        DomainError::Storage(format!("Migration failed: {e}"))
                    })?;
            }
        }

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
    let streams_loaded: i32 = row.get("streams_loaded");
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
        streams_loaded: streams_loaded != 0,
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
    let created_at: String = row.get("created_at");

    Ok(Training {
        id: parse_uuid(&id)?,
        user_id: parse_uuid(&user_id)?,
        name,
        description,
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
        let row = sqlx::query("SELECT id, username, display_name, created_at FROM users WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to get user: {e}")))?
            .ok_or_else(|| DomainError::NotFound(format!("User {id} not found")))?;

        row_to_user(&row)
    }

    async fn list_users(&self) -> Result<Vec<User>, DomainError> {
        let rows = sqlx::query("SELECT id, username, display_name, created_at FROM users ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("Failed to list users: {e}")))?;

        rows.iter().map(row_to_user).collect()
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError> {
        let row = sqlx::query(
            "SELECT id, username, display_name, created_at FROM users WHERE username = ?",
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
            .bind(&activity.summary_polyline)
            .bind(activity.workout_type)
            .bind(if activity.streams_loaded { 1i32 } else { 0i32 })
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

    async fn mark_streams_loaded(&self, activity_id: Uuid) -> Result<(), DomainError> {
        let result =
            sqlx::query("UPDATE activities SET streams_loaded = 1 WHERE id = ?")
                .bind(activity_id.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    DomainError::Storage(format!("Failed to mark streams loaded: {e}"))
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
            "INSERT INTO trainings (id, user_id, name, description, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(training.id.to_string())
        .bind(training.user_id.to_string())
        .bind(&training.name)
        .bind(&training.description)
        .bind(training.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("Failed to create training: {e}")))?;

        Ok(())
    }

    async fn get_training(&self, id: Uuid, user_id: Uuid) -> Result<Training, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, name, description, created_at
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
            "SELECT id, user_id, name, description, created_at
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
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE trainings SET name = ?, description = ? WHERE id = ? AND user_id = ?",
        )
        .bind(&name)
        .bind(&description)
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
            "SELECT t.id, t.user_id, t.name, t.description, t.created_at
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
}
