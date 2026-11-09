//! HTTP routes and error handling for the coordinator API.

use std::sync::Arc;

use axum::{
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::store::AppState;

mod accounts;
mod proposals;
mod signatures;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("unprocessable: {0}")]
    Unprocessable(String),
    #[error("upstream: {0}")]
    Upstream(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        ApiError::Internal(error.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        ApiError::Internal(error.to_string())
    }
}

impl From<crate::verify::VerifyError> for ApiError {
    fn from(error: crate::verify::VerifyError) -> Self {
        match error {
            crate::verify::VerifyError::NotFound => ApiError::NotFound(error.to_string()),
            crate::verify::VerifyError::NotASigner { .. }
            | crate::verify::VerifyError::DuplicateSignature { .. }
            | crate::verify::VerifyError::SignatureInvalid
            | crate::verify::VerifyError::Expired
            | crate::verify::VerifyError::TimeBoundsExpired
            | crate::verify::VerifyError::ThresholdNotMet { .. } => {
                ApiError::Unprocessable(error.to_string())
            }
            crate::verify::VerifyError::AccountNotFound { .. }
            | crate::verify::VerifyError::NetworkError(_) => ApiError::Upstream(error.to_string()),
            crate::verify::VerifyError::InvalidXdr(_)
            | crate::verify::VerifyError::InvalidPublicKey(_)
            | crate::verify::VerifyError::InvalidSignature
            | crate::verify::VerifyError::SecretKeyRejected => {
                ApiError::BadRequest(error.to_string())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::BadRequest(_) => axum::http::StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => axum::http::StatusCode::CONFLICT,
            ApiError::Unprocessable(_) => axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Upstream(_) => axum::http::StatusCode::BAD_GATEWAY,
            ApiError::Internal(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(ErrorBody { error: self.to_string() })).into_response()
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/proposals", post(crate::routes::proposals::create_proposal))
        .route("/proposals/:id", get(crate::routes::proposals::get_proposal))
        .route(
            "/proposals/:id/signatures",
            post(crate::routes::signatures::add_signature),
        )
        .route(
            "/proposals/:id/submit",
            post(crate::routes::signatures::submit_proposal),
        )
        .route(
            "/accounts/:address/proposals",
            get(crate::routes::proposals::list_proposals),
        )
        .route("/accounts/:address", get(crate::routes::accounts::get_account))
        .with_state(state)
}
