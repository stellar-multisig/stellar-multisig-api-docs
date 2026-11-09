# Stellar Multisig API + Docs

Coordinator API for Stellar multisig (M-of-N) proposal signing, plus the
Next.js documentation site.

```
stellar-multisig-api-docs/
├── api/                 # Rust + Axum coordinator API (maintainer-only)
└── docs/                # Next.js 14 documentation site (contributor-owned)
```

## Stellar integration

The coordinator is a **verification layer between clients and the Stellar
network**. It never signs or holds secret keys — it orchestrates signature
collection and submits transaction XDR when the M-of-N threshold is met.

### Network support

| Network | Passphrase | Horizon |
|---------|-----------|---------|
| testnet | `Test SDF Network ; September 2015` | `https://horizon-testnet.stellar.org` |
| pubnet | `Public Global Stellar Network ; September 2015` | `https://horizon.stellar.org` |

The network is configured via `STELLAR_NETWORK` and `HORIZON_URL` environment
variables.

### Transaction lifecycle

```
Client                              Coordinator                    Stellar Network
  │                                  │                             Horizon / Soroban
  │                                  │                             (SAC custom account)
  ├─ POST /proposals (XDR) ─────────>│                             │
  │                                  │ store XDR + metadata        │
  │                                  │                             │
  ├─ GET /accounts/:address ────────>│                             │
  │                                  │ fetch signers + thresholds  │
  │                                  │<──── Horizon response ─────┤
  │                                  │                             │
  ├─ POST /proposals/:id/signatures ─>│                             │
  │                                  │ recompute tx hash (XDR)     │
  │                                  │ verify ed25519 sig vs hash  │
  │                                  │ confirm signer via Horizon  │
  │                                  │ reject duplicates           │
  │                                  │ store signature             │
  │                                  │                             │
  ├─ POST /proposals/:id/submit ────>│                             │
  │                                  │ re-check readiness (fresh)  │
  │                                  │<──── Horizon response ─────┤
  │                                  │ submit XDR to network       │
  │                                  │──── submit tx ─────────────>│
  │                                  │                             │
```

### Signature verification (security core)

For every signature added, the coordinator:

1. **Recomputes the transaction hash** from the stored base64 XDR and the
   network passphrase (SHA-256 of passphrase mixed with the signed envelope).
2. **Verifies the ed25519 signature** against that 32-byte hash.
3. **Confirms the signing key is a current signer** on the account by fetching
   signer data from Horizon — never trusting the request body.
4. **Rejects duplicate signatures** from the same signer.
5. **Sums signer weights** and compares against the account's threshold for the
   operation category (low / medium / high).
6. **Re-checks readiness against fresh Horizon state immediately before submit**,
   since signers or thresholds may have changed on-chain.
7. **Rejects expired proposals** and proposals past the transaction's time bounds.

The API never receives or stores secret keys and rejects any request containing
an `S...` strkey.

### Soroban Smart Accounts

The coordinator works with Soroban custom accounts (`__check_auth`) deployed
via the [`stellar-multisig-contracts`](https://github.com/stellar-multisig/stellar-multisig-contracts)
repo:

- **multisig-account** — on-chain contract implementing `__check_auth` for
  M-of-N ed25519 verification
- **account-factory** — deterministic deployment of multisig accounts

A Soroban host invokes `__check_auth` on the multisig contract when
`require_auth()` is called on the account address. Signatures must be submitted
in ascending public key order (enforced on-chain). The coordinator verifies each
signature independently; on-chain `__check_auth` enforces ordering and the
final threshold count.

## API (`api/`)

Rust + Axum + tokio + sqlx (SQLite) coordinator service.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/proposals` | Create a proposal (network, account, XDR, category, expiry) |
| `GET` | `/proposals/:id` | Proposal detail, signatures, readiness |
| `GET` | `/accounts/:address/proposals` | List proposals for an account |
| `POST` | `/proposals/:id/signatures` | Add a signer's ed25519 signature |
| `POST` | `/proposals/:id/submit` | Submit XDR if threshold met |
| `GET` | `/accounts/:address` | Signers, weights, thresholds from Horizon |

### Build & run

```bash
cd api
cargo build --release
cargo test

DATABASE_URL=sqlite:./multisig.db \
HORIZON_URL=https://horizon-testnet.stellar.org \
STELLAR_NETWORK=testnet \
BIND_ADDR=0.0.0.0:3000 \
cargo run
```

## Docs (`docs/`)

Next.js 14 + TypeScript + Tailwind CSS + MDX documentation site.

```
docs/src/app/docs/
  getting-started/page.mdx
  concepts/
    classic-multisig/page.mdx
    soroban-smart-accounts/page.mdx
    security-model/page.mdx
  contracts/
    multisig-account/page.mdx
    account-factory/page.mdx
  api/
    overview/page.mdx
    proposals/page.mdx
    signatures/page.mdx
    accounts/page.mdx
  web-app-guide/page.mdx
  integration/page.mdx
```

Deployed to Vercel.

## Contributing

Contributors work exclusively in `docs/`. The `api/` folder is maintainer-only;
PRs touching `api/` will be closed. See `CONTRIBUTING.md`.

## Sister repositories

| Repo | URL |
|------|-----|
| Contracts | https://github.com/stellar-multisig/stellar-multisig-contracts |
| Web app | https://github.com/stellar-multisig/stellar-multisig-web |

## License

MIT
