use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use storage::Storage;

use crate::errors::AppError;
use crate::helpers::invite_code_helper::{
    hash_invite_code, invite_code_is_valid_for_redemption, normalize_invite_code,
};
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::DomainError;

pub async fn logout(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/logout");

    let cookie = Cookie::build("session", "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::ZERO)
        .finish();

    Ok(HttpResponse::Ok().cookie(cookie).json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn me(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/me user_id={}", user.user_id);
    let u = state.storage.get_user_by_id(user.user_id).await?;
    Ok(HttpResponse::Ok().json(u))
}

#[derive(Deserialize)]
pub struct StravaAuthStartBody {
    pub invite_code: Option<String>,
}

pub async fn strava_auth_start(
    state: web::Data<AppState>,
    body: Option<web::Json<StravaAuthStartBody>>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/strava/start");

    if !state.strava_client.is_configured() {
        return Err(DomainError::BadRequest("Strava is not configured".into()).into());
    }

    let invite_code_hash = if let Some(raw_code) = body
        .as_ref()
        .and_then(|payload| payload.invite_code.as_deref())
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        // If there is an invite code provided, make sure
        let normalized = normalize_invite_code(raw_code)?;
        let code_hash = hash_invite_code(&normalized);
        let invite = match state.storage.get_invite_code_by_hash(&code_hash).await {
            Ok(invite) => invite,
            Err(DomainError::NotFound(_)) => {
                return Err(DomainError::BadRequest("Invite code is invalid".into()).into())
            }
            Err(e) => return Err(e.into()),
        };

        if !invite_code_is_valid_for_redemption(&invite) {
            return Err(DomainError::BadRequest(
                "Invite code is invalid, expired, revoked, or already used".into(),
            )
            .into());
        }

        Some(code_hash)
    } else {
        None
    };

    let oauth_state = state.jwt.create_strava_login_state(invite_code_hash)?;
    let url = state.strava_client.authorize_url(&oauth_state);
    log::info!(
        "Strava auth start: issuing signed oauth state token with invite_hash_present={} authorize_url={}",
        body.as_ref()
            .and_then(|payload| payload.invite_code.as_deref())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .is_some(),
        url
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "url": url })))
}
