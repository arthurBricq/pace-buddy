use crate::errors::AppError;
use crate::helpers::conversation_manager;
use crate::helpers::mas_estimator::{build_race_mas_estimates, list_race_activities};
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use actix_web::{web, HttpResponse};
use chrono::{Datelike, Utc};
use domain::{AthleteProfile, DomainError, IdentityProfile};
use serde::{Deserialize, Serialize};
use storage::Storage;
use uuid::Uuid;

pub async fn get_mas(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/mas user_id={}", user.user_id);
    let u = state.storage.get_user_by_id(user.user_id).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "mas_kmh": u.mas_current,
    })))
}

#[derive(Deserialize)]
pub struct UpdateMASRequest {
    pub mas_kmh: Option<f64>,
}

pub async fn update_mas(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<UpdateMASRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!(
        "PATCH /auth/mas user_id={} mas_kmh={:?}",
        user.user_id,
        body.mas_kmh
    );
    state
        .storage
        .update_user_mas(user.user_id, body.mas_kmh)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
    })))
}

pub async fn recompute_mas(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/mas/recompute user_id={}", user.user_id);
    let mas_kmh = state
        .recompute_user_mas_from_races(user.user_id)
        .await?
        .ok_or_else(|| {
            DomainError::BadRequest("No eligible races available to compute MAS".into())
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "mas_kmh": mas_kmh,
    })))
}

pub async fn mas_estimates(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/mas/estimates user_id={}", user.user_id);
    let races = list_race_activities(state.storage.as_ref(), user.user_id).await?;
    let estimates = build_race_mas_estimates(&races);
    Ok(HttpResponse::Ok().json(estimates))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[derive(Deserialize)]
pub struct UpsertIdentityProfileRequest {
    pub name: Option<String>,
    pub age: Option<i32>,
    pub email: Option<String>,
    pub gender: Option<String>,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
}

#[derive(Deserialize)]
pub struct UpsertAthleteProfileRequest {
    pub goal_description: Option<String>,
    pub goal_date: Option<String>,
    pub goal_distance_km: Option<f64>,
    pub goal_target_time_seconds: Option<i32>,
    pub goal_sport_type: Option<String>,
    pub goal_elevation_gain_m: Option<f64>,
    pub additional_info: Option<String>,
}

pub async fn onboarding_status(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/onboarding/status user_id={}", user.user_id);
    let has_identity_profile = state
        .storage
        .get_identity_profile(user.user_id)
        .await?
        .is_some();
    let has_athlete_profile = state
        .storage
        .get_athlete_profile(user.user_id)
        .await?
        .is_some();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "needs_onboarding": !(has_identity_profile && has_athlete_profile),
    })))
}

pub async fn get_identity_profile(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/profile/identity user_id={}", user.user_id);
    let profile = state.storage.get_identity_profile(user.user_id).await?;
    Ok(HttpResponse::Ok().json(profile))
}

pub async fn upsert_identity_profile(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<UpsertIdentityProfileRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("PUT /auth/profile/identity user_id={}", user.user_id);

    if let Some(age) = body.age {
        if !(1..=120).contains(&age) {
            return Err(DomainError::BadRequest("Age must be between 1 and 120".into()).into());
        }
    }
    if let Some(height_cm) = body.height_cm {
        if !(50.0..=260.0).contains(&height_cm) {
            return Err(
                DomainError::BadRequest("Height must be between 50 and 260 cm".into()).into(),
            );
        }
    }
    if let Some(weight_kg) = body.weight_kg {
        if !(20.0..=250.0).contains(&weight_kg) {
            return Err(
                DomainError::BadRequest("Weight must be between 20 and 250 kg".into()).into(),
            );
        }
    }

    let profile = IdentityProfile {
        user_id: user.user_id,
        name: normalize_optional_string(body.name.clone()),
        age: body.age,
        email: normalize_optional_string(body.email.clone()),
        gender: normalize_optional_string(body.gender.clone()),
        height_cm: body.height_cm,
        weight_kg: body.weight_kg,
        updated_at: Utc::now(),
    };

    state.storage.upsert_identity_profile(&profile).await?;
    Ok(HttpResponse::Ok().json(profile))
}

pub async fn get_athlete_profile(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/profile/athlete user_id={}", user.user_id);
    let profile = state.storage.get_athlete_profile(user.user_id).await?;
    Ok(HttpResponse::Ok().json(profile))
}

