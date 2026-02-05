use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::cookie::{Cookie, SameSite};
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
    };

    state.storage.create_user(&user).await?;

    let ccr = state.webauthn.start_registration(user_id, &body.username).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user_id": user_id,
        "options": ccr,
    })))
}

pub async fn register_finish(
    state: web::Data<AppState>,
    body: web::Json<RegisterFinishRequest>,
) -> Result<HttpResponse, AppError> {
    let passkey = state
        .webauthn
        .finish_registration(body.user_id, &body.credential)
        .await?;

    let passkey_json = serde_json::to_string(&passkey)
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    state.storage.store_passkey(body.user_id, &passkey_json).await?;

    let token = state.jwt.create_token(body.user_id)?;
    let cookie = build_session_cookie(&token);

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
    let _auth_result = state
        .webauthn
        .finish_authentication(body.user_id, &body.credential)
        .await?;

    let token = state.jwt.create_token(body.user_id)?;
    let cookie = build_session_cookie(&token);

    Ok(HttpResponse::Ok().cookie(cookie).json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn logout(_req: HttpRequest) -> Result<HttpResponse, AppError> {
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
    let u = state.storage.get_user_by_id(user.user_id).await?;
    Ok(HttpResponse::Ok().json(u))
}

fn build_session_cookie(token: &str) -> Cookie<'static> {
    Cookie::build("session", token.to_owned())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::days(7))
        .finish()
}
