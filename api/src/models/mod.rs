//! Data models for the Stellar multisig coordinator API.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A pending multisig proposal stored by the coordinator.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Proposal {
    pub id: i64,
    pub network: String,
    pub account: String,
    pub xdr: String,
    pub expiry: i64,
    pub status: String,
    pub created_at: i64,
}

/// Request to create a new proposal.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProposal {
    pub network: String,
    pub account: String,
    pub xdr: String,
    #[serde(default = "default_category")]
    pub category: String,
    pub expiry: i64,
}

fn default_category() -> String {
    "medium".to_string()
}

/// A single signature collected for a proposal.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Signature {
    pub id: i64,
    pub proposal_id: i64,
    pub signer: String,
    pub signature: String,
    pub created_at: i64,
}

/// Request to add a signer's signature to a proposal.
#[derive(Debug, Deserialize, Serialize)]
pub struct AddSignature {
    pub signer: String,
    pub signature: String,
}

/// Readiness of a proposal against the account threshold.
#[derive(Debug, Serialize)]
pub struct Readiness {
    pub category: String,
    pub signed_weight: u32,
    pub threshold: u32,
    pub threshold_met: bool,
}

/// Full proposal detail returned to the client.
#[derive(Debug, Serialize)]
pub struct ProposalResponse {
    pub id: i64,
    pub network: String,
    pub account: String,
    pub expiry: i64,
    pub status: String,
    pub signatures: Vec<SignatureResponse>,
    pub readiness: Readiness,
}

/// Signature returned to the client.
#[derive(Debug, Serialize)]
pub struct SignatureResponse {
    pub id: i64,
    pub signer: String,
    pub signature: String,
    pub created_at: i64,
}

/// One registered signer of an account, fetched from the network.
#[derive(Debug, Serialize, Clone)]
pub struct Signer {
    pub key: String,
    pub weight: u32,
}

/// Thresholds for an account, fetched from the network.
#[derive(Debug, Serialize, Clone)]
pub struct Thresholds {
    pub low: u32,
    pub medium: u32,
    pub high: u32,
}

/// Account signing metadata fetched live from Horizon.
#[derive(Debug, Serialize, Clone)]
pub struct AccountInfo {
    pub address: String,
    pub signers: Vec<Signer>,
    pub thresholds: Thresholds,
}
