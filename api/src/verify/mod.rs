//! Signature and threshold verification — the security core of the coordinator.
//!
//! The coordinator never trusts client claims. Every signature is re-verified
//! against a transaction hash recomputed from the stored XDR and the network
//! passphrase, and every signer is validated against live network state.

use crate::models::{AccountInfo, Readiness, Signer, Thresholds};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519::Signature as EdSignature;
use ed25519_dalek::VerifyingKey;
use sha2::{Digest, Sha256};
use signature::Verifier;
use stellar_xdr::{Limited, Limits, ReadXdr};
use thiserror::Error;

/// strkey version bytes (G = public key, S = secret seed).
const VERSION_PUBLIC_KEY: u8 = 0x30;
const VERSION_SECRET_KEY: u8 = 0x90;

/// Base32 alphabet used by Stellar strkeys (RFC 4648, no padding).
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("invalid transaction XDR: {0}")]
    InvalidXdr(String),
    #[error("invalid Stellar public key: {0}")]
    InvalidPublicKey(String),
    #[error("secret keys are not accepted in requests")]
    SecretKeyRejected,
    #[error("invalid signature encoding")]
    InvalidSignature,
    #[error("ed25519 signature verification failed")]
    SignatureInvalid,
    #[error("public key {signer} is not a current signer of account {account}")]
    NotASigner { signer: String, account: String },
    #[error("duplicate signature from signer {0}")]
    DuplicateSignature(String),
    #[error("network request failed: {0}")]
    NetworkError(String),
    #[error("account {account} not found on network {network}")]
    AccountNotFound { account: String, network: String },
    #[error("proposal has expired")]
    Expired,
    #[error("transaction time bounds have passed")]
    TimeBoundsExpired,
    #[error("threshold not met: signed weight {signed_weight} < threshold {threshold}")]
    ThresholdNotMet { signed_weight: u32, threshold: u32 },
    #[error("proposal not found")]
    NotFound,
}

pub fn network_passphrase(network: &str) -> String {
    match network {
        "testnet" => "Test SDF Network ; September 2015".to_string(),
        "pubnet" | "mainnet" => "Public Global Stellar Network ; September 2015".to_string(),
        other => other.to_string(),
    }
}

/// Result of validating a transaction envelope: its signature hash and the
/// latest time bound, if any.
pub struct TransactionDigest {
    pub hash: [u8; 32],
    pub max_time: Option<u64>,
}

/// Recompute the transaction hash from the stored XDR and network passphrase.
/// The hash is computed over the transaction envelope with its signatures
/// cleared, mixed with the network id (sha256 of the passphrase), matching
/// how Stellar signs transactions.
pub fn transaction_digest(network: &str, xdr: &str) -> Result<TransactionDigest, VerifyError> {
    let raw = STANDARD
        .decode(xdr)
        .map_err(|e| VerifyError::InvalidXdr(e.to_string()))?;
    let mut reader = raw.as_slice();
    let mut limited = Limited::new(&mut reader, Limits::none());
    let envelope: stellar_xdr::TransactionEnvelope =
        ReadXdr::read_xdr(&mut limited).map_err(|e| VerifyError::InvalidXdr(e.to_string()))?;

    let max_time = match &envelope {
        stellar_xdr::TransactionEnvelope::Tx(env) => match &env.tx.cond {
            stellar_xdr::Preconditions::Time(bounds) => Some(bounds.max_time.0),
            _ => None,
        },
        stellar_xdr::TransactionEnvelope::TxV0(env) => match &env.tx.time_bounds {
            Some(bounds) => Some(bounds.max_time.0),
            _ => None,
        },
        stellar_xdr::TransactionEnvelope::TxFeeBump(env) => match &env.tx.inner_tx {
            stellar_xdr::FeeBumpTransactionInnerTx::Tx(inner) => match &inner.tx.cond {
                stellar_xdr::Preconditions::Time(bounds) => Some(bounds.max_time.0),
                _ => None,
            },
        },
    };

    // Compute the Stellar transaction hash over the envelope with signatures
    // cleared, mixed with sha256(network passphrase) as the network id.
    let network_id: [u8; 32] = Sha256::digest(network_passphrase(network).as_bytes()).into();
    let hash: [u8; 32] = envelope
        .hash(network_id)
        .map_err(|e| VerifyError::InvalidXdr(e.to_string()))?;

    Ok(TransactionDigest { hash, max_time })
}

