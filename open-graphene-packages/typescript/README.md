# Native Open Graphene TypeScript

Native ESM SDK for Node.js >=22.12 and browsers, generated from the same pinned
C++ protocol specifications as Rust. No bitsharesjs wrapper, Rust/WASM runtime,
or duplicate protocol specification.

Implemented for Swaplock and BitShares:

- Generated JSON/RPC bindings: 95/78 operation variants, 47/32 RPC methods.
- FC encoding and typed operation factories for all 88/71 nonvirtual operations.
- Compound transactions, recursive proposal fees, fee caps, asset amounts,
  immutable preparation, multiple signatures, broadcast callbacks and inclusion.
- Endpoint selection, chain-pinned reconnect, bounded subscriptions, account,
  balance, order and history watches, market notices and ChainStore.
- Native encrypted memo, wallet keys, brain keys, account-role derivation,
  signatures, addresses, hashes and authority analysis.
- Swaplock room-access digests, mutation preconditions and member/key helpers.
- `@open-graphene/core` utilities and `@open-graphene/graphene` chain facade.

See the [Rust parity matrix and live evidence](../../docs/TYPESCRIPT-RUST-PARITY-2026-09-29.md).
318 operation vectors match independent Rust and native C++ serializers. Three
compound/guarded Swaplock transactions were included and made irreversible.
Packages have not been published. This is capability parity for the supported
Rust FC profile, not exhaustive input/branch coverage or a mainnet broadcast test.

## Development

From this directory, with pnpm 10.15 and the repository's Rust toolchain:

```sh
pnpm install --frozen-lockfile
cargo fetch --locked
pnpm generate
pnpm test
pnpm generate:check
pnpm exec playwright install chromium
pnpm test:browser
```

Generation uses `open-graphene-gen-bindings-ts` and the existing
`../rust/graphene-chain-*/graphene-chain-*-spec/dist/*.open-graphene.json`.
Do not edit `src/generated`. The generator validates IR before writing, detects
file/content drift with `--check`, and refuses to remove unmanaged extra files.
After Rust generators or extraction change, regenerate both languages.

## JSON boundary

```ts
import {
  TransactionCodec, parseJson, stringifyJson,
} from '@open-graphene/chain-swaplock-bindings';

const transaction = TransactionCodec.decode(parseJson(rawRpcJson));
// 64/128-bit integers are bigint; protocol bytes are Uint8Array.
const wireJson = stringifyJson(TransactionCodec.encode(transaction));
```

Always parse raw responses with `parseJson`: native `JSON.parse` may already
have rounded a wide integer before a decoder receives it. Unsafe numeric input
is rejected. Struct optionals become omitted properties; nullable vector/RPC
positions remain `null`. Typed IDs are chain-specific. Unknown fetched object
types remain `{ kind: 'unknown', value: rawObject }`; missing objects remain null.
Dynamic IR fields stay `unknown`, never `any`.

The codec currently depends on FC's checked public-key parser, which uses
browser-compatible hash/Base58 libraries; no Node `Buffer` shim is needed.
Tests use public fixtures only and do not require private keys, `.env`, or a node.
See [reference evidence](tests/fixtures/README.md) and the
[implementation plan](../../docs/TYPESCRIPT-PLAN.md).

## Live Swaplock testnet smoke test

```sh
pnpm test:testnet
pnpm test:testnet:browser
# Optionally select a specific endpoint:
pnpm test:testnet wss://node01.swaplock.chainpool.online:8090
```

These opt-in commands connect to the two nodes advertised by
`https://portal.swaplock.chainpool.online/`, verify the chain ID against the
generated spec, and perform 21 read/fee-estimation checks per node. Browser mode
runs the same generated codecs in Chromium on the actual portal origin.
This older smoke script uses a test adapter; the production session and transport
are exercised by the full live suite below. There is no signing, broadcasting, private-key loading,
or account creation. Live tests are excluded from the default offline CI suite.
Each run emits a JSON report and exits unsuccessfully if any check fails.

The first live run found a missing inherited `extended_asset_object.id` in the
shared extractor. The fix is covered by an extraction regression test and
regenerated Rust/TypeScript bindings. See the
[testnet report](../../docs/TYPESCRIPT-TESTNET-2026-09-29.md).

## Signed live transfer

```sh
pnpm build
# Signs locally and checks native serialization/authority, without submitting:
node examples/live-transfer.mjs --genesis /local/path/genesis.private.json
# Explicitly submit one small transfer between controlled genesis accounts:
node examples/live-transfer.mjs --genesis /local/path/genesis.private.json --broadcast --report /tmp/transfer.json
```

The example selects a funded genesis account whose active key still satisfies
its on-chain authority, transfers 0.01 core units and caps the fee at 3 core
units. It compares the signed FC bytes with the node's C++ serializer and asks
the node to verify authority before broadcasting. The public transaction is
recorded before sending. It then checks inclusion on both nodes and waits up to
60 seconds for irreversibility. An ambiguous send is never retried automatically.
Use `stringifyJson` for wire values returned by `SignedTransfer.toJSON()`.
Private keys are loaded only by this explicit example and are not reported.

## Full live RPC inventory

The current SDK exposes 47 generated RPC methods across five APIs. The full
opt-in suite uses the production transport in Node.js or Chromium and records
access denials separately from successful functional checks.

```sh
pnpm build
# Read-only: reuse the public fixture IDs from the coverage report:
node scripts/test-all-live.mjs /tmp/sdk-node.json /tmp/sdk-fixtures.json
node scripts/test-all-live.mjs /tmp/sdk-browser.json /tmp/sdk-fixtures.json --browser
```

