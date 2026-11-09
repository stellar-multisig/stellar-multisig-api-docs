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
| `POST` | `/proposals` | Create a proposal |
| `GET` | `/proposals/:id` | Proposal detail |
| `GET` | `/accounts/:address/proposals` | List proposals |
| `POST` | `/proposals/:id/signatures` | Add a signature |
| `POST` | `/proposals/:id/submit` | Submit if threshold met |
| `GET` | `/accounts/:address` | Account signers/thresholds |

## Security

The coordinator never trusts client claims. See the [Security Model](../docs/src/app/docs/concepts/security-model/page.mdx).

## Maintainer-only

This directory is maintainer-only. Contributors work in `docs/` only.