pub async fn upsert_athlete_profile(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<UpsertAthleteProfileRequest>,
) -> Result<HttpResponse, AppError> {
    log::info!("PUT /auth/profile/athlete user_id={}", user.user_id);

    let goal_date = normalize_optional_string(body.goal_date.clone());
    if let Some(raw_goal_date) = goal_date.as_deref() {
        chrono::NaiveDate::parse_from_str(raw_goal_date, "%Y-%m-%d").map_err(|_| {
            DomainError::BadRequest("Goal date must be in YYYY-MM-DD format".into())
        })?;
    }

    if let Some(goal_distance_km) = body.goal_distance_km {
        if goal_distance_km <= 0.0 {
            return Err(DomainError::BadRequest("Goal distance must be > 0".into()).into());
        }
    }
    if let Some(goal_target_time_seconds) = body.goal_target_time_seconds {
        if goal_target_time_seconds <= 0 {
            return Err(DomainError::BadRequest("Goal target time must be > 0".into()).into());
        }
    }
    if let Some(goal_elevation_gain_m) = body.goal_elevation_gain_m {
        if goal_elevation_gain_m < 0.0 {
            return Err(DomainError::BadRequest("Goal elevation gain must be >= 0".into()).into());
        }
    }

    let goal_sport_type = normalize_optional_string(body.goal_sport_type.clone())
        .map(|value| value.to_lowercase())
        .map(|value| value.replace('-', "_"));
    if let Some(value) = goal_sport_type.as_deref() {
        if value != "running" && value != "trail_running" {
            return Err(DomainError::BadRequest(
                "Goal sport type must be one of: running, trail_running".into(),
            )
            .into());
        }
    }

    let profile = AthleteProfile {
        user_id: user.user_id,
        goal_description: normalize_optional_string(body.goal_description.clone()),
        goal_date,
        goal_distance_km: body.goal_distance_km,
        goal_target_time_seconds: body.goal_target_time_seconds,
        goal_sport_type,
        goal_elevation_gain_m: body.goal_elevation_gain_m,
        additional_info: normalize_optional_string(body.additional_info.clone()),
        updated_at: Utc::now(),
    };

    state.storage.upsert_athlete_profile(&profile).await?;
    Ok(HttpResponse::Ok().json(profile))
}

pub async fn profile(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/profile user_id={}", user.user_id);

    let u = state.storage.get_user_by_id(user.user_id).await?;
    let identity_profile = state.storage.get_identity_profile(user.user_id).await?;
    let athlete_profile = state.storage.get_athlete_profile(user.user_id).await?;

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
        .get_running_stats(
            user.user_id,
            Some(last_year_start),
            Some(this_year_start),
            false,
        )
        .await?;
    let all_time = state
        .storage
        .get_running_stats(user.user_id, None, None, false)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user": u,
        "identity_profile": identity_profile,
        "athlete_profile": athlete_profile,
        "stats": {
            "ytd": ytd,
            "last_year": last_year,
            "all_time": all_time,
        }
    })))
}

#[derive(Serialize)]
pub struct AiCostSummary {
    pub total_cost: f64,
    pub expensive_requests: Vec<ExpensiveRequest>,
}

#[derive(Serialize)]
pub struct ExpensiveRequest {
    pub id: String,
    pub r#type: String, // currently "chat"
    pub title: String,
    pub model: Option<String>,
    pub cost: f64,
    pub created_at: String,
    pub training_id: Option<String>,
}

pub async fn ai_cost_summary(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/ai-cost-summary user_id={}", user.user_id);
    let mut expensive_requests = Vec::new();
    let mut total_cost = 0.0;

    // Get all chats and their messages (chat-level insight cost + message costs)
    let chats = state.storage.list_ai_chats(user.user_id).await?;
    for chat in chats {
        let messages = state.storage.get_ai_chat_messages(chat.id).await?;
        let chat_cost = state.cost_to_user_quota(conversation_manager::effective_chat_cost_raw(
            &chat, &messages,
        ));
        total_cost += chat_cost;
        if chat_cost > 0.0 {
            expensive_requests.push(ExpensiveRequest {
                id: chat.id.to_string(),
                r#type: "chat".to_string(),
                title: chat.title,
                model: Some(chat.model),
                cost: chat_cost,
                created_at: chat.created_at.to_rfc3339(),
                training_id: chat.training_id.map(|id| id.to_string()),
            });
        }
    }

    // Sort by cost descending
    expensive_requests.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(HttpResponse::Ok().json(AiCostSummary {
        total_cost,
        expensive_requests,
    }))
}

pub async fn quota_status(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/quota user_id={}", user.user_id);
    let balance = state.storage.get_user_quota(user.user_id).await?;
    let requests = state.storage.get_user_quota_requests(user.user_id).await?;
    let has_pending = requests
        .iter()
        .any(|r| r.status == domain::QuotaRequestStatus::Pending);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "balance_usd": balance,
        "has_pending_request": has_pending,
        "requests": requests,
    })))
}

pub async fn request_quota(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::info!("POST /auth/quota/request user_id={}", user.user_id);
    let requests = state.storage.get_user_quota_requests(user.user_id).await?;
    if requests
        .iter()
        .any(|r| r.status == domain::QuotaRequestStatus::Pending)
    {
        return Err(DomainError::BadRequest("You already have a pending request".into()).into());
    }

    let req = domain::QuotaRequest {
        id: Uuid::new_v4(),
        user_id: user.user_id,
        status: domain::QuotaRequestStatus::Pending,
        requested_at: chrono::Utc::now(),
        resolved_at: None,
        granted_amount_usd: None,
    };
    state.storage.create_quota_request(&req).await?;
    Ok(HttpResponse::Created().json(req))
}
