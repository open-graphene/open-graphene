# R-Squared

R-Squared support uses the protocol extracted from
[`R-Squared-core` at `137a5b7c1e2da23b29ea101631229e165a15a851`](https://github.com/R-Squared-Project/R-Squared-core/tree/137a5b7c1e2da23b29ea101631229e165a15a851).
The existing `mirror.toml` manages the source checkout at
`blockchains/rsquared/R-Squared-core`; CI fetches this exact revision.

Chain identity follows the upstream `R-Squaredjs-ws/src/ChainConfig.js`:

- Chain ID: `a89f8a1cd2a699e5c521b87cc6210198ed0aad9e2a483322c6db2c391b278f64`.
- Public key prefix and core asset symbol: `RQRX`.
- Compact signatures use the Graphene legacy canonical signature profile.

## Packages and coverage

- Rust: `graphene-chain-rsquared-bindings` provides protocol types, JSON/RPC bindings and FC encoding. The Rust `graphene` facade remains Swaplock-only.
- TypeScript: `@open-graphene/chain-rsquared-bindings`, `@open-graphene/chain-rsquared-api` and `@open-graphene/chain-rsquared-react` are independent of the other chain packages.
- The TypeScript facade exposes `Graphene.rsquared(...)`, `Graphene.connect('rsquared', ...)`, `RSquaredClient` and the `rsquared` namespace.
- The shared spec contains 64 operations, including five virtual operations. All 59 user operations have generated builders and React preparation helpers.
- 31 RPC methods cover database, history, orders and network broadcast. React provides 30 read hooks and one explicit broadcast mutation. R-Squared has no `crypto_api` in the pinned source.
- Domain methods include personal data, content cards and permissions. Operation tags come from R-Squared's own `operations.hpp`; BitShares tags must not be reused.

The API includes account/asset reads, subscriptions, transaction preparation,
fee limits, canonical signing, explicit broadcasting and inclusion polling.
Generated encoders retain the existing fail-closed extension policy; unknown
extension payloads are rejected rather than guessed.

## Usage

Build the TypeScript workspace before using these unpublished packages:

```sh
cd open-graphene-packages/typescript
pnpm install --frozen-lockfile
pnpm build
```

```ts
import { RSquaredClient } from '@open-graphene/chain-rsquared-api';
import { AccountId } from '@open-graphene/chain-rsquared-bindings';

const client = await RSquaredClient.connect([
  'wss://node01.rsquared.digital:8090',
  'wss://node02.rsquared.digital:8090',
]);
try {
  const head = await client.database.getDynamicGlobalProperties({});
  console.log(head.head_block_number);

  const prepared = await client.operations.personal_data_create({
    subject_account: AccountId('1.2.1'),
    operator_account: AccountId('1.2.2'),
    url: 'https://example.com/data',
    hash: 'content-hash',
    storage_data: '',
  }).maxFee(100000n).prepare();
  // Preparation does not sign or broadcast.
  console.log(prepared.bytes);
} finally {
  client.close();
}
```

Typed IDs can be created with `AccountId` from the bindings package when needed.
React applications mount `RSquaredProvider` inside `QueryClientProvider`; use
`useDatabaseGetContentCards`, `useDatabaseGetPermissions`,
`usePreparePersonalDataCreate` or `usePrepareContentCardCreate` for domain flows.
The application owns and closes its client.

## Regeneration and validation

```sh
open-graphene-packages/rust/graphene-chain-rsquared/graphene-chain-rsquared-spec/bin/gen.sh
open-graphene-packages/rust/graphene-chain-rsquared/graphene-chain-rsquared-bindings/bin/gen.sh
pnpm --dir open-graphene-packages/typescript generate
```

Tests cover R-Squared wire tags and byte order, fee parameter layouts, domain
RPC exports, transaction signing with the chain digest, wrong-chain rejection,
fee/balance/head checks and ambiguous broadcast responses. Packed-package tests
check that a standalone R-Squared consumer imports no other chain SDK.

Live validation on 2026-10-16 confirmed both endpoints:

- `wss://node01.rsquared.digital:8090`
- `wss://node02.rsquared.digital:8090`

On each node, seven of eight checks passed: chain identity was verified during
connection, and the checks covered head/block decoding, core asset and account
reads, the full 64-entry fee schedule, personal-data and permission queries,
personal-data operation preparation with live fee pricing, and a subscription
receiving a newer block. The personal-data and permission queries returned empty
lists for `committee-account`; populated domain-object decoding was not verified.

`database.get_content_cards` was rejected on both nodes because the server's
`content_cards` plugin is disabled. This is an endpoint capability limitation;
the test retains the failure and exits nonzero rather than reporting full coverage.
No transaction was signed or broadcast, so live submission and inclusion remain
unverified.

Repeat this read-only check from the TypeScript workspace:

```sh
pnpm test:rsquared:live
# Optional report path followed by custom endpoints:
pnpm test:rsquared:live /tmp/rsquared-sdk-live.json wss://node01.rsquared.digital:8090
```

The default JSON report is `/tmp/rsquared-sdk-live.json`. The script closes every
subscription and client after the checks.
