use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::helpers::activity_sync_helper::sync_user_activities;
use crate::helpers::strava_token_helper::get_valid_access_token;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{DomainError, StravaToken};
use strava_client::conversions::strava_activity_to_domain;

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
    let oauth_state = state.jwt.create_strava_link_state(user.user_id)?;
    let url = state.strava_client.authorize_url(&oauth_state);
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

    let state_raw = match query.state.as_deref() {
        Some(s) => s,
        None => {
            log::warn!("Strava callback: missing state parameter");
            return redirect_with_error(frontend, "Missing state parameter");
        }
    };

    let oauth_state = match app.jwt.verify_oauth_state(state_raw) {
        Ok(claims) => claims,
        Err(e) => {
            log::warn!("Strava callback: invalid OAuth state token: {e}");
            return redirect_with_error(frontend, "Invalid OAuth state");
        }
    };

    log::info!(
        "Strava callback verified state: purpose={} user_id_present={}",
        oauth_state.purpose,
        oauth_state.user_id.is_some()
    );

    let token_resp = match app.strava_client.exchange_code(&query.code).await {
        Ok(t) => t,
        Err(e) => {
            log::error!("Strava token exchange failed: {e}");
            return redirect_with_error(frontend, &format!("Token exchange failed: {e}"));
        }
    };

    let athlete_id = token_resp.athlete.as_ref().map(|a| a.id).unwrap_or(0);
    if athlete_id == 0 {
        return redirect_with_error(frontend, "Strava did not return an athlete id");
    }

    match oauth_state.purpose.as_str() {
        "strava_login" => handle_strava_login_callback(&app, athlete_id, token_resp).await,
        "strava_link" => {
            let Some(user_id) = oauth_state
                .user_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok())
            else {
                return redirect_with_error(frontend, "Invalid link state payload");
            };

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

            let should_start_initial_sync = match app.storage.get_latest_activity_start(user_id).await {
                Ok(None) => true,
                Ok(Some(_)) => false,
                Err(e) => {
                    log::warn!(
                        "Could not determine if initial sync should start for user {}: {}",
                        user_id, e
                    );
                    false
                }
            };
            if should_start_initial_sync {
                start_background_initial_sync(app.clone(), user_id);
            }

            log::info!("Strava linked successfully for user {user_id}, redirecting to frontend");
            HttpResponse::Found()
                .append_header(("Location", format!("{frontend}/profile")))
                .finish()
        }
        _ => redirect_with_error(frontend, "Invalid state purpose"),
    }
}

async fn generate_unique_strava_username(
    app: &web::Data<AppState>,
    athlete_id: i64,
) -> Result<String, DomainError> {
    let base = format!("strava_{athlete_id}");
    if app.storage.get_user_by_username(&base).await.is_err() {
        return Ok(base);
    }

    for suffix in 1..=9999 {
        let candidate = format!("strava_{athlete_id}_{suffix}");
        if app.storage.get_user_by_username(&candidate).await.is_err() {
            return Ok(candidate);
        }
    }

    Err(DomainError::Internal(
        "Unable to allocate a unique Strava username".into(),
    ))
}

async fn handle_strava_login_callback(
    app: &web::Data<AppState>,
    athlete_id: i64,
    token_resp: strava_client::types::TokenResponse,
) -> HttpResponse {
    let frontend = &app.frontend_url;
    let mut created_new_user = false;

    let user_id = match app.storage.get_strava_token_by_athlete_id(athlete_id).await {
        Ok(existing) => existing.user_id,
        Err(_) => {
            let username = match generate_unique_strava_username(app, athlete_id).await {
                Ok(u) => u,
                Err(e) => return redirect_with_error(frontend, &format!("Failed to create user: {e}")),
            };
            let user = domain::User {
                id: Uuid::new_v4(),
                username: username.clone(),
                display_name: username,
                email: None,
                created_at: Utc::now(),
                mas_current: None,
                quota_balance_usd: 0.0,
            };
            if let Err(e) = app.storage.create_user(&user).await {
                return redirect_with_error(frontend, &format!("Failed to create user: {e}"));
            }
            created_new_user = true;
            user.id
        }
    };

    let strava_token = StravaToken {
        user_id,
        strava_athlete_id: athlete_id,
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at: DateTime::<Utc>::from_timestamp(token_resp.expires_at, 0)
            .unwrap_or_else(Utc::now),
    };

    if let Err(e) = app.storage.upsert_strava_token(&strava_token).await {
        return redirect_with_error(frontend, &format!("Failed to save Strava token: {e}"));
    }

    if created_new_user {
        start_background_initial_sync(app.clone(), user_id);
    }

    let jwt = match app.jwt.create_token(user_id) {
        Ok(v) => v,
        Err(e) => return redirect_with_error(frontend, &format!("Failed to create session token: {e}")),
    };

    HttpResponse::Found()
        .append_header(("Location", format!("{frontend}/activities")))
        .cookie(build_session_cookie(&jwt))
        .finish()
}

fn build_session_cookie(token: &str) -> Cookie<'static> {
    Cookie::build("session", token.to_owned())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::days(7))
        .finish()
}

fn redirect_with_error(frontend_url: &str, message: &str) -> HttpResponse {
    let encoded = urlencoding::encode(message);
    HttpResponse::Found()
        .append_header(("Location", format!("{frontend_url}/profile?error={encoded}")))
        .finish()
}

