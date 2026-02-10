use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::cookie::{Cookie, SameSite};
use chrono::Datelike;
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;
use webauthn_rs_proto::{RegisterPublicKeyCredential, PublicKeyCredential};

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::DomainError;

#[derive(Deserialize)]
pub struct RegisterStartRequest {
    pub username: String,
    pub display_name: String,
}

pub async fn register_start(
    state: web::Data<AppState>,
    body: web::Json<RegisterStartRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/register/start username={}", body.username);

    if body.username.is_empty() {
        return Err(DomainError::BadRequest("Username required".into()).into());
    }

    // Check if username already exists
    if state.storage.get_user_by_username(&body.username).await.is_ok() {
        return Err(DomainError::BadRequest("Username already taken".into()).into());
    }

    let user_id = Uuid::new_v4();
    let user = domain::User {
        id: user_id,
        username: body.username.clone(),
        display_name: body.display_name.clone(),
        created_at: chrono::Utc::now(),
        mas_current: None,
    };

    state.storage.create_user(&user).await?;

    let ccr = state.webauthn.start_registration(user_id, &body.username).await?;

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

    let passkey_json = serde_json::to_string(&passkey)
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    state.storage.store_passkey(body.user_id, &passkey_json).await?;

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

#[derive(Deserialize)]
pub struct LoginStartRequest {
    pub username: String,
}

pub async fn login_start(
    state: web::Data<AppState>,
    body: web::Json<LoginStartRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/login/start username={}", body.username);

    let user = state
        .storage
        .get_user_by_username(&body.username)
        .await
        .map_err(|_| DomainError::NotFound("User not found".into()))?;

    let passkey_jsons = state.storage.get_passkeys_for_user(user.id).await?;
    if passkey_jsons.is_empty() {
        return Err(DomainError::Auth("No passkeys registered".into()).into());
    }

    let passkeys: Vec<webauthn_rs::prelude::Passkey> = passkey_jsons
        .iter()
        .filter_map(|j| serde_json::from_str(j).ok())
        .collect();

    if passkeys.is_empty() {
        return Err(DomainError::Auth("Failed to parse passkeys".into()).into());
    }

    let rcr = state
        .webauthn
        .start_authentication(user.id, &passkeys)
        .await?;

    log::info!("Auth challenge created for user {}", user.id);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user_id": user.id,
        "options": rcr,
    })))
}

#[derive(Deserialize)]
pub struct LoginFinishRequest {
    pub user_id: Uuid,
    pub credential: PublicKeyCredential,
}

pub async fn login_finish(
    state: web::Data<AppState>,
    body: web::Json<LoginFinishRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/login/finish user_id={}", body.user_id);

    let _auth_result = state
        .webauthn
        .finish_authentication(body.user_id, &body.credential)
        .await?;

    let token = state.jwt.create_token(body.user_id)?;
    let cookie = build_session_cookie(&token);

    log::info!("User {} logged in successfully", body.user_id);

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

pub async fn list_all_users(
    state: web::Data<AppState>,
    _user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/users");
    let users = state.storage.list_users().await?;
    Ok(HttpResponse::Ok().json(users))
}

pub async fn get_mas(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/mas user_id={}", user.user_id);
    let u = state.storage.get_user_by_id(user.user_id).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "mas_mps": u.mas_current,
    })))
}

#[derive(Deserialize)]
pub struct UpdateMASRequest {
    pub mas_mps: Option<f64>,
}

pub async fn update_mas(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<UpdateMASRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("PATCH /auth/mas user_id={} mas_mps={:?}", user.user_id, body.mas_mps);
    state.storage.update_user_mas(user.user_id, body.mas_mps).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn profile(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/profile user_id={}", user.user_id);

    let u = state.storage.get_user_by_id(user.user_id).await?;

    let now = chrono::Utc::now();
    let this_year_start = chrono::NaiveDate::from_ymd_opt(now.year(), 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let last_year_start = chrono::NaiveDate::from_ymd_opt(now.year() - 1, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let ytd = state
        .storage
        .get_running_stats(user.user_id, Some(this_year_start), None, true)
        .await?;
    let last_year = state
        .storage
        .get_running_stats(user.user_id, Some(last_year_start), Some(this_year_start), false)
        .await?;
    let all_time = state
        .storage
        .get_running_stats(user.user_id, None, None, false)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user": u,
        "stats": {
            "ytd": ytd,
            "last_year": last_year,
            "all_time": all_time,
        }
    })))
}

fn build_session_cookie(token: &str) -> Cookie<'static> {
    Cookie::build("session", token.to_owned())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::days(7))
        .finish()
}
