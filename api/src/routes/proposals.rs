//! Proposal routes: create, get, list.

use axum::extract::{Path, State};
use axum::Json;
use sqlx::Error as SqlxError;
use std::sync::Arc;

use crate::models::{CreateProposal, ProposalResponse, SignatureResponse};
use crate::store;
use crate::verify;

pub async fn create_proposal(
    State(state): State<Arc<crate::store::AppState>>,
    Json(payload): Json<CreateProposal>,
) -> Result<Json<ProposalResponse>, crate::routes::ApiError> {
    verify::reject_secret_keys(&serde_json::to_value(&payload)?)?;

    let category = match payload.category.as_str() {
        "low" | "medium" | "high" => payload.category.as_str(),
        _ => {
            return Err(crate::routes::ApiError::BadRequest(
                "category must be one of: low, medium, high".to_string(),
            ))
        }
    };

    let now = store::now_seconds();
    if payload.expiry <= now {
        return Err(crate::routes::ApiError::Unprocessable(
            "proposal expiry must be in the future".to_string(),
        ));
    }

    let digest = verify::transaction_digest(&payload.network, &payload.xdr)?;
    if let Some(max_time) = digest.max_time {
        if max_time <= now as u64 {
            return Err(crate::routes::ApiError::Unprocessable(
                "transaction time bounds have passed".to_string(),
            ));
        }
    }

    let proposal = store::create_proposal(&state.db, &payload).await?;

    let account = verify::fetch_account(&state.horizon, &proposal.network, &proposal.account)
        .await
        .map_err(|e| match e {
            crate::verify::VerifyError::AccountNotFound { .. } => {
                crate::routes::ApiError::NotFound(e.to_string())
            }
            _ => crate::routes::ApiError::Upstream(e.to_string()),
        })?;

    let readiness = verify::compute_readiness(&account, &[], category);

    Ok(Json(ProposalResponse {
        id: proposal.id,
        network: proposal.network,
        account: proposal.account,
        expiry: proposal.expiry,
        status: proposal.status,
        signatures: vec![],
        readiness,
    }))
}

pub async fn get_proposal(
    State(state): State<Arc<crate::store::AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ProposalResponse>, crate::routes::ApiError> {
    let proposal = match store::get_proposal(&state.db, id).await {
        Ok(proposal) => proposal,
        Err(SqlxError::RowNotFound) => {
            return Err(crate::routes::ApiError::NotFound(format!(
                "proposal {id} not found"
            )))
        }
        Err(e) => return Err(e.into()),
    };

    let signatures = store::list_signatures(&state.db, id).await?;
    let signature_responses: Vec<SignatureResponse> = signatures
        .iter()
        .map(|s| SignatureResponse {
            id: s.id,
            signer: s.signer.clone(),
            signature: s.signature.clone(),
            created_at: s.created_at,
        })
        .collect();

    let account = verify::fetch_account(&state.horizon, &proposal.network, &proposal.account)
        .await
        .map_err(|e| match e {
            crate::verify::VerifyError::AccountNotFound { .. } => {
                crate::routes::ApiError::NotFound(e.to_string())
            }
            _ => crate::routes::ApiError::Upstream(e.to_string()),
        })?;

    let signed_signers = verify::signed_signers(&account, &signatures);
    let readiness = verify::compute_readiness(&account, &signed_signers, "medium");

    Ok(Json(ProposalResponse {
        id: proposal.id,
        network: proposal.network,
        account: proposal.account,
        expiry: proposal.expiry,
        status: proposal.status,
        signatures: signature_responses,
        readiness,
    }))
}

pub async fn list_proposals(
    State(state): State<Arc<crate::store::AppState>>,
    Path(address): Path<String>,
) -> Result<Json<Vec<crate::models::Proposal>>, crate::routes::ApiError> {
    let proposals = store::list_proposals_by_account(&state.db, &address).await?;
    Ok(Json(proposals))
}
