use actix_web::{HttpResponse, ResponseError};
use domain::DomainError;
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub struct AppError(pub DomainError);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self {
        AppError(e)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let body = ErrorBody {
            error: self.0.to_string(),
        };
        match &self.0 {
            DomainError::NotFound(_) => HttpResponse::NotFound().json(body),
            DomainError::Unauthorized(_) => HttpResponse::Unauthorized().json(body),
            DomainError::BadRequest(_) => HttpResponse::BadRequest().json(body),
            DomainError::Forbidden(_) => HttpResponse::Forbidden().json(body),
            DomainError::StravaRateLimited => {
                HttpResponse::TooManyRequests().json(body)
            }
            DomainError::StravaApi(_) => HttpResponse::BadGateway().json(body),
            DomainError::Auth(_) => HttpResponse::Unauthorized().json(body),
            DomainError::QuotaExhausted(_) => {
                HttpResponse::PaymentRequired().json(body)
            }
            DomainError::Storage(_) | DomainError::Internal(_) => {
                log::error!("Internal error: {}", self.0);
                HttpResponse::InternalServerError().json(ErrorBody {
                    error: "Internal server error".into(),
                })
            }
        }
    }
}
