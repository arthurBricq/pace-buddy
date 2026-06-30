use chrono::{DateTime, Utc};
use domain::{DomainError, StravaToken};
use std::sync::Arc;
use storage::{SqliteStorage, Storage};
use strava_client::StravaClient;
use uuid::Uuid;

/// Get a valid (non-expired) access token for the user.
/// Automatically refreshes if expired.
pub async fn get_valid_access_token(
    storage: &Arc<SqliteStorage>,
    strava_client: &Arc<StravaClient>,
    user_id: Uuid,
) -> Result<String, DomainError> {
    let token = storage.get_strava_token(user_id).await?;

    if !token.is_expired() {
        return Ok(token.access_token);
    }

    let refreshed = strava_client.refresh_token(&token.refresh_token).await?;

    let new_token = StravaToken {
        user_id,
        strava_athlete_id: token.strava_athlete_id,
        access_token: refreshed.access_token.clone(),
        refresh_token: refreshed.refresh_token,
        expires_at: DateTime::<Utc>::from_timestamp(refreshed.expires_at, 0)
            .unwrap_or_else(Utc::now),
    };

    storage.upsert_strava_token(&new_token).await?;

    Ok(refreshed.access_token)
}
