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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateProposal;

    async fn test_pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_proposal() {
        let pool = test_pool().await;
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let proposal = create_proposal(
            &pool,
            &CreateProposal {
                network: "testnet".to_string(),
                account: "GD...".to_string(),
                xdr: "AAAA".to_string(),
                category: "medium".to_string(),
                expiry: 9999999999,
            },
        )
        .await
        .unwrap();

        assert_eq!(proposal.network, "testnet");
        assert_eq!(proposal.status, "pending");
        assert_eq!(proposal.id, 1);

        let fetched = get_proposal(&pool, proposal.id).await.unwrap();
        assert_eq!(fetched.id, proposal.id);
    }

    #[tokio::test]
    async fn test_add_and_list_signatures() {
        let pool = test_pool().await;
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let proposal = create_proposal(
            &pool,
            &CreateProposal {
                network: "testnet".to_string(),
                account: "GD...".to_string(),
                xdr: "AAAA".to_string(),
                category: "medium".to_string(),
                expiry: 9999999999,
            },
        )
        .await
        .unwrap();

        let sig = add_signature(
            &pool,
            proposal.id,
            &crate::models::AddSignature {
                signer: "GA...".to_string(),
                signature: "BASE64".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(sig.signer, "GA...");
        assert_eq!(sig.proposal_id, proposal.id);

        let sigs = list_signatures(&pool, proposal.id).await.unwrap();
        assert_eq!(sigs.len(), 1);

        let exists = signature_exists(&pool, proposal.id, "GA...").await.unwrap();
        assert!(exists);

        let dup = signature_exists(&pool, proposal.id, "GB...").await.unwrap();
        assert!(!dup);
    }
}
