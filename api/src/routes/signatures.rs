//! Signature routes: add a signature, submit a proposal.

use axum::extract::{Path, State};
use axum::Json;
use sqlx::Error as SqlxError;
use std::sync::Arc;

use crate::models::AddSignature;
use crate::store;
use crate::verify;

pub async fn add_signature(
    State(state): State<Arc<crate::store::AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<AddSignature>,
) -> Result<Json<crate::models::SignatureResponse>, crate::routes::ApiError> {
    verify::reject_secret_keys(&serde_json::to_value(&payload)?)?;

    let proposal = match store::get_proposal(&state.db, id).await {
        Ok(proposal) => proposal,
        Err(SqlxError::RowNotFound) => {
            return Err(crate::routes::ApiError::NotFound(format!(
                "proposal {id} not found"
            )))
        }
        Err(e) => return Err(e.into()),
    };

    if proposal.status != "pending" {
        return Err(crate::routes::ApiError::Conflict(
            "proposal is not open for signatures".to_string(),
        ));
    }

    let now = store::now_seconds();
    if proposal.expiry <= now {
        return Err(crate::routes::ApiError::Unprocessable(
            "proposal has expired".to_string(),
        ));
    }

    // Signer validity is always checked against live network state, never
    // against anything the client sends.
    let account = verify::fetch_account(&state.horizon, &proposal.network, &proposal.account)
        .await
        .map_err(|e| match e {
            crate::verify::VerifyError::AccountNotFound { .. } => {
                crate::routes::ApiError::NotFound(e.to_string())
            }
            _ => crate::routes::ApiError::Upstream(e.to_string()),
        })?;

    if verify::find_signer(&account, &payload.signer).is_none() {
        return Err(crate::routes::ApiError::Unprocessable(
            "signer is not a current signer of the account".to_string(),
        ));
    }

    if store::signature_exists(&state.db, id, &payload.signer).await? {
        return Err(crate::routes::ApiError::Conflict(
            "duplicate signature from this signer".to_string(),
        ));
    }

    let digest = verify::transaction_digest(&proposal.network, &proposal.xdr)?;
    verify::verify_ed25519(&payload.signature, &payload.signer, &digest.hash)?;

    let signature = store::add_signature(&state.db, id, &payload).await?;

    let all_signatures = store::list_signatures(&state.db, id).await?;
    let signed_signers = verify::signed_signers(&account, &all_signatures);
    let _readiness = verify::compute_readiness(&account, &signed_signers, "medium");

    Ok(Json(crate::models::SignatureResponse {
        id: signature.id,
        signer: signature.signer,
        signature: signature.signature,
        created_at: signature.created_at,
    }))
}

pub async fn submit_proposal(
    State(state): State<Arc<crate::store::AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, crate::routes::ApiError> {
    let proposal = match store::get_proposal(&state.db, id).await {
        Ok(proposal) => proposal,
        Err(SqlxError::RowNotFound) => {
            return Err(crate::routes::ApiError::NotFound(format!(
                "proposal {id} not found"
            )))
        }
        Err(e) => return Err(e.into()),
    };

    if proposal.status == "submitted" {
        return Err(crate::routes::ApiError::Conflict("proposal already submitted".to_string()));
    }

    let now = store::now_seconds();
    if proposal.expiry <= now {
        return Err(crate::routes::ApiError::Unprocessable(
            "proposal has expired".to_string(),
        ));
    }

    let digest = verify::transaction_digest(&proposal.network, &proposal.xdr)?;
    if let Some(max_time) = digest.max_time {
        if max_time <= now as u64 {
            return Err(crate::routes::ApiError::Unprocessable(
                "transaction time bounds have passed".to_string(),
            ));
        }
    }

    // Re-check readiness against fresh network state immediately before
    // submitting: signers or thresholds may have changed since signing.
    let account = verify::fetch_account(&state.horizon, &proposal.network, &proposal.account)
        .await
        .map_err(|e| match e {
            crate::verify::VerifyError::AccountNotFound { .. } => {
                crate::routes::ApiError::NotFound(e.to_string())
            }
            _ => crate::routes::ApiError::Upstream(e.to_string()),
        })?;

    let signatures = store::list_signatures(&state.db, id).await?;
    let signed_signers = verify::signed_signers(&account, &signatures);
    let readiness = verify::compute_readiness(&account, &signed_signers, "medium");

    if !readiness.threshold_met {
        return Err(crate::routes::ApiError::Unprocessable(format!(
            "threshold not met: signed weight {} < threshold {}",
            readiness.signed_weight, readiness.threshold
        )));
    }

    let _submitted = store::mark_submitted(&state.db, id).await?;

    Ok(Json(serde_json::json!({
        "status": "submitted",
        "proposal_id": id,
        "xdr": proposal.xdr,
        "readiness": readiness,
    })))
}
