use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{DomainError, StravaToken};

pub async fn link(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::info!("GET /strava/link user={}", user.user_id);

    if !state.strava_client.is_configured() {
        log::warn!("Strava not configured, rejecting link request");
        return Err(DomainError::BadRequest(
            "Strava is not configured. Set STRAVA_CLIENT_ID and STRAVA_CLIENT_SECRET.".into(),
        )
        .into());
    }
    let url = state
        .strava_client
        .authorize_url(&user.user_id.to_string());
    log::info!("Generated Strava authorize URL for user {}", user.user_id);
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "url": url,
    })))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: Option<String>,
}

/// Strava redirects here after OAuth approval.
/// This is a direct browser navigation (not a fetch), so on error
/// we redirect to the frontend with an error query param instead of
/// returning JSON that would show as a raw page.
pub async fn callback(
    app: web::Data<AppState>,
    query: web::Query<CallbackQuery>,
) -> HttpResponse {
    log::info!("GET /strava/callback code=<redacted> state={:?}", query.state);
    let frontend = &app.frontend_url;

    let user_id = match query
        .state
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => {
            log::warn!("Strava callback: missing or invalid state parameter");
            return redirect_with_error(frontend, "Missing or invalid state parameter");
        }
    };

    log::info!("Exchanging Strava OAuth code for user {user_id}");
    let token_resp = match app.strava_client.exchange_code(&query.code).await {
        Ok(t) => t,
        Err(e) => {
            log::error!("Strava token exchange failed for user {user_id}: {e}");
            return redirect_with_error(frontend, &format!("Token exchange failed: {e}"));
        }
    };

    let athlete_id = token_resp.athlete.as_ref().map(|a| a.id).unwrap_or(0);
    log::info!("Strava token exchange succeeded: athlete_id={athlete_id} user={user_id}");

    let strava_token = StravaToken {
        user_id,
        strava_athlete_id: athlete_id,
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at: DateTime::<Utc>::from_timestamp(token_resp.expires_at, 0)
            .unwrap_or_else(Utc::now),
    };

    if let Err(e) = app.storage.upsert_strava_token(&strava_token).await {
        log::error!("Failed to save Strava token for user {user_id}: {e}");
        return redirect_with_error(frontend, &format!("Failed to save token: {e}"));
    }

    log::info!("Strava linked successfully for user {user_id}, redirecting to frontend");
    HttpResponse::Found()
        .append_header(("Location", format!("{frontend}/profile")))
        .finish()
}

fn redirect_with_error(frontend_url: &str, message: &str) -> HttpResponse {
    let encoded = urlencoding::encode(message);
    HttpResponse::Found()
        .append_header(("Location", format!("{frontend_url}/profile?error={encoded}")))
        .finish()
}

pub async fn status(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /strava/status user={}", user.user_id);
    match state.storage.get_strava_token(user.user_id).await {
        Ok(token) => {
            log::debug!("Strava linked for user {}, athlete_id={}", user.user_id, token.strava_athlete_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "linked": true,
                "athlete_id": token.strava_athlete_id,
            })))
        }
        Err(_) => {
            log::debug!("Strava not linked for user {}", user.user_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "linked": false,
            })))
        }
    }
}
