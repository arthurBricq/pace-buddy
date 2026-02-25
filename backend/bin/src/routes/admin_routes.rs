use actix_web::{web, HttpResponse};
use serde::Serialize;
use storage::Storage;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;

#[derive(Serialize)]
struct AdminStats {
    user_count: usize,
}

/// Verify the authenticated user is the admin by checking their Strava athlete ID.
async fn verify_admin(
    state: &web::Data<AppState>,
    user: &AuthenticatedUser,
) -> Result<(), AppError> {
    let admin_id = state.admin_strava_athlete_id.ok_or_else(|| {
        domain::DomainError::Forbidden("Admin access is not configured".into())
    })?;

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
