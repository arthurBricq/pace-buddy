use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::Training;

fn parse_date(s: &str) -> Result<DateTime<Utc>, String> {
    s.parse::<DateTime<Utc>>()
        .map_err(|e| format!("Invalid date '{}': {}", s, e))
}

#[derive(Deserialize)]
pub struct CreateTrainingRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub race_goal: Option<String>,
}

pub async fn create_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CreateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /trainings user={} name={}", user.user_id, body.name);

    let start_date = body
        .start_date
        .as_deref()
        .map(parse_date)
        .transpose()
        .map_err(|e| AppError(domain::DomainError::BadRequest(e)))?;
    let end_date = body
        .end_date
        .as_deref()
        .map(parse_date)
        .transpose()
        .map_err(|e| AppError(domain::DomainError::BadRequest(e)))?;

    let training = Training {
        id: Uuid::new_v4(),
        user_id: user.user_id,
        name: body.name.clone(),
        description: body.description.clone(),
        start_date,
        end_date,
        race_goal: body.race_goal.clone(),
        created_at: Utc::now(),
    };

    state.storage.create_training(&training).await?;

    Ok(HttpResponse::Created().json(training))
}

pub async fn list_trainings(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /trainings user={}", user.user_id);

    let trainings = state.storage.list_trainings(user.user_id).await?;

    Ok(HttpResponse::Ok().json(trainings))
}

pub async fn get_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("GET /trainings/{training_id} user={}", user.user_id);

    let training = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(training))
}

#[derive(Deserialize)]
pub struct UpdateTrainingRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub race_goal: Option<String>,
}

pub async fn update_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("PATCH /trainings/{training_id} user={}", user.user_id);

    let current = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    let name = body.name.clone().unwrap_or(current.name);
    let description = body.description.clone().or(current.description);
    let start_date = match &body.start_date {
        Some(s) => Some(parse_date(s).map_err(|e| AppError(domain::DomainError::BadRequest(e)))?),
        None => current.start_date,
    };
    let end_date = match &body.end_date {
        Some(s) => Some(parse_date(s).map_err(|e| AppError(domain::DomainError::BadRequest(e)))?),
        None => current.end_date,
    };
    let race_goal = body.race_goal.clone().or(current.race_goal);

    state
        .storage
        .update_training(training_id, user.user_id, name, description, start_date, end_date, race_goal)
        .await?;

    let updated = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(updated))
}

pub async fn delete_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!("DELETE /trainings/{training_id} user={}", user.user_id);

    state
        .storage
        .delete_training(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn add_activity_to_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let (training_id, activity_id) = path.into_inner();
    log::info!(
        "POST /trainings/{training_id}/activities/{activity_id} user={}",
        user.user_id
    );

    state
        .storage
        .add_activity_to_training(training_id, activity_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn remove_activity_from_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let (training_id, activity_id) = path.into_inner();
    log::info!(
        "DELETE /trainings/{training_id}/activities/{activity_id} user={}",
        user.user_id
    );

    state
        .storage
        .remove_activity_from_training(training_id, activity_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn get_training_activities(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!(
        "GET /trainings/{training_id}/activities user={}",
        user.user_id
    );

    let activities = state
        .storage
        .get_training_activities(training_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(activities))
}

pub async fn get_activity_trainings(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let activity_id = path.into_inner();
    log::info!(
        "GET /activities/{activity_id}/trainings user={}",
        user.user_id
    );

    let trainings = state
        .storage
        .get_activity_trainings(activity_id, user.user_id)
        .await?;

    Ok(HttpResponse::Ok().json(trainings))
}
