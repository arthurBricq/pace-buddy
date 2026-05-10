use actix_web::{web, HttpResponse};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::helpers::invite_code_helper::{
    generate_invite_code, hash_invite_code, invite_code_is_valid_for_redemption,
    normalize_invite_code,
};
use crate::helpers::strava_data_helper::{load_and_cache_laps, load_and_cache_streams};
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;

#[derive(Serialize)]
struct AdminStats {
    user_count: usize,
}

#[derive(Serialize)]
pub struct AdminUserQuotaSpending {
    user_id: String,
    username: String,
    display_name: String,
    email: Option<String>,
    created_at: String,
    quota_balance_usd: f64,
    total_granted_usd: f64,
    total_spent_usd: f64,
}

#[derive(Serialize)]
pub struct AdminInviteCode {
    id: String,
    created_by_user_id: Option<String>,
    created_for: Option<String>,
    created_at: String,
    expires_at: Option<String>,
    used_at: Option<String>,
    used_by_strava_athlete_id: Option<i64>,
    revoked_at: Option<String>,
    is_redeemable: bool,
}

#[derive(Serialize)]
pub struct AdminCoachContextRow {
    user_id: String,
    username: String,
    display_name: String,
    model: String,
    personality: String,
    settings_updated_at: String,
    memory_updated_at: String,
    state_updated_at: String,
    last_interaction_at: Option<String>,
    last_seen_activity_start_date: Option<String>,
    pinned_facts_count: usize,
    episodic_memory_count: usize,
    rolling_summary: String,
    active_coaching_plan: String,
    message_count_since_normalization: i32,
    context_snapshot: Option<String>,
}

/// Verify the authenticated user is the admin by checking their Strava athlete ID.
async fn verify_admin(
    state: &web::Data<AppState>,
    user: &AuthenticatedUser,
) -> Result<(), AppError> {
    let admin_id = state
        .admin_strava_athlete_id
        .ok_or_else(|| domain::DomainError::Forbidden("Admin access is not configured".into()))?;

    let token = state
        .storage
        .get_strava_token(user.user_id)
        .await
        .map_err(|_| domain::DomainError::Forbidden("Not an admin".into()))?;

    log::info!(
        "Admin verification for user: {} with athlete-id {}",
        user.user_id,
        token.strava_athlete_id
    );

    if token.strava_athlete_id != admin_id {
        return Err(domain::DomainError::Forbidden("Not an admin".into()).into());
    }

    Ok(())
}

fn to_admin_invite_code(invite: &domain::invite_code::InviteCode) -> AdminInviteCode {
    AdminInviteCode {
        id: invite.id.to_string(),
        created_by_user_id: invite.created_by_user_id.as_ref().map(|id| id.to_string()),
        created_for: invite.created_for.clone(),
        created_at: invite.created_at.to_rfc3339(),
        expires_at: invite.expires_at.as_ref().map(|dt| dt.to_rfc3339()),
        used_at: invite.used_at.as_ref().map(|dt| dt.to_rfc3339()),
        used_by_strava_athlete_id: invite.used_by_strava_athlete_id,
        revoked_at: invite.revoked_at.as_ref().map(|dt| dt.to_rfc3339()),
        is_redeemable: invite_code_is_valid_for_redemption(invite),
    }
}

pub async fn stats(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;

    let users = state.storage.list_users().await?;

    Ok(HttpResponse::Ok().json(AdminStats {
        user_count: users.len(),
    }))
}

pub async fn users_by_quota_spent(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;

    let users = state.storage.list_users().await?;
    let mut leaderboard = Vec::with_capacity(users.len());

    for u in users {
        let requests = state.storage.get_user_quota_requests(u.id).await?;
        let total_granted_usd: f64 = requests
            .iter()
            .filter(|r| r.status == domain::QuotaRequestStatus::Approved)
            .map(|r| r.granted_amount_usd.unwrap_or(0.0))
            .sum();

        let total_spent_usd = (domain::user::DEFAULT_INITIAL_USER_QUOTA_USD + total_granted_usd
            - u.quota_balance_usd)
            .max(0.0);

        leaderboard.push(AdminUserQuotaSpending {
            user_id: u.id.to_string(),
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            created_at: u.created_at.to_rfc3339(),
            quota_balance_usd: u.quota_balance_usd,
            total_granted_usd,
            total_spent_usd,
        });
    }

    leaderboard.sort_by(|a, b| {
        b.total_spent_usd
            .partial_cmp(&a.total_spent_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.username.cmp(&b.username))
    });

    Ok(HttpResponse::Ok().json(leaderboard))
}

