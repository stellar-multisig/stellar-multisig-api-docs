# Stellar Multisig Docs

Next.js 14 documentation site for the Stellar Multisig Coordinator API.

## Structure

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

## Contributing

Contributors work exclusively in `docs/`. The `api/` folder is maintainer-only.
See the root `CONTRIBUTING.md` for details.

## Development

```bash
cd docs
npm install
npm run dev
```

## Build

```bash
cd docs
npm run build
```

## Deploy

Deployed to Vercel. Connect this repository to Vercel and set the root
directory to `docs/`.

The build command is `npm run build` and the output directory is `.next/`.

## License

MIT
