# Preparing a TypeScript release

The 13 public packages currently use version `0.1.0`. This is a release candidate;
the checks below prepare and validate artifacts without publishing to npm.

From `open-graphene-packages/typescript`:

```sh
pnpm install --frozen-lockfile
pnpm generate:check
pnpm test
pnpm test:react:matrix
pnpm test:browser
pnpm test:packages
```

`test:packages` builds the workspace, packs all public packages and installs the
tarballs in two independent temporary applications. It checks ESM imports, TypeScript
declarations with `skipLibCheck: false`, package exports, both license files and
chain isolation. Its offline installation needs dependencies already cached by
the workspace install. Set `GRAPHENE_PNPM_STORE` if a separate writable pnpm store
is needed. Temporary artifacts are retained for inspection.

The printed `release-plan.json` contains the dependency order, artifact versions,
SHA-256 hashes and consumer results. The
[recorded artifact checks](TYPESCRIPT-PACKAGES-2026-09-30.json) describe the tested
candidate tarballs; changes require a new build and report. For the current
dependency graph the order is:

1. `@open-graphene/primitives`
2. `@open-graphene/fc`
3. `@open-graphene/codec`
4. `@open-graphene/transport`
5. `@open-graphene/chain-bitshares-bindings`
6. `@open-graphene/core`
7. `@open-graphene/chain-bitshares-api`
8. `@open-graphene/react-core`
9. `@open-graphene/chain-bitshares-react`
10. `@open-graphene/chain-swaplock-bindings`
11. `@open-graphene/chain-swaplock-api`
12. `@open-graphene/chain-swaplock-react`
13. `@open-graphene/graphene`

Before the first publication, verify ownership of the npm scope and whether any
candidate versions already exist. Registry availability has not been checked by
the offline artifact tests. Keep the initial packages on the same version; update
all public manifests and the lockfile together, describe the changes in
[the changelog](../open-graphene-packages/typescript/CHANGELOG.md), then rebuild and
regenerate the release plan. npm versions are immutable.

Once publication is authorized, publish the exact tested tarballs in the generated
order, using an authenticated npm account with access to the scope:

```sh
npm publish /absolute/path/to/open-graphene-primitives-0.1.0.tgz --access public
```

Repeat for the remaining artifacts and wait for each dependency to become available
before publishing its dependents. After publication, install the registry versions
in fresh applications without tarball overrides and repeat the import/type checks.
Tag the verified release commit. CI validates artifacts; it does not publish them.

## Opt-in Swaplock room transaction check

```sh
pnpm test:react:room:live /absolute/path/to/genesis.private.json /tmp/react-room-live.json
```

This command sends a create and delete transaction on the pinned Swaplock testnet
using the genesis `swaplock` active key. It runs actual React hooks in a simulated
DOM, verifies signed FC bytes and authority with the node, waits for inclusion and
observes creation/deletion through the live room hook. It stores public transaction
data before broadcast; private keys are never written to the report. Keep the report
to avoid resubmission on a rerun. An unresolved transaction stops the run and requires
chain-state reconciliation; never delete its report just to retry. A known created
room is deleted on failure when the delete outcome is not already unresolved.

This transaction test is separate from CI and from Chromium browser checks. The
React 18/19 matrix uses simulated RPC to exercise the same room pipeline without
broadcasting. On 2026-09-30 the matrix and packed-package checks passed, but the
new live room run failed to connect to the testnet in the restricted environment.
No transaction from that run was signed or broadcast. Its live acceptance check
remains outstanding.
