use storage::Storage;
use uuid::Uuid;

use crate::helpers::strava_token_helper::get_valid_access_token;
use crate::state::AppState;
use domain::DomainError;
use strava_client::conversions::strava_activity_to_domain;

pub async fn sync_user_activities(
    state: &AppState,
    user_id: Uuid,
    after: Option<i64>,
    before: Option<i64>,
) -> Result<usize, DomainError> {
    // If no explicit `after` is provided, default to latest stored activity start_date
    // so we only fetch new activities (incremental sync).
    let after = match after {
        Some(ts) => Some(ts),
        None => {
            let latest = state.storage.get_latest_activity_start(user_id).await?;
            latest.map(|dt| dt.timestamp())
        }
    };

    log::info!(
        "Starting Strava sync user={} after={:?} before={:?}",
        user_id,
        after,
        before
    );

    let access_token =
        get_valid_access_token(&state.storage, &state.strava_client, user_id).await?;

    let mut all_activities = Vec::new();
    let mut page = 1u32;
    let per_page = 200u32;

    loop {
        log::info!(
            "Fetching Strava activities page {page} for user {}",
            user_id
        );
        let strava_activities = state
            .strava_client
            .get_activities(&access_token, page, per_page, after, before)
            .await?;

        let count = strava_activities.len();
        log::info!(
            "Got {count} activities from Strava (page {page}) for user {}",
            user_id
        );

        let domain_activities: Vec<_> = strava_activities
            .iter()
            .map(|sa| strava_activity_to_domain(sa, user_id))
            .collect();

        all_activities.extend(domain_activities);

        if (count as u32) < per_page {
            break;
        }
        page += 1;
    }

    let total = all_activities.len();
    state.storage.upsert_activities(&all_activities).await?;

    if let Some(mas_mps) = state.recompute_user_mas_from_races(user_id).await? {
        log::info!(
            "Updated MAS after sync user={} mas_mps={:.4}",
            user_id,
            mas_mps
        );
    }

    log::info!(
        "Strava sync complete: {total} activities for user {}",
        user_id
    );
    Ok(total)
}