pub async fn list_coach_contexts(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;

    let users = state.storage.list_users().await?;
    let mut rows = Vec::with_capacity(users.len());

    for u in users {
        rows.push(build_admin_coach_context_row(&state, &u, false).await?);
    }

    rows.sort_by(|a, b| a.username.cmp(&b.username));

    Ok(HttpResponse::Ok().json(rows))
}

pub async fn get_coach_context(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    let coach_user_id = path.into_inner();
    let coach_user = state.storage.get_user_by_id(coach_user_id).await?;
    let row = build_admin_coach_context_row(&state, &coach_user, true).await?;
    Ok(HttpResponse::Ok().json(row))
}

async fn build_admin_coach_context_row(
    state: &web::Data<AppState>,
    user: &domain::User,
    include_snapshot: bool,
) -> Result<AdminCoachContextRow, AppError> {
    let settings = state
        .storage
        .get_or_create_running_coach_settings(user.id)
        .await?;
    let memory = state
        .storage
        .get_or_create_running_coach_memory(user.id)
        .await?;
    let coach_state = state
        .storage
        .get_or_create_running_coach_state(user.id)
        .await?;

    let context_snapshot = if include_snapshot {
        Some(
            state
                .coach_memory
                .build_context(user.id, &settings, &coach_state, &memory.data)
                .await?
                .content,
        )
    } else {
        None
    };

    Ok(AdminCoachContextRow {
        user_id: user.id.to_string(),
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        model: settings.model,
        personality: settings.personality,
        settings_updated_at: settings.updated_at.to_rfc3339(),
        memory_updated_at: memory.updated_at.to_rfc3339(),
        state_updated_at: coach_state.updated_at.to_rfc3339(),
        last_interaction_at: coach_state.last_interaction_at.map(|v| v.to_rfc3339()),
        last_seen_activity_start_date: coach_state
            .last_seen_activity_start_date
            .map(|v| v.to_rfc3339()),
        pinned_facts_count: memory.data.pinned_facts.len(),
        episodic_memory_count: memory.data.episodic_memory.len(),
        rolling_summary: memory.data.rolling_summary,
        active_coaching_plan: memory.data.active_coaching_plan,
        message_count_since_normalization: memory.message_count_since_normalization,
        context_snapshot,
    })
}

pub async fn list_quota_requests(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    let requests = state.storage.get_pending_quota_requests().await?;
    Ok(HttpResponse::Ok().json(requests))
}

#[derive(Deserialize)]
pub struct ApproveQuotaBody {
    pub amount_usd: f64,
}

pub async fn approve_quota_request(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<ApproveQuotaBody>,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    let request_id = path.into_inner();

    let req = state.storage.get_quota_request(request_id).await?;
    if req.status != domain::QuotaRequestStatus::Pending {
        return Err(domain::DomainError::BadRequest("Request is not pending".into()).into());
    }

    state
        .storage
        .resolve_quota_request(
            request_id,
            domain::QuotaRequestStatus::Approved,
            Some(body.amount_usd),
        )
        .await?;

    state
        .storage
        .add_quota(req.user_id, body.amount_usd)
        .await?;

    log::info!(
        "Admin approved quota request {} for user {} amount=${:.2}",
        request_id,
        req.user_id,
        body.amount_usd
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })))
}

pub async fn reject_quota_request(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    let request_id = path.into_inner();

    state
        .storage
        .resolve_quota_request(request_id, domain::QuotaRequestStatus::Rejected, None)
        .await?;

    log::info!("Admin rejected quota request {}", request_id);

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })))
}

