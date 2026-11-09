//! Account routes: live signer and threshold lookup from Horizon.

use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use crate::verify;

pub async fn get_account(
    State(state): State<Arc<crate::store::AppState>>,
    Path(address): Path<String>,
) -> Result<Json<crate::models::AccountInfo>, crate::routes::ApiError> {
    let account = verify::fetch_account(&state.horizon, &state.network, &address)
        .await
        .map_err(|e| match e {
            crate::verify::VerifyError::AccountNotFound { .. } => {
                crate::routes::ApiError::NotFound(e.to_string())
            }
            _ => crate::routes::ApiError::Upstream(e.to_string()),
        })?;
    Ok(Json(account))
}
