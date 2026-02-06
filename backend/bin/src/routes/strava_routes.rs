use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use storage::Storage;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{DomainError, StravaToken};

pub async fn link(
    state: web::Data<AppState>,
    _user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    if !state.strava_client.is_configured() {
        return Err(DomainError::BadRequest(
            "Strava is not configured. Set STRAVA_CLIENT_ID and STRAVA_CLIENT_SECRET.".into(),
        )
        .into());
    }
    let url = state.strava_client.authorize_url();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "url": url,
    })))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
}

pub async fn callback_with_cookie(
    state: web::Data<AppState>,
    query: web::Query<CallbackQuery>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let token_resp = state.strava_client.exchange_code(&query.code).await?;

    let athlete_id = token_resp
        .athlete
        .as_ref()
        .map(|a| a.id)
        .unwrap_or(0);

    let strava_token = StravaToken {
        user_id: user.user_id,
        strava_athlete_id: athlete_id,
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at: DateTime::<Utc>::from_timestamp(token_resp.expires_at, 0)
            .unwrap_or_else(Utc::now),
    };

    state.storage.upsert_strava_token(&strava_token).await?;

    // Redirect to frontend
    let redirect_url = format!("{}/activities", state.frontend_url);
    Ok(HttpResponse::Found()
        .append_header(("Location", redirect_url))
        .finish())
}

pub async fn status(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    match state.storage.get_strava_token(user.user_id).await {
        Ok(token) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "linked": true,
            "athlete_id": token.strava_athlete_id,
        }))),
        Err(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "linked": false,
        }))),
    }
}
