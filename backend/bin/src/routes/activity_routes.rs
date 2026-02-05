use actix_web::{web, HttpResponse};
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::helpers::strava_token_helper::get_valid_access_token;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{ActivityTag, DomainError};
use strava_client::conversions::{strava_activity_to_domain, strava_streams_to_domain};

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
    let access_token =
        get_valid_access_token(&state.storage, &state.strava_client, user.user_id).await?;

    let mut all_activities = Vec::new();
    let mut page = 1u32;
    let per_page = 200u32;

    loop {
        let strava_activities = state
            .strava_client
            .get_activities(&access_token, page, per_page, body.after, body.before)
            .await?;

        let count = strava_activities.len();
        let domain_activities: Vec<_> = strava_activities
            .iter()
            .map(|sa| strava_activity_to_domain(sa, user.user_id))
            .collect();

        all_activities.extend(domain_activities);

        if (count as u32) < per_page {
            break;
        }
        page += 1;
    }

    let total = all_activities.len();
    state.storage.upsert_activities(&all_activities).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "synced": total,
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

    let activities = state
        .storage
        .get_activities(user.user_id, limit, offset)
        .await?;

    Ok(HttpResponse::Ok().json(activities))
}

pub async fn get_activity(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
    let activity = state
        .storage
        .get_activity(activity_id, user.user_id)
        .await?;

    // Lazy-load streams if not yet loaded
    let streams = if !activity.streams_loaded {
        match load_and_store_streams(&state, &activity).await {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to load streams for activity {}: {}", activity_id, e);
                vec![]
            }
        }
    } else {
        state.storage.get_streams(activity_id).await?
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "activity": activity,
        "streams": streams,
    })))
}

async fn load_and_store_streams(
    state: &web::Data<AppState>,
    activity: &domain::Activity,
) -> Result<Vec<domain::ActivityStream>, DomainError> {
    let access_token = get_valid_access_token(
        &state.storage,
        &state.strava_client,
        activity.user_id,
    )
    .await?;

    let strava_streams = state
        .strava_client
        .get_activity_streams(&access_token, activity.strava_id)
        .await?;

    let streams = strava_streams_to_domain(strava_streams, activity.id);

    if !streams.is_empty() {
        state.storage.store_streams(&streams).await?;
    }
    state.storage.mark_streams_loaded(activity.id).await?;

    Ok(streams)
}

#[derive(Deserialize)]
pub struct TagUpdateRequest {
    pub tag: String,
}

pub async fn update_tag(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<TagUpdateRequest>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
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
