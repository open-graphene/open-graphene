# Open Graphene Docs

Fumadocs with Next.js static export, hosted at https://open-graphene.github.io/open-graphene/.

## Development

Use Node.js 22.14+ and pnpm 10.15.0 from this directory:

```sh
pnpm install --frozen-lockfile
pnpm dev
```

Open http://localhost:3013/open-graphene/.
This directory has its own pnpm workspace and lockfile so CI can build independently.

## Content and checks

Write MDX in `content/docs/`; configure navigation in `meta.json`.
The UI uses the upstream Fumadocs theme; custom styles use Vanilla Extract.

```sh
pnpm build
pnpm types:check
pnpm format:check
```

The build exports HTML, assets, a browser search index, and Markdown/LLM routes into `out/`.
No Node.js server is required in production. Serve `out/` mounted at `/open-graphene/` for static preview.

## Publishing

`.github/workflows/docs-pages.yml` builds and deploys docs changes on `main` to GitHub Pages.
Pull requests run checks without deployment. Manual deployment is also supported.
GitHub Pages must use GitHub Actions as its build source.
