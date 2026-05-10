use actix_web::web;
use storage::Storage;
use uuid::Uuid;

use crate::helpers::strava_token_helper::get_valid_access_token;
use crate::state::AppState;
use domain::DomainError;
use strava_client::conversions::strava_activity_to_domain;

/// Result of `sync_user_activities`: the count of activities synced and the
/// list of their Strava IDs. The IDs let the post-sync interval-parsing hook
/// look up canonical activity rows by `(strava_id, user_id)` — necessary
/// because `upsert_activities` preserves existing UUIDs on conflict, so the
/// in-memory `Activity` objects we built from the Strava response don't
/// always carry the canonical `id`.
pub struct SyncOutcome {
    pub synced: usize,
    pub strava_ids: Vec<i64>,
}

pub async fn sync_user_activities(
    state: &AppState,
    user_id: Uuid,
    after: Option<i64>,
    before: Option<i64>,
) -> Result<SyncOutcome, DomainError> {
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
    let strava_ids: Vec<i64> = all_activities.iter().map(|a| a.strava_id).collect();
    state.storage.upsert_activities(&all_activities).await?;

    if let Some(mas_kmh) = state.recompute_user_mas_from_races(user_id).await? {
        log::info!(
            "Updated MAS after sync user={} mas_kmh={:.4}",
            user_id,
            mas_kmh
        );
    }

    log::info!(
        "Strava sync complete: {total} activities for user {}",
        user_id
    );
    Ok(SyncOutcome {
        synced: total,
        strava_ids,
    })
}

/// Parse intervals for the activities just synced.
///
/// Each strava_id is looked up canonically by `(strava_id, user_id)`
/// (because `upsert_activities` preserves existing UUIDs on conflict, our
/// in-memory Activity objects don't always carry the right id), then filtered
/// to `sport_type == "Run"`, then handed to `resolve_intervals`.
///
/// `resolve_intervals` short-circuits when an interval result is already
/// cached, so re-syncing the same activity is essentially free. Older
/// activities (not in `strava_ids`) get parsed lazily when the user opens
/// their detail page.
pub fn spawn_post_sync_interval_parsing(
    app: web::Data<AppState>,
    user_id: Uuid,
    strava_ids: Vec<i64>,
) {
    if strava_ids.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let total = strava_ids.len();
        let (mut ok, mut skipped, mut failed) = (0usize, 0usize, 0usize);

        for strava_id in strava_ids {
            let activity = match app
                .storage
                .get_activity_by_strava_id(strava_id, user_id)
                .await
            {
                Ok(a) => a,
                Err(e) => {
                    log::warn!(
                        "post-sync interval parsing: lookup failed for strava_id {strava_id}: {e}"
                    );
                    failed += 1;
                    continue;
                }
            };

            if activity.sport_type != "Run" {
                skipped += 1;
                continue;
            }

            match app.resolve_intervals(&activity, None, None).await {
                Ok(_) => ok += 1,
                Err(e) => {
                    log::warn!(
                        "post-sync interval parse failed for activity {}: {}",
                        activity.id,
                        e
                    );
                    failed += 1;
                }
            }
        }

        log::info!(
            "Post-sync interval parsing for user {}: total={} parsed_or_cached={} non_run_skipped={} failed={}",
            user_id,
            total,
            ok,
            skipped,
            failed
        );
    });
}
