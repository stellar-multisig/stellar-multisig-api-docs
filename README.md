# Stellar Multisig API + Docs

Coordinator API for Stellar multisig (M-of-N) proposal signing, plus the
Next.js documentation site.

```
stellar-multisig-api-docs/
├── api/                 # Rust + Axum coordinator API (maintainer-only)
└── docs/                # Next.js 14 documentation site (contributor-owned)
```

## API (`api/`)

Rust + Axum + tokio + sqlx (SQLite) coordinator service that:

- Stores pending multisig proposals
- Verifies and collects ed25519 signatures
- Submits transactions when the threshold is met

Endpoints:

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/proposals` | Create a proposal |
| `GET` | `/proposals/:id` | Proposal detail, signatures, readiness |
| `GET` | `/accounts/:address/proposals` | List proposals for an account |
| `POST` | `/proposals/:id/signatures` | Add a signer's signature |
| `POST` | `/proposals/:id/submit` | Submit if threshold met |
| `GET` | `/accounts/:address` | Signers, weights, thresholds from network |

Security core: the coordinator never trusts client claims. Every signature is
re-verified against a transaction hash recomputed from the stored XDR and
network passphrase. Signer metadata is always fetched live from Horizon.

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
