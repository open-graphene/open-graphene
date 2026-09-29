# Native Open Graphene TypeScript

Initial implementation of the native ESM SDK for Node.js >=22.12 and browsers.
This workspace consumes the same checked-in protocol specifications as Rust.
It does not wrap bitsharesjs, execute Rust/WASM, or maintain duplicate specs.

Implemented:

- `@open-graphene/primitives`: checked, branded chain/object IDs, timestamps,
  vote IDs and byte conversions.
- `@open-graphene/fc`: primitive FC writer, packed versus typed IDs, public-key
  checksum decoding, transaction preimages and hashes; the `/signing` export adds
  WIF, secp256k1 compact low-S signatures, verification and recovery.
- `@open-graphene/codec`: lossless JSON, checked struct/variant codecs, recursive
  fees, object routing and positional RPC descriptors.
- `@open-graphene/chain-swaplock-bindings`: generated definitions and codecs for
  95 operations and descriptors for 47 RPC methods from the current spec.
- `@open-graphene/chain-bitshares-bindings`: 78 operations and 11 RPC methods.
- Generated FC for the transfer/transaction dependency closure in both chains;
  other operations and nonempty future extensions fail closed.
- `@open-graphene/transport`: one-connection WebSocket RPC, API discovery,
  request multiplexing and timeouts, without automatic retries.
- `@open-graphene/chain-swaplock-api`: core-asset transfer preparation with fee
  cap, immutable transaction snapshots, single-active-key signing, broadcast
  and block inclusion lookup.

- `@open-graphene/chain-bitshares-api`: BitShares mainnet client with the same
  core-transfer lifecycle and a `BitSharesWifSigner` producing legacy canonical
  compact signatures. Chain identity is checked before use.

These are generated definitions, not a claim that every operation has been
independently tested against a live node. Each generated `support.json` records
dynamic/unresolved fields and the narrow FC/signing scope. A native TypeScript
transfer has been included and made irreversible on the Swaplock testnet;
see [live transaction evidence](../../docs/TYPESCRIPT-LIVE-TRANSFER-2026-09-29.md).
Remaining work includes other FC operations, memo encryption, multisig, reconnect/subscriptions, richer transaction orchestration and
ChainStore. Packages have not been published.

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
The socket adapter is test-only; this does not implement the future production
transport/session layer. There is no signing, broadcasting, private-key loading,
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

The read-only suite covers the ten generated non-broadcast methods. Broadcast
and inclusion are tested with a local RPC harness; no live BitShares transfer
has been submitted. See [validation evidence](../../docs/TYPESCRIPT-BITSHARES-API-2026-09-29.md).
Memo encryption, other operation serializers, multisig and subscriptions remain
outside this initial API stage.
