//! SQLite persistence layer for proposals and signatures.

use crate::models::{Proposal, Signature};
use sqlx::sqlite::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AppState {
    pub db: SqlitePool,
    pub horizon: String,
    pub network: String,
}

pub(crate) fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs() as i64
}

pub async fn create_proposal(
    pool: &SqlitePool,
    proposal: &crate::models::CreateProposal,
) -> Result<Proposal, sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO proposals (network, account, xdr, expiry, status, created_at)
           VALUES (?1, ?2, ?3, ?4, 'pending', ?5)"#,
    )
    .bind(&proposal.network)
    .bind(&proposal.account)
    .bind(&proposal.xdr)
    .bind(proposal.expiry)
    .bind(now_seconds())
    .execute(pool)
    .await?;

    let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()").fetch_one(pool).await?;
    get_proposal(pool, id).await
}

pub async fn get_proposal(pool: &SqlitePool, id: i64) -> Result<Proposal, sqlx::Error> {
    sqlx::query_as::<_, Proposal>("SELECT * FROM proposals WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn list_proposals_by_account(
    pool: &SqlitePool,
    account: &str,
) -> Result<Vec<Proposal>, sqlx::Error> {
    sqlx::query_as::<_, Proposal>("SELECT * FROM proposals WHERE account = ?1 ORDER BY id DESC")
        .bind(account)
        .fetch_all(pool)
        .await
}

pub async fn signature_exists(pool: &SqlitePool, proposal_id: i64, signer: &str) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM signatures WHERE proposal_id = ?1 AND signer = ?2",
    )
    .bind(proposal_id)
    .bind(signer)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

pub async fn add_signature(
    pool: &SqlitePool,
    proposal_id: i64,
    signature: &crate::models::AddSignature,
) -> Result<Signature, sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO signatures (proposal_id, signer, signature, created_at)
           VALUES (?1, ?2, ?3, ?4)"#,
    )
    .bind(proposal_id)
    .bind(&signature.signer)
    .bind(&signature.signature)
    .bind(now_seconds())
    .execute(pool)
    .await?;

    let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()").fetch_one(pool).await?;
    Ok(Signature {
        id,
        proposal_id,
        signer: signature.signer.clone(),
        signature: signature.signature.clone(),
        created_at: now_seconds(),
    })
}

pub async fn list_signatures(
    pool: &SqlitePool,
    proposal_id: i64,
) -> Result<Vec<Signature>, sqlx::Error> {
    sqlx::query_as::<_, Signature>("SELECT * FROM signatures WHERE proposal_id = ?1 ORDER BY id ASC")
        .bind(proposal_id)
        .fetch_all(pool)
        .await
}

pub async fn mark_submitted(pool: &SqlitePool, id: i64) -> Result<Proposal, sqlx::Error> {
    sqlx::query("UPDATE proposals SET status = 'submitted' WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    get_proposal(pool, id).await
}
