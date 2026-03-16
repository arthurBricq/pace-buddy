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