fn start_background_initial_sync(app: web::Data<AppState>, user_id: Uuid) {
    tokio::spawn(async move {
        if !app.try_begin_activities_sync(user_id).await {
            log::info!(
                "Initial background sync skipped for user {} because another sync is already running",
                user_id
            );
            return;
        }

        log::info!("Starting initial background Strava sync for user {}", user_id);
        let result = sync_user_activities(&app, user_id, None, None).await;

        match result {
            Ok(total) => {
                app.mark_activities_sync_finished(user_id).await;
                log::info!(
                    "Initial background Strava sync complete for user {}: {} activities",
                    user_id,
                    total
                );
            }
            Err(e) => {
                app.mark_activities_sync_failed(user_id, e.to_string()).await;
                log::error!(
                    "Initial background Strava sync failed for user {}: {}",
                    user_id,
                    e
                );
            }
        }
    });
}

pub async fn disconnect(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /strava/disconnect user={}", user.user_id);

    // Get the current access token to deauthorize with Strava
    let token = state.storage.get_strava_token(user.user_id).await?;

    // Call Strava's deauthorize endpoint
    if let Err(e) = state.strava_client.deauthorize(&token.access_token).await {
        log::warn!("Strava deauthorize API call failed (proceeding with local cleanup): {e}");
    }

    // Delete all Strava-related data locally
    state.storage.delete_strava_data(user.user_id).await?;

    log::info!("Strava disconnected for user {}", user.user_id);
    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })))
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

// ---------------------------------------------------------------------------
// Webhook endpoints (unauthenticated — called by Strava)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct WebhookValidateQuery {
    #[serde(rename = "hub.mode")]
    pub hub_mode: String,
    #[serde(rename = "hub.verify_token")]
    pub hub_verify_token: String,
    #[serde(rename = "hub.challenge")]
    pub hub_challenge: String,
}

pub async fn webhook_validate(
    state: web::Data<AppState>,
    query: web::Query<WebhookValidateQuery>,
) -> HttpResponse {
    log::info!("GET /strava/webhook hub.mode={}", query.hub_mode);

    let verify_token = match &state.strava_webhook_verify_token {
        Some(t) => t,
        None => {
            log::warn!("Webhook verify token not configured, rejecting");
            return HttpResponse::Forbidden().finish();
        }
    };

    if query.hub_mode != "subscribe" || &query.hub_verify_token != verify_token {
        log::warn!("Webhook validation failed: mode or token mismatch");
        return HttpResponse::Forbidden().finish();
    }

    log::info!("Webhook validation succeeded, returning challenge");
    HttpResponse::Ok().json(serde_json::json!({
        "hub.challenge": query.hub_challenge,
    }))
}

#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    pub aspect_type: String,     // "create", "update", "delete"
    pub object_id: i64,          // activity id or athlete id
    pub object_type: String,     // "activity" or "athlete"
    pub owner_id: i64,           // athlete id
    #[allow(dead_code)]
    pub subscription_id: i64,
    #[allow(dead_code)]
    pub event_time: i64,
    #[serde(default)]
    pub updates: HashMap<String, serde_json::Value>,
}

pub async fn webhook_event(
    state: web::Data<AppState>,
    body: web::Json<WebhookEvent>,
) -> HttpResponse {
    log::info!(
        "POST /strava/webhook object_type={} aspect_type={} object_id={} owner_id={}",
        body.object_type, body.aspect_type, body.object_id, body.owner_id
    );

    let event = body.into_inner();
    let storage = Arc::clone(&state.storage);
    let strava_client = Arc::clone(&state.strava_client);

    tokio::spawn(async move {
        if let Err(e) = handle_webhook_event(event, storage, strava_client).await {
            log::error!("Webhook event handler error: {e}");
        }
    });

    HttpResponse::Ok().finish()
}

async fn handle_webhook_event(
    event: WebhookEvent,
    storage: Arc<storage::SqliteStorage>,
    strava_client: Arc<strava_client::StravaClient>,
) -> Result<(), DomainError> {
    // Look up the user by Strava athlete id
    let token = storage.get_strava_token_by_athlete_id(event.owner_id).await?;
    let user_id = token.user_id;

    match (event.object_type.as_str(), event.aspect_type.as_str()) {
        ("activity", "create") | ("activity", "update") => {
            log::info!("Webhook: syncing activity {} for user {}", event.object_id, user_id);
            let access_token = get_valid_access_token(&storage, &strava_client, user_id).await?;
            let strava_activity = strava_client.get_activity(&access_token, event.object_id).await?;
            let domain_activity = strava_activity_to_domain(&strava_activity, user_id);
            storage.upsert_activities(&[domain_activity]).await?;
            log::info!("Webhook: activity {} synced for user {}", event.object_id, user_id);
        }
        ("activity", "delete") => {
            log::info!("Webhook: deleting activity {} for user {}", event.object_id, user_id);
            storage.delete_activity_by_strava_id(event.object_id, user_id).await?;
            log::info!("Webhook: activity {} deleted for user {}", event.object_id, user_id);
        }
        ("athlete", "update") => {
            // Check if this is a deauthorization event
            if event.updates.get("authorized").and_then(|v| v.as_str()) == Some("false") {
                log::info!("Webhook: athlete {} deauthorized, cleaning up user {}", event.owner_id, user_id);
                storage.delete_strava_data(user_id).await?;
                log::info!("Webhook: strava data deleted for user {}", user_id);
            }
        }
        _ => {
            log::debug!("Webhook: ignoring event {}:{}", event.object_type, event.aspect_type);
        }
    }

    Ok(())
}
