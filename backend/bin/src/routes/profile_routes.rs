use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::state::AppState;
use actix_web::{web, HttpResponse};
use chrono::Datelike;
use domain::DomainError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use storage::Storage;

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
    log::info!(
        "PATCH /auth/mas user_id={} mas_mps={:?}",
        user.user_id,
        body.mas_mps
    );
    state
        .storage
        .update_user_mas(user.user_id, body.mas_mps)
        .await?;
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
    pub r#type: String, // "insight" or "chat"
    pub title: String,
    pub model: Option<String>,
    pub cost: f64,
    pub created_at: String,
    pub training_id: Option<String>, // For insights, link to training
}

pub async fn ai_cost_summary(
    state: web::Data<AppState>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    log::debug!("GET /auth/ai-cost-summary user_id={}", user.user_id);
    let mut expensive_requests = Vec::new();
    let mut total_cost = 0.0;

    // Get all training insights
    let trainings = state.storage.list_trainings(user.user_id).await?;
    for training in trainings {
        let insights = state
            .storage
            .get_training_insights(training.id, user.user_id)
            .await?;
        for insight in insights {
            if let Some(cost) = insight.cost {
                let user_cost = state.cost_to_user_quota(cost);
                total_cost += user_cost;
                expensive_requests.push(ExpensiveRequest {
                    id: insight.id.to_string(),
                    r#type: "insight".to_string(),
                    title: insight.display_label,
                    model: insight.model,
                    cost: user_cost,
                    created_at: insight.created_at.to_rfc3339(),
                    training_id: Some(training.id.to_string()),
                });
            }
        }
    }

    // Get all chats and their messages
    let chats = state.storage.list_ai_chats(user.user_id).await?;
    for chat in chats {
        let messages = state.storage.get_ai_chat_messages(chat.id).await?;
        let chat_cost: f64 = messages
            .iter()
            .map(|m| state.cost_to_user_quota(m.cost))
            .sum::<f64>();
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