/// Decode a Stellar strkey ed25519 public key (`G...`) into its raw 32 bytes.
pub fn decode_public_key(strkey: &str) -> Result<[u8; 32], VerifyError> {
    let pk = stellar_strkey::ed25519::PublicKey::from_string(strkey)
        .map_err(|_| VerifyError::InvalidPublicKey(strkey.to_string()))?;
    Ok(pk.0)
}

/// Reject any request whose string fields contain a Stellar secret seed (`S...`).
/// The coordinator never receives or stores secret key material.
pub fn reject_secret_keys(value: &serde_json::Value) -> Result<(), VerifyError> {
    if contains_secret_key(value) {
        return Err(VerifyError::SecretKeyRejected);
    }
    Ok(())
}

fn contains_secret_key(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => is_secret_strkey(s),
        serde_json::Value::Array(items) => items.iter().any(contains_secret_key),
        serde_json::Value::Object(map) => map.values().any(contains_secret_key),
        _ => false,
    }
}

fn is_secret_strkey(s: &str) -> bool {
    if !s.starts_with('S') {
        return false;
    }
    let Ok(bytes) = base32_decode(s) else { return false };
    if bytes.len() != 35 || bytes[0] != VERSION_SECRET_KEY {
        return false;
    }
    let expected = u16::from_be_bytes([bytes[33], bytes[34]]);
    crc16_xmodem(&bytes[..33]) == expected
}

fn base32_decode(input: &str) -> Result<Vec<u8>, ()> {
    let mut buffer: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::new();
    for ch in input.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'2'..=b'7' => ch - b'2' + 26,
            _ => return Err(()),
        };
        buffer = (buffer << 5) | (v as u32);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// XMODEM CRC-16 (poly 0x1021, init 0), as used by Stellar strkeys.
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for byte in data {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Verify an ed25519 signature (base64) over the 32-byte transaction hash by
/// the given Stellar public key strkey.
pub fn verify_ed25519(
    signature_b64: &str,
    public_key_str: &str,
    hash: &[u8; 32],
) -> Result<(), VerifyError> {
    let sig_bytes = STANDARD
        .decode(signature_b64)
        .map_err(|_| VerifyError::InvalidSignature)?;
    if sig_bytes.len() != 64 {
        return Err(VerifyError::InvalidSignature);
    }
    let signature = EdSignature::from_slice(&sig_bytes).map_err(|_| VerifyError::InvalidSignature)?;
    let pk_bytes = decode_public_key(public_key_str)?;
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| VerifyError::InvalidPublicKey(public_key_str.to_string()))?;
    verifying_key.verify(hash, &signature).map_err(|_| VerifyError::SignatureInvalid)?;
    Ok(())
}

/// Find a signer by key in live account data.
pub fn find_signer<'a>(account: &'a AccountInfo, signer: &str) -> Option<&'a Signer> {
    account.signers.iter().find(|s| s.key == signer)
}

/// Look up an account's signers and thresholds live from Horizon.
pub async fn fetch_account(
    horizon: &str,
    network: &str,
    address: &str,
) -> Result<AccountInfo, VerifyError> {
    let url = format!("{}/accounts/{}", horizon.trim_end_matches('/'), address);
    let response = reqwest::get(url)
        .await
        .map_err(|e| VerifyError::NetworkError(e.to_string()))?;
    if response.status().is_client_error() && response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(VerifyError::AccountNotFound {
            account: address.to_string(),
            network: network.to_string(),
        });
    }
    if !response.status().is_success() {
        return Err(VerifyError::NetworkError(format!(
            "Horizon returned {} for {}",
            response.status(),
            address
        )));
    }
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| VerifyError::NetworkError(e.to_string()))?;

    let thresholds = value
        .get("thresholds")
        .and_then(|t| t.as_object())
        .ok_or_else(|| VerifyError::NetworkError(format!("missing thresholds for {address}")))?;
    let low = thresholds
        .get("low_threshold")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let medium = thresholds
        .get("medium_threshold")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let high = thresholds
        .get("high_threshold")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let signers = value
        .get("signers")
        .and_then(|s| s.as_array())
        .ok_or_else(|| VerifyError::NetworkError(format!("missing signers for {address}")))?
        .iter()
        .map(|entry| {
            let key = entry
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| VerifyError::NetworkError("signer missing key".to_string()))?
                .to_string();
            let weight = entry
                .get("weight")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            Ok(Signer { key, weight })
        })
        .collect::<Result<Vec<Signer>, VerifyError>>()?;

    Ok(AccountInfo {
        address: address.to_string(),
        signers,
        thresholds: Thresholds { low, medium, high },
    })
}

