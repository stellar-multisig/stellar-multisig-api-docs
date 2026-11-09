# Contributing to Stellar Multisig API + Docs

## Scope

Contributors work **exclusively in `docs/`**. The `api/` folder is
maintainer-only.

PRs touching `api/` will be closed without review. If you want to propose a
change to the coordinator API, open an issue for discussion first.

## Documentation

All documentation lives in `docs/src/app/docs/` as MDX pages:

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

## Guidelines

- No placeholder content — all code examples must be real and tested
- Write for developers integrating with the coordinator API
- Include curl and TypeScript examples where relevant
- Keep pages focused and scannable

## Development

```bash
# Docs site
cd docs
npm install
npm run dev
npm run build
```

## Sister repositories

- [Contracts](https://github.com/stellar-multisig/stellar-multisig-contracts)
- [Web app](https://github.com/stellar-multisig/stellar-multisig-web)

## License

MIT
