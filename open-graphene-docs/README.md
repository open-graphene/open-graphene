# Open Graphene Docs

Fumadocs with Next.js static export, hosted at https://open-graphene.github.io/open-graphene/.

## Development

Use Node.js 22.14+ and pnpm 10.15.0 from this directory:

```sh
pnpm --dir ../open-graphene-packages/typescript install --frozen-lockfile
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

## Generated API reference

`pnpm reference:generate` uses TypeDoc and typedoc-plugin-markdown to document
all public exports from the three chain API, React and bindings packages, plus
shared packages and FC subpath exports. Source mappings resolve workspace
imports directly to TypeScript source, so extraction does not depend on stale
`dist/` output. The SDK workspace dependencies must be installed first.

`content/docs/reference/` and `.reference/` are generated and ignored by Git.
Both `pnpm dev` and `pnpm build` regenerate the reference. Edit SDK comments
and exports to change reference content; edit `content/docs/guides/` for guides.
TypeScript 6 is pinned in the docs toolchain because TypeDoc 0.28 supports its
compiler API; the SDK retains its own TypeScript version.

`pnpm reference:check` independently compares the catalogs to TypeScript's
public exports and checks local reference links and representative nested RPC
methods. It also typechecks the connection and React examples against SDK source.
`pnpm export:check` verifies every HTML/Markdown page pair and the Pages size limit.
Symbol pages are linked from package catalogs and indexed by search,
while the sidebar stays focused on guides and package catalogs.

The Pages workflow also runs when TypeScript SDK files change, keeping the
published reference synchronized with the source.

Search indexes symbol names and member headings, with chain/package breadcrumbs;
reference parameter tables are excluded from the search index to keep the download bounded.
