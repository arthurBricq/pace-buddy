use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, HttpRequest, HttpResponse};
use storage::Storage;

use crate::errors::AppError;
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

pub async fn strava_auth_start(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/strava/start");

    if !state.strava_client.is_configured() {
        return Err(DomainError::BadRequest("Strava is not configured".into()).into());
    }

    let oauth_state = state.jwt.create_strava_login_state()?;
    let url = state.strava_client.authorize_url(&oauth_state);
    log::info!(
        "Strava auth start: issuing signed oauth state token state={} authorize_url={}",
        oauth_state,
        url
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "url": url })))
}
