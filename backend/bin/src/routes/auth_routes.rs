use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;
use webauthn_rs_proto::{PublicKeyCredential, RegisterPublicKeyCredential};

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::DomainError;

#[derive(Deserialize)]
pub struct RegisterStartRequest {
    pub username: String,
    pub email: Option<String>,
}

pub async fn register_start(
    state: web::Data<AppState>,
    body: web::Json<RegisterStartRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/register/start username={}", body.username);

    let username = body.username.trim();
    if username.is_empty() {
        return Err(DomainError::BadRequest("Username required".into()).into());
    }

    // Check if username already exists
    if state.storage.get_user_by_username(username).await.is_ok() {
        return Err(DomainError::BadRequest("Username already taken".into()).into());
    }

    let email = body
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Some(email) = &email {
        if !email.contains('@') {
            return Err(DomainError::BadRequest("Invalid email format".into()).into());
        }
        if state.storage.get_user_by_email(email).await.is_ok() {
            return Err(DomainError::BadRequest("Email already taken".into()).into());
        }
    }

    let user = domain::User::new(username.to_string(), username.to_string(), email);
    let user_id = user.id;

    state.storage.create_user(&user).await?;

    let ccr = state.webauthn.start_registration(user_id, username).await?;

    log::info!("Registration challenge created for user {user_id}");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user_id": user_id,
        "options": ccr,
    })))
}

pub async fn register_finish(
    state: web::Data<AppState>,
    body: web::Json<RegisterFinishRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/register/finish user_id={}", body.user_id);

    let passkey = state
        .webauthn
        .finish_registration(body.user_id, &body.credential)
        .await?;

    let passkey_json =
        serde_json::to_string(&passkey).map_err(|e| DomainError::Internal(e.to_string()))?;

    state
        .storage
        .store_passkey(body.user_id, &passkey_json)
        .await?;

    let token = state.jwt.create_token(body.user_id)?;
    let cookie = build_session_cookie(&token);

    log::info!("User {} registered successfully", body.user_id);

    Ok(HttpResponse::Ok().cookie(cookie).json(serde_json::json!({
        "status": "ok",
    })))
}

#[derive(Deserialize)]
pub struct RegisterFinishRequest {
    pub user_id: Uuid,
    pub credential: RegisterPublicKeyCredential,
}

pub async fn login_start(
    state: web::Data<AppState>,
    _body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/login/start (username-less)");
    let (auth_id, rcr) = state.webauthn.start_authentication().await?;
    log::info!("Auth challenge created auth_id={auth_id}");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "auth_id": auth_id,
        "options": rcr,
    })))
}

#[derive(Deserialize)]
pub struct LoginFinishRequest {
    pub auth_id: Uuid,
    pub credential: PublicKeyCredential,
}

pub async fn login_finish(
    state: web::Data<AppState>,
    body: web::Json<LoginFinishRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/login/finish auth_id={}", body.auth_id);

    let user_id = state
        .webauthn
        .identify_user_from_authentication(&body.credential)?;

    let passkey_jsons = state.storage.get_passkeys_for_user(user_id).await?;
    if passkey_jsons.is_empty() {
        return Err(DomainError::Auth("No passkeys registered".into()).into());
    }

    let passkeys: Vec<webauthn_rs::prelude::Passkey> = passkey_jsons
        .iter()
        .filter_map(|j| serde_json::from_str(j).ok())
        .collect();

    let _auth_result = state
        .webauthn
        .finish_authentication(body.auth_id, &body.credential, &passkeys)
        .await?;

    let token = state.jwt.create_token(user_id)?;
    let cookie = build_session_cookie(&token);

    log::info!("User {} logged in successfully", user_id);

    Ok(HttpResponse::Ok().cookie(cookie).json(serde_json::json!({
        "status": "ok",
    })))
}

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

fn build_session_cookie(token: &str) -> Cookie<'static> {
    Cookie::build("session", token.to_owned())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::days(7))
        .finish()
}
