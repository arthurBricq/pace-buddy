use actix_web::web;
use domain::DomainError;
use storage::Storage;
use strava_client::conversions::{strava_laps_to_domain, strava_streams_to_domain};

use crate::helpers::strava_token_helper::get_valid_access_token;
use crate::state::AppState;

/// Fetch streams from Strava, cache non-GPS streams in DB, return all (including latlng).
pub async fn load_and_cache_streams(
    state: &web::Data<AppState>,
    activity: &domain::Activity,
) -> Result<Vec<domain::ActivityStream>, DomainError> {
    let all_streams = fetch_streams_from_strava(state, activity).await?;

    // store_streams filters out LatLng in the storage layer
    if !all_streams.is_empty() {
        state.storage.store_streams(&all_streams).await?;
    }
    state.storage.mark_streams_fetched(activity.id).await?;

    Ok(all_streams)
}

/// Fetch the polyline on demand from Strava (never persisted).
pub async fn fetch_polyline(
    state: &web::Data<AppState>,
    activity: &domain::Activity,
) -> Result<Option<String>, DomainError> {
    let token =
        get_valid_access_token(&state.storage, &state.strava_client, activity.user_id).await?;
    let strava_activity = state
        .strava_client
        .get_activity(&token, activity.strava_id)
        .await?;
    Ok(strava_activity.map.and_then(|m| m.summary_polyline))
}

/// Fetch only streams from Strava.
pub async fn fetch_streams_from_strava(
    state: &web::Data<AppState>,
    activity: &domain::Activity,
) -> Result<Vec<domain::ActivityStream>, DomainError> {
    let access_token =
        get_valid_access_token(&state.storage, &state.strava_client, activity.user_id).await?;

    let strava_streams = state
        .strava_client
        .get_activity_streams(&access_token, activity.strava_id)
        .await?;

    Ok(strava_streams_to_domain(strava_streams, activity.id))
}

/// Fetch laps from Strava and cache them in DB.
pub async fn load_and_cache_laps(
    state: &web::Data<AppState>,
    activity: &domain::Activity,
) -> Result<Vec<domain::ActivityLap>, DomainError> {
    let laps = fetch_laps_from_strava(state, activity).await?;
    if !laps.is_empty() {
        state.storage.store_laps(&laps).await?;
    }
    Ok(laps)
}

/// Fetch only laps from Strava.
pub async fn fetch_laps_from_strava(
    state: &web::Data<AppState>,
    activity: &domain::Activity,
) -> Result<Vec<domain::ActivityLap>, DomainError> {
    let access_token =
        get_valid_access_token(&state.storage, &state.strava_client, activity.user_id).await?;

    let strava_laps = state
        .strava_client
        .get_activity_laps(&access_token, activity.strava_id)
        .await?;

    Ok(strava_laps_to_domain(strava_laps, activity.id))
}