pub async fn delete_all_data(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    state.storage.delete_all_data().await?;
    log::warn!("Admin {} deleted all database data", user.user_id);
    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })))
}

#[derive(Deserialize)]
pub struct CreateInviteCodeBody {
    pub created_for: Option<String>,
    pub expires_in_days: Option<i64>,
    pub code: Option<String>,
}

#[derive(Serialize)]
pub struct CreateInviteCodeResponse {
    code: String,
    invite: AdminInviteCode,
}

pub async fn create_invite_code(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CreateInviteCodeBody>,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;

    let created_for = body
        .created_for
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());

    let expires_at = match body.expires_in_days {
        Some(days) if days <= 0 => {
            return Err(
                domain::DomainError::BadRequest("expires_in_days must be positive".into()).into(),
            )
        }
        Some(days) => Some(Utc::now() + Duration::days(days)),
        None => None,
    };

    let requested_code = body
        .code
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let max_attempts = if requested_code.is_some() { 1 } else { 5 };

    for _ in 0..max_attempts {
        let code = if let Some(raw_code) = requested_code {
            normalize_invite_code(raw_code)?
        } else {
            generate_invite_code()
        };

        let invite = domain::invite_code::InviteCode {
            id: Uuid::new_v4(),
            code_hash: hash_invite_code(&code),
            created_by_user_id: Some(user.user_id),
            created_for: created_for.clone(),
            created_at: Utc::now(),
            expires_at: expires_at.clone(),
            used_at: None,
            used_by_strava_athlete_id: None,
            revoked_at: None,
        };

        match state.storage.create_invite_code(&invite).await {
            Ok(()) => {
                return Ok(HttpResponse::Ok().json(CreateInviteCodeResponse {
                    code,
                    invite: to_admin_invite_code(&invite),
                }))
            }
            Err(domain::DomainError::Storage(message))
                if requested_code.is_some() && message.contains("UNIQUE") =>
            {
                return Err(
                    domain::DomainError::BadRequest("Invite code already exists".into()).into(),
                )
            }
            Err(domain::DomainError::Storage(message))
                if requested_code.is_none() && message.contains("UNIQUE") =>
            {
                continue
            }
            Err(e) => return Err(e.into()),
        }
    }

    Err(domain::DomainError::Internal("Failed to generate a unique invite code".into()).into())
}

pub async fn list_invite_codes(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    let invites = state.storage.list_invite_codes().await?;
    let response: Vec<AdminInviteCode> = invites.iter().map(to_admin_invite_code).collect();
    Ok(HttpResponse::Ok().json(response))
}

pub async fn revoke_invite_code(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    state.storage.revoke_invite_code(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })))
}

#[derive(Serialize)]
struct ActivityDumpIntervalResult {
    algorithm: String,
    result: serde_json::Value,
}

#[derive(Serialize)]
struct ActivityDump {
    activity: domain::Activity,
    streams: Vec<domain::ActivityStream>,
    laps: Vec<domain::ActivityLap>,
    interval_result: Option<ActivityDumpIntervalResult>,
}

pub async fn dump_activity(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    verify_admin(&state, &user).await?;
    let activity_id = path.into_inner();

    let activity = state.storage.get_activity(activity_id, user.user_id).await?;

    let mut streams = state
        .storage
        .get_streams(activity_id)
        .await
        .unwrap_or_default();
    if streams.is_empty() {
        streams = load_and_cache_streams(&state, &activity).await?;
    }

    let mut laps = state.storage.get_laps(activity_id).await.unwrap_or_default();
    if laps.is_empty() {
        laps = load_and_cache_laps(&state, &activity).await?;
    }

    let interval_result = state
        .storage
        .get_interval_result(activity_id)
        .await?
        .map(|(algorithm, result_json)| {
            let result = serde_json::from_str::<serde_json::Value>(&result_json)
                .unwrap_or(serde_json::Value::String(result_json));
            ActivityDumpIntervalResult { algorithm, result }
        });

    Ok(HttpResponse::Ok().json(ActivityDump {
        activity,
        streams,
        laps,
        interval_result,
    }))
}
