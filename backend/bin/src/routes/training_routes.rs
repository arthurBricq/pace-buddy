use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use storage::Storage;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use domain::{DomainError, Training};

#[derive(Deserialize)]
pub struct CreateTrainingRequest {
    pub name: String,
    pub description: Option<String>,
}

pub async fn create_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CreateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!(
        "POST /trainings user={} name={}",
        user.user_id,
        body.name
    );

    let training = Training {
        id: Uuid::new_v4(),
        user_id: user.user_id,
        name: body.name.clone(),
        description: body.description.clone(),
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
}

pub async fn update_training(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateTrainingRequest>,
) -> Result<HttpResponse, AppError> {
    let training_id = path.into_inner();
    log::info!(
        "PATCH /trainings/{training_id} user={}",
        user.user_id
    );

    let current = state
        .storage
        .get_training(training_id, user.user_id)
        .await?;

    let name = body.name.clone().unwrap_or(current.name);
    let description = body.description.clone().or(current.description);

    state
        .storage
        .update_training(training_id, user.user_id, name, description)
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
    log::info!("GET /trainings/{training_id}/activities user={}", user.user_id);

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
