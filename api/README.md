# Stellar Multisig Coordinator API

Rust + Axum + tokio + sqlx (SQLite) coordinator service for Stellar multisig
proposal signing.

## Build

```bash
cargo build --release
```

## Test

```bash
cargo test
```

## Run

```bash
DATABASE_URL=sqlite:./multisig.db \
HORIZON_URL=https://horizon-testnet.stellar.org \
STELLAR_NETWORK=testnet \
BIND_ADDR=0.0.0.0:3000 \
cargo run
```

## Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/proposals` | Create a proposal |
| `GET` | `/proposals/:id` | Proposal detail |
| `GET` | `/accounts/:address/proposals` | List proposals |
| `POST` | `/proposals/:id/signatures` | Add a signature |
| `POST` | `/proposals/:id/submit` | Submit if threshold met |
| `GET` | `/accounts/:address` | Account signers/thresholds |

## Examples

### Create a proposal

```bash
curl -X POST http://localhost:3000/proposals \
  -H "Content-Type: application/json" \
  -d '{
    "network": "testnet",
    "account": "GD...multisig-account...",
    "xdr": "AAAA...base64-encoded-transaction-envelope...",
    "category": "medium",
    "expiry": 1735689600
  }'
```

### Add a signature

```bash
curl -X POST http://localhost:3000/proposals/1/signatures \
  -H "Content-Type: application/json" \
  -d '{
    "signer": "GA...signer...",
    "signature": "BASE64_ED25519_SIGNATURE..."
  }'
```

### Submit a proposal

```bash
curl -X POST http://localhost:3000/proposals/1/submit
```

### Get account info

```bash
curl http://localhost:3000/accounts/GD...multisig-account...
```

## Security

The coordinator never trusts client claims. See the [Security Model](../docs/src/app/docs/concepts/security-model/page.mdx).

Verification rules for every signature:

1. Recompute the transaction hash from the stored XDR and network passphrase;
   verify the ed25519 signature against that hash.
2. Confirm the signing key is a current signer on the account, fetched from
   the network — not from the request.
3. Reject duplicate signatures from the same signer.
4. Compute readiness by summing signer weights against the account's threshold
   for the operation category (low/medium/high), using data fetched from the
   network.
5. Re-check readiness against fresh network state immediately before submit.
6. Reject proposals past their expiry or past the transaction's time bounds.

## Maintainer-only

This directory is maintainer-only. Contributors work in `docs/` only.