The fixture JSON accepts `assetId`, `subjectRoomId`, and `grantCardId`.
Without suitable market/grant fixtures the suite reports missing coverage or
failed positive checks. It exits nonzero for failed or untested methods and
missing test data. Unavailable APIs remain explicitly marked in the report;
exit zero does **not** mean all 47 methods passed.

To prepare a fresh positive market/grant scenario, these separate commands
submit transactions and spend testnet fees (asset creation is 500 BTS on the
tested network). Use a new output path for each independent fixture set:

```sh
node scripts/seed-live-market.mjs /local/path/genesis.private.json /tmp/new-sdk-fixtures.json
node scripts/test-all-live.mjs /tmp/sdk-node.json /tmp/new-sdk-fixtures.json
node scripts/test-all-live.mjs /tmp/sdk-browser.json /tmp/new-sdk-fixtures.json --browser
node scripts/cleanup-live-market.mjs /local/path/genesis.private.json /tmp/new-sdk-fixtures.json
```

Setup uses the node's C++ FC serializer and a local signature; it is not
additional native TypeScript FC coverage. Its journal prevents resubmitting
an attempt with an unresolved outcome. Cleanup cancels the resting test order;
the asset, rooms, card and grant remain labelled test fixtures. Market
history and resting orders may expire, so results depend on fixture age.

The broadcast method is exercised separately by `examples/live-transfer.mjs`.
See the [full live coverage report](../../docs/TYPESCRIPT-SDK-LIVE-COVERAGE-2026-09-29.md):
39 reads passed on both nodes in both runtimes, one native TypeScript transfer
was confirmed and irreversible. After crypto_api was enabled, all seven crypto
methods also passed on both nodes in both runtimes, including negative cases,
cross-node proofs and values above 2^53. See the
[post-deployment crypto report](../../docs/TYPESCRIPT-SWAPLOCK-CRYPTO-LIVE-2026-09-29.md).
Together the two runs cover all 47 current RPC methods; broadcast was not
repeated in the crypto run.


## BitShares API

```ts
import { BitSharesClient, BitSharesWifSigner } from '@open-graphene/chain-bitshares-api';
import { DatabaseGetDynamicGlobalProperties } from '@open-graphene/chain-bitshares-bindings';

const client = await BitSharesClient.connect(endpoint);
try {
  const head = await client.rpc.invoke(DatabaseGetDynamicGlobalProperties, {});
  console.log(head.head_block_number);
} finally {
  client.close();
}
```

Use `client.prepareTransfer({ from, to, amount, maxFee })` (amounts in BTS atoms),
`prepared.sign(new BitSharesWifSigner(wif))`, `client.broadcast(signed)`, and
`client.waitForInclusion(signed)` for transfers. Dispose the signer when finished.
Broadcast submits to the connected mainnet and consumes real network fees.
The client supports mainnet's generated chain ID and BTS public-key prefix;
arbitrary testnets require a corresponding generated chain profile.

External signers must return a valid low-S signature with legacy Graphene
canonical r/s lengths. The shared `WifSigner` retains its Swaplock low-S default;
`BitSharesWifSigner` explicitly selects the canonical profile. Retry entropy is
deterministic, but signatures need not be byte-identical to bitsharesjs.

```sh
# Read-only mainnet checks, including native C++/TypeScript FC comparison:
pnpm test:bitshares:live /tmp/bitshares-node.json
pnpm test:bitshares:live /tmp/bitshares-browser.json --browser
```

The expanded read-only suite covers 31 non-broadcast RPC methods plus a native
FC comparison. On both tested public endpoints, 24 RPC methods and FC passed;
seven crypto methods were denied by node permissions. Broadcast and inclusion
are tested with a local RPC harness; no live BitShares transfer was submitted.
See the [current parity report](../../docs/TYPESCRIPT-RUST-PARITY-2026-09-29.md).

## Operations, sessions and wallets

```ts
import { Graphene, PrivateKey } from '@open-graphene/graphene';
import { AccountId, AssetId } from '@open-graphene/chain-swaplock-bindings';

const client = await Graphene.swaplock(endpoints);
const signer = PrivateKey.fromWif(wif);
try {
  const prepared = await client.operations.transfer({
    from: AccountId('1.2.100'), to: AccountId('1.2.101'),
    amount: { amount: 1n, asset_id: AssetId('1.3.0') },
  }).maxFee(300000n).prepare();
  const signed = await prepared.sign([signer]);
  // Submitting consumes network fees:
  await client.broadcast(signed);
  await client.waitForInclusion(signed);
} finally {
  signer.dispose();
  client.close();
}
```

Factories use generated snake_case protocol fields; fee and extensions have
checked defaults. Compose with `.addOperation(...)`, or pass an operation array
to `client.prepareOperations`. Use `BitSharesWifSigner` or the
`graphene-legacy` signing profile for BitShares.

Streams are async iterators with an explicit `.close()`. Reconnect closes old
streams; create new watches and snapshots after reconnect. History watches
refresh the requested page rather than promising an immutable historical cursor.
Populated extension support follows Rust's allowlist; unsupported payloads,
nonempty legacy address maps and virtual signing fail closed.
JavaScript cannot guarantee erasure of immutable strings or caller-held copies;
`dispose()` clears the SDK's owned key byte arrays.

Independent FC verification (read-only, built Rust examples required):

```sh
cargo build --offline --locked -p graphene-chain-swaplock-bindings --example fc_oracle
cargo build --offline --locked -p graphene-chain-bitshares-bindings --example fc_oracle_bitshares
node scripts/fc-parity-vectors.mjs --rich --live
node scripts/fc-parity-vectors.mjs --bitshares --rich --live
# Explicit opt-in: broadcasts testnet transactions and cleans up its room/order:
node scripts/test-parity-live.mjs /local/path/genesis.private.json /tmp/parity-live.json
```