/// Sum the weights of the given signers and compare against the account's
/// threshold for the operation category (low/medium/high).
pub fn compute_readiness(
    account: &AccountInfo,
    signed_signers: &[Signer],
    category: &str,
) -> Readiness {
    let threshold = match category {
        "low" => account.thresholds.low,
        "high" => account.thresholds.high,
        _ => account.thresholds.medium,
    };
    let signed_weight: u32 = signed_signers.iter().map(|s| s.weight).sum();
    Readiness {
        category: category.to_string(),
        signed_weight,
        threshold,
        threshold_met: signed_weight >= threshold,
    }
}

/// Collect the signers (with live weights) that have signatures on a proposal.
pub fn signed_signers(
    account: &AccountInfo,
    signatures: &[crate::models::Signature],
) -> Vec<Signer> {
    let mut out = Vec::new();
    for signature in signatures {
        if let Some(signer) = find_signer(account, &signature.signer) {
            out.push(signer.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreateProposal, Signature};
    use ed25519_dalek::SigningKey;
    use signature::Signer as _;
    use stellar_strkey::ed25519::{PrivateKey as StrkeyPrivateKey, PublicKey as StrkeyPublicKey};

    fn make_keypair() -> (SigningKey, [u8; 32]) {
        // Deterministic seed for reproducible tests.
        let seed = [0x42u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key: [u8; 32] = signing_key.verifying_key().to_bytes();
        (signing_key, verifying_key)
    }

    fn make_keypair2() -> (SigningKey, [u8; 32]) {
        let seed = [0x43u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key: [u8; 32] = signing_key.verifying_key().to_bytes();
        (signing_key, verifying_key)
    }

    fn public_key_strkey(pk: &[u8; 32]) -> String {
        StrkeyPublicKey::from_payload(pk)
            .unwrap()
            .to_string()
            .as_str()
            .to_string()
    }

    fn secret_key_strkey(pk: &[u8; 32]) -> String {
        let sk = StrkeyPrivateKey::from_payload(pk).unwrap();
        let mut binary = Vec::with_capacity(35);
        binary.push(VERSION_SECRET_KEY);
        binary.extend_from_slice(&sk.0);
        let crc = crc16_xmodem(&binary);
        binary.extend_from_slice(&crc.to_be_bytes());
        let mut buffer: u32 = 0;
        let mut bits = 0;
        let mut out = String::new();
        for byte in &binary {
            buffer = (buffer << 8) | (*byte as u32);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                let idx = ((buffer >> bits) & 0x1f) as usize;
                out.push(BASE32_ALPHABET[idx] as char);
            }
        }
        if bits > 0 {
            out.push(BASE32_ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
        }
        out
    }

    fn make_test_xdr() -> String {
        use stellar_xdr::{
            AccountId, Limits, Memo, MuxedAccount, Preconditions, SequenceNumber, Transaction,
            TransactionEnvelope, TransactionExt, TransactionV1Envelope, WriteXdr,
        };

        let tx = Transaction {
            source_account: MuxedAccount::from(AccountId::default()),
            fee: 100,
            seq_num: SequenceNumber(1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: Default::default(),
            ext: TransactionExt::V0,
        };
        let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx,
            signatures: Default::default(),
        });
        let mut encoded = Vec::new();
        let mut limited = Limited::new(&mut encoded, Limits::none());
        envelope.write_xdr(&mut limited).unwrap();
        STANDARD.encode(&encoded)
    }

    #[test]
    fn test_decode_public_key_valid() {
        let (_sk, pk) = make_keypair();
        let strkey = public_key_strkey(&pk);
        let decoded = decode_public_key(&strkey).unwrap();
        assert_eq!(decoded, pk);
    }

    #[test]
    fn test_decode_public_key_rejects_secret_key() {
        let (_sk, pk) = make_keypair();
        let strkey = secret_key_strkey(&pk);
        assert!(decode_public_key(&strkey).is_err());
    }

    #[test]
    fn test_verify_ed25519_valid() {
        let (signing_key, pk) = make_keypair();
        let strkey = public_key_strkey(&pk);
        let hash = [42u8; 32];
        let signature = signing_key.sign(&hash);
        let sig_b64 = STANDARD.encode(signature.to_bytes());
        assert!(verify_ed25519(&sig_b64, &strkey, &hash).is_ok());
    }

    #[test]
    fn test_verify_ed25519_invalid_signature() {
        let (_sk, pk) = make_keypair();
        let strkey = public_key_strkey(&pk);
        let hash = [42u8; 32];
        let sig_b64 = STANDARD.encode([0u8; 64]);
        assert!(verify_ed25519(&sig_b64, &strkey, &hash).is_err());
    }

    #[test]
    fn test_verify_ed25519_wrong_key() {
        let (signing_key, pk) = make_keypair();
        let (signing_key2, pk2) = make_keypair2();
        let strkey2 = public_key_strkey(&pk2);
        let hash = [42u8; 32];
        let signature = signing_key.sign(&hash);
        let sig_b64 = STANDARD.encode(signature.to_bytes());
        // Signed by pk but verified against pk2
        assert!(verify_ed25519(&sig_b64, &strkey2, &hash).is_err());
    }

    #[test]
    fn test_transaction_digest() {
        let xdr = make_test_xdr();
        let digest = transaction_digest("testnet", &xdr).unwrap();
        assert_eq!(digest.hash.len(), 32);
        let digest2 = transaction_digest("testnet", &xdr).unwrap();
        assert_eq!(digest.hash, digest2.hash);
    }

    #[test]
    fn test_transaction_digest_different_networks() {
        let xdr = make_test_xdr();
        let d1 = transaction_digest("testnet", &xdr).unwrap();
        let d2 = transaction_digest("pubnet", &xdr).unwrap();
        assert_ne!(d1.hash, d2.hash);
    }

    #[test]
    fn test_transaction_digest_invalid_xdr() {
        assert!(transaction_digest("testnet", "not-valid-xdr").is_err());
    }

    #[test]
    fn test_compute_readiness_below_threshold() {
        let account = AccountInfo {
            address: "GAAAAA...".to_string(),
            signers: vec![
                Signer { key: "A".to_string(), weight: 1 },
                Signer { key: "B".to_string(), weight: 1 },
                Signer { key: "C".to_string(), weight: 1 },
            ],
            thresholds: Thresholds { low: 1, medium: 2, high: 3 },
        };
        let signed = vec![
            Signer { key: "A".to_string(), weight: 1 },
            Signer { key: "B".to_string(), weight: 1 },
        ];
        let readiness = compute_readiness(&account, &signed, "high");
        assert_eq!(readiness.signed_weight, 2);
        assert_eq!(readiness.threshold, 3);
        assert!(!readiness.threshold_met);
    }

    #[test]
    fn test_compute_readiness_at_threshold() {
        let account = AccountInfo {
            address: "GAAAAA...".to_string(),
            signers: vec![
                Signer { key: "A".to_string(), weight: 1 },
                Signer { key: "B".to_string(), weight: 1 },
                Signer { key: "C".to_string(), weight: 1 },
            ],
            thresholds: Thresholds { low: 1, medium: 2, high: 3 },
        };
        let signed = vec![
            Signer { key: "A".to_string(), weight: 1 },
            Signer { key: "B".to_string(), weight: 1 },
            Signer { key: "C".to_string(), weight: 1 },
        ];
        let readiness = compute_readiness(&account, &signed, "high");
        assert_eq!(readiness.signed_weight, 3);
        assert_eq!(readiness.threshold, 3);
        assert!(readiness.threshold_met);
    }

    #[test]
    fn test_compute_readiness_weighted_signers() {
        let account = AccountInfo {
            address: "GAAAAA...".to_string(),
            signers: vec![
                Signer { key: "A".to_string(), weight: 10 },
                Signer { key: "B".to_string(), weight: 1 },
                Signer { key: "C".to_string(), weight: 1 },
            ],
            thresholds: Thresholds { low: 1, medium: 5, high: 10 },
        };
        let signed = vec![Signer { key: "A".to_string(), weight: 10 }];
        let readiness = compute_readiness(&account, &signed, "high");
        assert_eq!(readiness.signed_weight, 10);
        assert!(readiness.threshold_met);

        let signed = vec![
            Signer { key: "B".to_string(), weight: 1 },
            Signer { key: "C".to_string(), weight: 1 },
        ];
        let readiness = compute_readiness(&account, &signed, "high");
        assert_eq!(readiness.signed_weight, 2);
        assert!(!readiness.threshold_met);
    }

    #[test]
    fn test_compute_readiness_medium_category() {
        let account = AccountInfo {
            address: "GAAAAA...".to_string(),
            signers: vec![Signer { key: "A".to_string(), weight: 2 }],
            thresholds: Thresholds { low: 1, medium: 5, high: 10 },
        };
        let signed = vec![Signer { key: "A".to_string(), weight: 2 }];
        let readiness = compute_readiness(&account, &signed, "medium");
        assert_eq!(readiness.threshold, 5);
        assert!(!readiness.threshold_met);
        let readiness = compute_readiness(&account, &signed, "low");
        assert_eq!(readiness.threshold, 1);
        assert!(readiness.threshold_met);
    }

    #[test]
    fn test_signed_signers_filters_unknown() {
        let account = AccountInfo {
            address: "GAAAAA...".to_string(),
            signers: vec![Signer { key: "A".to_string(), weight: 1 }],
            thresholds: Thresholds { low: 1, medium: 1, high: 1 },
        };
        let signatures = vec![
            Signature {
                id: 1,
                proposal_id: 1,
                signer: "A".to_string(),
                signature: "sig1".to_string(),
                created_at: 100,
            },
            Signature {
                id: 2,
                proposal_id: 1,
                signer: "Z".to_string(),
                signature: "sig2".to_string(),
                created_at: 101,
            },
        ];
        let result = signed_signers(&account, &signatures);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "A");
    }

    #[test]
    fn test_reject_secret_keys_detects_s_strkey() {
        let (_sk, pk) = make_keypair();
        let value = serde_json::json!({
            "network": "testnet",
            "account": "GAAAAA...",
            "secret": secret_key_strkey(&pk)
        });
        assert!(reject_secret_keys(&value).is_err());
    }

    #[test]
    fn test_reject_secret_keys_allows_public_keys() {
        let value = serde_json::json!({
            "network": "testnet",
            "account": "GAAAAA...",
            "xdr": "AAAA..."
        });
        assert!(reject_secret_keys(&value).is_ok());
    }

    #[test]
    fn test_reject_secret_keys_detects_nested() {
        let (_sk, pk) = make_keypair();
        let value = serde_json::json!({
            "network": "testnet",
            "signatures": [
                { "signer": public_key_strkey(&pk), "signature": "AAAA" },
                { "signer": secret_key_strkey(&pk), "signature": "BBBB" }
            ]
        });
        assert!(reject_secret_keys(&value).is_err());
    }

    #[test]
    fn test_network_passphrase_testnet() {
        assert_eq!(
            network_passphrase("testnet"),
            "Test SDF Network ; September 2015"
        );
    }

    #[test]
    fn test_network_passphrase_pubnet() {
        assert_eq!(
            network_passphrase("pubnet"),
            "Public Global Stellar Network ; September 2015"
        );
    }

    #[test]
    fn test_create_proposal_struct() {
        let _ = CreateProposal {
            network: "testnet".to_string(),
            account: "GAAAAA...".to_string(),
            xdr: "AAAA".to_string(),
            category: "medium".to_string(),
            expiry: 9999999999,
        };
    }
}
