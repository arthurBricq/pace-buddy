use actix_web::{web, HttpResponse};
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::helpers::activity_sync_helper::sync_user_activities;
use crate::helpers::strava_data_helper::{
    fetch_polyline, fetch_streams_from_strava, load_and_cache_streams,
};
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{ActivityTag, DomainError};

#[derive(Deserialize)]
pub struct SyncRequest {
    pub after: Option<i64>,
    pub before: Option<i64>,
}

pub async fn sync_activities(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<SyncRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!(
        "POST /activities/sync user={} after={:?} before={:?}",
        user.user_id,
        body.after,
        body.before
    );

    if !state.try_begin_activities_sync(user.user_id).await {
        log::info!(
            "Sync already running for user {}, returning early",
            user.user_id
        );
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "synced": 0,
            "already_running": true,
        })));
    }

    let result = sync_user_activities(&state, user.user_id, body.after, body.before).await;
    let total = match result {
        Ok(total) => {
            state.mark_activities_sync_finished(user.user_id).await;
            total
        }
        Err(e) => {
            state
                .mark_activities_sync_failed(user.user_id, e.to_string())
                .await;
            return Err(AppError(e));
        }
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "synced": total,
    })))
}

pub async fn sync_status(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let (status, error) = state.get_activities_sync_status(user.user_id).await;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": status,
        "error": error,
    })))
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_activities(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, AppError> {
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

    log::debug!(
        "GET /activities user={} limit={limit} offset={offset}",
        user.user_id
    );

    let activities = state
        .storage
        .get_activities(user.user_id, limit, offset)
        .await?;

    log::debug!("Returning {} activities", activities.len());

    Ok(HttpResponse::Ok().json(activities))
}

pub async fn get_activity(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
    log::info!("GET /activities/{activity_id} user={}", user.user_id);

    let mut activity = state
        .storage
        .get_activity(activity_id, user.user_id)
        .await?;

    // --- Streams: serve from cache, re-fetch if purged/empty ---
    let mut streams = state
        .storage
        .get_streams(activity_id)
        .await
        .unwrap_or_default();
    if streams.is_empty() {
        match load_and_cache_streams(&state, &activity).await {
            Ok(s) => streams = s,
            Err(e) => log::warn!("Failed to load streams for activity {activity_id}: {e}"),
        }
    }

    // --- Polyline: always fetch on demand (never persisted) ---
    if let Ok(polyline) = fetch_polyline(&state, &activity).await {
        activity.summary_polyline = polyline;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "activity": activity,
        "streams": streams,
    })))
}

#[derive(Deserialize)]
pub struct TagUpdateRequest {
    pub tag: String,
}

pub async fn get_intervals(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
    log::info!(
        "GET /activities/{activity_id}/intervals user={}",
        user.user_id
    );

    let activity = state
        .storage
        .get_activity(activity_id, user.user_id)
        .await?;

    // Try cached streams first, fall back to Strava
    let mut streams = state
        .storage
        .get_streams(activity_id)
        .await
        .unwrap_or_default();
    if streams.is_empty() {
        streams = fetch_streams_from_strava(&state, &activity).await?;
    }
    let config = intervals::types::IntervalConfig::default();
    match state.parse_intervals(&streams, &config, None).await {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => {
            log::warn!("Interval parsing failed for {activity_id}: {e}");
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "segments": [],
                "reps": [],
                "is_interval_workout": false,
                "interval_score": 0.0,
                "threshold_speed_mps": 0.0,
                "cluster_low_mps": 0.0,
                "cluster_high_mps": 0.0,
            })))
        }
    }
}

pub async fn update_tag(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<TagUpdateRequest>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
    log::info!(
        "PATCH /activities/{activity_id}/tag user={} tag={}",
        user.user_id,
        body.tag
    );

    let tag: ActivityTag = body
        .tag
        .parse()
        .map_err(|e: String| DomainError::BadRequest(e))?;

    state
        .storage
        .update_activity_tag(activity_id, user.user_id, tag)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}
