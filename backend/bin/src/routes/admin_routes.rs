use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
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

    log::info!("Admin verification for user: {}", user.user_id);

    if token.strava_athlete_id != admin_id {
        return Err(domain::DomainError::Forbidden("Not an admin".into()).into());
    }

    Ok(())
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
