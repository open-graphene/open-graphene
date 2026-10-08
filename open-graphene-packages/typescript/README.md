# Open Graphene SDK for TypeScript

Build applications for **Swaplock and BitShares** in Node.js and the browser.
Read accounts and balances, send transactions, encrypt memos, work with Swaplock
rooms, and subscribe to changes through a typed API.

The SDK is native TypeScript. Its protocol bindings come from the same C++ source
specifications as the Rust SDK. It uses neither a bitsharesjs wrapper nor WASM.

## Start here

Requires Node.js **22.12+** or a modern browser. Browser memo encryption requires
Web Crypto in a secure context, such as HTTPS or localhost. Packages are ESM.

**The packages are not published to npm yet.** Build them from this workspace:

```sh
# Run from open-graphene-packages/typescript
pnpm install --frozen-lockfile
pnpm build
```

For an application in the same pnpm workspace, add the packages you import with
`workspace:*`. The main entry point is `@open-graphene/graphene`; chain-specific
packages are available when you only need one chain.

| Package                                   | Use it for                                                               |
| ----------------------------------------- | ------------------------------------------------------------------------ |
| `@open-graphene/chain-swaplock-react`     | Swaplock React hooks, live updates and operation preparation             |
| `@open-graphene/chain-bitshares-react`    | BitShares React hooks, live updates and operation preparation            |
| `@open-graphene/react-core`               | Shared React adapter and cache utilities                                 |
| `@open-graphene/graphene`                 | Connect to either chain; access wallet, memo and amount helpers          |
| `@open-graphene/chain-swaplock-api`       | Swaplock transactions, queries and room-access helpers                   |
| `@open-graphene/chain-bitshares-api`      | BitShares transactions and queries                                       |
| `@open-graphene/chain-swaplock-bindings`  | Swaplock protocol types, operation factories and codecs                  |
| `@open-graphene/chain-bitshares-bindings` | BitShares protocol types, operation factories and codecs                 |
| `@open-graphene/core`                     | Amount formatting, account-name validation and authority analysis        |
| `@open-graphene/fc`                       | Binary serialization; `/signing`, `/memo`, `/wallet` and `/hash` exports |
| `@open-graphene/transport`                | WebSocket sessions, subscriptions and ChainStore                         |
| `@open-graphene/codec`                    | Lossless JSON and checked runtime codecs                                 |
| `@open-graphene/primitives`               | IDs, timestamps and byte conversions                                     |

## Formatting

Run `pnpm format` from this workspace to format all packages, scripts, tests and
documentation, or run it inside an individual package to format only that package.
Use `pnpm format:check` to verify formatting without changing files. All packages
inherit the workspace `.prettierrc.json` configuration. Generated sources,
build output and the lockfile are excluded; use `pnpm generate:check` to verify
generated sources.

## React applications

Use [`@open-graphene/chain-swaplock-react`](graphene-chain-swaplock/graphene-chain-swaplock-react/README.md)
or [`@open-graphene/chain-bitshares-react`](graphene-chain-bitshares/graphene-chain-bitshares-react/README.md)
for TanStack Query hooks, live subscriptions and dedicated preparation hooks for
every user operation. The adapters share `@open-graphene/react-core` and keep
React dependencies out of the core SDK. See the
[React integration guide](../../docs/TYPESCRIPT-REACT.md) for examples.

See the [release guide](../../docs/TYPESCRIPT-RELEASE.md) for package artifact
checks, React 18/19 validation and the opt-in live room transaction test. The
current `0.1.0` manifests are release candidates; npm publication is pending.

## Connect and read an account

This example connects to the Swaplock testnet, reads an account and its balances,
and closes the connection. Read-only calls do not need a private key.

```ts
import { Graphene, formatRawAmount } from '@open-graphene/graphene';
import { CHAIN } from '@open-graphene/chain-swaplock-bindings';

const client = await Graphene.swaplock(
  [
    'wss://node01.swaplock.chainpool.online:8090',
    'wss://node02.swaplock.chainpool.online:8090',
  ],
  {
    expectedChainId: CHAIN.chainId,
  },
);

try {
  const account = await client.database.account('swaplock');
  const balances = await client.database.accountBalances(account.id);
  const core = await client.database.asset('1.3.0');
  const balance = balances.find((item) => item.asset_id === core.id);

  console.log(account.id, account.name);
  console.log(
    formatRawAmount(balance?.amount ?? 0n, core.precision),
    core.symbol,
  );
} finally {
  client.close();
}
```

`SwaplockClient.connect`, `Graphene.swaplock`, `Graphene.connect('swaplock', ...)`
and `SwaplockClient.probeLatencies` require `expectedChainId`: the trusted lowercase
64-character hex ID of your deployment. `CHAIN.chainId` selects the bundled testnet;
pass your deployment ID for another Swaplock network. The client verifies that ID
before exposing the connection and on every reconnect. Do not derive the expected
ID from the untrusted node you are about to connect to.

`prepareTransfer` and `prepareOperations` capture the verified ID in the immutable
`prepared.chainId`. Both external signers and `signWithWifs` sign the digest for that
network. Direct constructors also require it: `new PreparedTransaction({ transaction,
startBlock, chainId })` and `new PreparedTransfer({ transaction, authority, startBlock,
chainId })`. Network selection does not change the Swaplock protocol or key prefix.
The React provider remains restricted to the bundled network.

`prepareOperations(operations, options)` accepts `maxHeadAgeSeconds` and
`maxHeadTimeAheadSeconds` (both default to 120). The limits apply to the exact
reference block used in the prepared transaction. Use stricter application limits
when needed, for example `{ maxFee: 1000n, expirationSeconds: 120,
maxHeadAgeSeconds: 30, maxHeadTimeAheadSeconds: 5 }`. Options are captured before
RPC work. Preparation only estimates fees and constructs the unsigned transaction;
it never signs or broadcasts.

`TransactionPreparationError.code` distinguishes `invalid-fee` (wrong count,
asset, negative amount or unexpected recursive structure), `fee-limit` and
`invalid-head` (block zero or a timestamp outside the configured limits). Decoder,
transport and cancellation errors propagate unchanged. Callers still own authority,
balance and application-intent checks.

`PreparedTransaction.sign(signer, { signal })` and
`signWithWifs(keys, { signal })` can stop waiting for an external signer, even if
it ignores cancellation. The caller still forwards the signal to its signing
provider when that provider supports cancellation. An already aborted signal
invokes no signer. Cancellation rejects with `signal.reason`, releases the abort
listener, ignores late results and never requests another signature. It cannot
undo work already started in a signing device.

`TransactionSigningError.code` distinguishes `expired`, `invalid-signature`,
`no-signers` and the WIF-specific `key-mismatch`. Expiration is checked before
signing and after each signer returns. Provider exceptions propagate unchanged;
applications should sanitize them at their public boundary. Public keys and
returned signatures are copied before verification so signer-owned buffers cannot
change an accepted signature while another key is being requested.

An endpoint list provides connection fallback;
`{ strategy: 'lowest-latency' }` selects by measured connection latency.
Automatic reconnect uses the selected endpoint and checks the chain ID again.

Pass an `AbortSignal` to cancel connection setup and the entire session lifetime:

```ts
import { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import { CHAIN } from '@open-graphene/chain-swaplock-bindings';

const controller = new AbortController();
const client = await SwaplockClient.connect(
  'wss://node01.swaplock.chainpool.online:8090',
  {
    expectedChainId: CHAIN.chainId,
    signal: controller.signal,
  },
);

try {
  // The owner of this operation can call controller.abort() at any time.
  const account = await client.database.account('swaplock');
  console.log(account.id);
} finally {
  client.close();
}
```

The same option is available on `RpcClient` and `GrapheneSession`. An already
aborted signal opens no socket. Cancellation rejects pending connection setup,
RPC calls and subscription waits with `signal.reason`, closes the socket, and
prevents endpoint fallback or reconnect. It does not wait for a server close
acknowledgement. Explicit `close()` also cancels pending reconnect work. Both
paths release socket listeners, abort listeners and pending timers.

Cancellation after sending a transaction does not undo it or prove rejection.
Check inclusion before considering another submission; broadcasts are never
retried automatically. Use a new signal and connection for a new session.

### Classify a chain mismatch

`@open-graphene/transport` exports `ChainIdMismatchError` with
`expectedChainId` and `actualChainId`. Connection setup and reconnection throw it
when a node returns a valid chain ID different from the required one. The failed
connection is closed. Invalid chain-ID responses and transport failures remain
separate errors. Swaplock's chain API re-exports the error and helper.

`hasChainIdMismatch(error)` recognizes the typed error directly or inside nested
`AggregateError` instances, including cyclic aggregates. It does not inspect
message text or follow arbitrary `cause` properties. A `true` result means that
**at least one** failure is a mismatch; other endpoints may have been unavailable.
This preserves diagnostics when all connection attempts fail. Fallback and
latency selection still use a compatible endpoint if one succeeds. Cancellation
takes precedence and stops further fallback attempts.

### Restore a saved signed transaction

`restoreSignedTransaction` decodes a saved transaction, checks its expected ID,
and verifies exactly one compact signature against the supplied chain ID and
compressed public key. It is synchronous and does not access the network or
request a new signature.

```ts
import { restoreSignedTransaction } from '@open-graphene/chain-swaplock-api';

const signed = restoreSignedTransaction({
  serializedTransaction: saved.signedTransaction,
  chainId: saved.chainId,
  expectedTransactionId: saved.transactionId,
  expectedPublicKey: signerPublicKeyBytes,
  startBlock: saved.startBlock,
});
```

`TransactionRestorationError.code` distinguishes `invalid-input`,
`invalid-transaction`, `transaction-mismatch` and `invalid-signature`. Decode
errors do not expose the serialized transaction in their messages. The returned
`SignedTransfer` owns its serialized snapshot; changing a decoded copy does not
change the restored transaction.

Expired transactions remain restorable so recovery can inspect earlier
inclusion. Restoring is not permission to send again; submission checks expiry
separately. Callers must still validate their approved domain intent and trust
or validate the supplied expectations. `startBlock` is a caller-owned search
hint, not signed metadata. This API supports one expected signer and does not
resolve account authority or multisignature thresholds.

### Broadcast callbacks

`client.networkBroadcast.sendTransactionWithCallback(signed)` submits once and
returns a handle with `wait(timeoutMs)` and `close()`. `wait` returns a typed
`BroadcastConfirmation`: transaction ID, block number, transaction index and a
decoded `ProcessedTransaction`. The SDK validates the callback shape, positions,
transaction ID, exact signed transaction bytes and operation result count. Every
settled wait closes its subscription; close the handle if you do not call wait.

`TransactionBroadcastError.code` distinguishes `expired` (rejected locally before
sending), `invalid-confirmation`, `transaction-mismatch`, `timeout` and `closed`.
A malformed or missing callback does not prove that the transaction failed.
Lost registration acknowledgements, cancellation and connection loss can produce
`BroadcastOutcomeUnknown`; no broadcast is retried automatically. Recovery must
check the original transaction before considering another submission.

A callback is a node report, not proof of canonical inclusion or irreversible
execution. Applications still verify the canonical block and domain effects,
and own durable submission records and recovery policy.

### Check direct single-key authority

`canKeySatisfyAuthority(authority, publicKey)` from `@open-graphene/core` (also
re-exported by the chain APIs) checks whether a directly listed public key alone
reaches a positive `weight_threshold`. The key must occur exactly once in
`key_auths`; duplicate entries do not add weight. Invalid threshold or selected
key weight ranges return `false`.

This synchronous check does not resolve delegated account/address authorities,
combine several signers, validate public-key encoding or prove possession of a
private key. `false` means the selected direct key cannot independently satisfy
this check, not that the account cannot authorize a transaction by other means.
A sufficient direct key remains sufficient when other authority members exist.
Applications still verify the account, key possession, current memo key and any
domain-specific permissions. Exact expected authority composition is a separate
application rule.

### Check head freshness and identity

`@open-graphene/primitives` exports `isHeadFresh(head, limits)` and
`isSameHead(before, after)` for decoded Graphene heads. These helpers make no RPC
calls and do not retry reads. Callers retain their own error mapping and retry
policy.

```ts
import { isHeadFresh, isSameHead } from '@open-graphene/primitives';

const fresh = isHeadFresh(after, {
  maxHeadAgeSeconds: 30,
  maxHeadTimeAheadSeconds: 5,
});
const unchanged = isSameHead(before, after);
```

Freshness uses the local clock with subsecond precision and inclusive limits.
Both limits are explicit nonnegative integers; invalid limits throw `RangeError`.
Block numbers must be positive uint32 values; malformed protocol times throw
rather than count as fresh. Head identity compares both the number and ID bytes,
so a same-height reorganization is a change. Matching heads do not establish an
atomic snapshot or independently verified consensus.

### Inspect a known transaction position

Use `client.inspectTransactionInBlock(options)` for a single inspection of a
position reported by a callback or indexer:

```ts
const inclusion = await client.inspectTransactionInBlock({
  transactionId: signed.id,
  blockNumber: confirmation.blockNumber,
  transactionIndex: confirmation.transactionIndex,
  maxHeadAgeSeconds: 30,
  maxHeadTimeAheadSeconds: 5,
});
```

The SDK reads head, block and head again, checks both heads for freshness and
requires matching head numbers and IDs. Each freshness limit defaults to 120
seconds. `TransactionInspectionError` reports `invalid-head` or `head-changed`;
RPC and decoding failures propagate. Session cancellation also cancels these
reads. This method does not poll or submit transactions.

A missing block, missing position, position above the observed head or different
transaction ID returns `undefined`. This does not prove rejection and must not
implicitly authorize a replacement transaction. A successful
`TransactionBlockInclusion` contains the decoded block and transaction,
`observedAt`, and `irreversible` based on the node's last irreversible block.
`blockFingerprint` is SHA-256 of the serialized signed header, **not** the
protocol's block ID. Retaining it lets a later inspection detect a replacement
block even if the same transaction occupies the same position.

The result is one node's observation, not independent consensus verification.
Applications still validate domain intent and effects and decide how a changed
or missing inclusion affects their durable records. `waitForInclusion` remains
a separate scanning helper; it does not perform this stable-head inspection.

## Read history and market data

Most applications can use `client.database`, `client.history`, `client.orders`
and `client.crypto` directly. Generated RPC method names use camelCase; their
parameter objects retain the protocol's snake_case field names.

```ts
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';

async function inspectAccount(client: SwaplockClient, accountName: string) {
  const page = await client.history.accountHistory(accountName, 20, 0);
  console.log(page.items);

  if (page.nextOffset !== null) {
    const nextPage = await client.history.accountHistory(
      accountName,
      20,
      page.nextOffset,
    );
    console.log(nextPage.items);
  }

  const orders = await client.database.accountOrders(accountName);
  const head = await client.database.getDynamicGlobalProperties({});
  console.log(orders, head.head_block_number);
}
```

History takes `(accountNameOrId, limit, offset)`; `limit + offset` must be at most 98. New transactions can shift offset-based pages. To read a market ticker, call
`client.database.getTicker({ base: 'BTS', quote: 'USD' })` on a chain where both
assets exist.

## Send a transfer on Swaplock

A transfer has four steps: **prepare → sign → broadcast → wait for inclusion**.
Preparation resolves accounts/assets, obtains fees and checks the liquid balance.
Signing happens locally. Broadcasting submits the transaction and spends network
fees; inclusion means it appeared in a block, not that it is irreversible yet.

The function below accepts an existing connection and the sender's active WIF key.
Use accounts you control and obtain keys from your application's key storage.

```ts
import { PrivateKey } from '@open-graphene/graphene';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';

async function sendTransfer(
  client: SwaplockClient,
  activeWif: string,
  from: string,
  to: string,
) {
  const signer = PrivateKey.fromWif(activeWif);
  try {
    const prepared = await client.prepareTransfer({
      from,
      to,
      amount: '0.01', // Decimal asset units, expressed as a string.
      asset: 'BTS',
      maxFee: 300000n, // Raw fee units: 3 BTS when precision is 5.
    });

    const signed = await prepared.sign(signer);
    console.log('Transaction ID:', signed.id);
    await client.broadcast(signed);
    return await client.waitForInclusion(signed);
  } finally {
    signer.dispose();
  }
}
```

Use `bigint` for raw amounts (`1000n`), or a decimal string (`'0.01'`) for
`prepareTransfer`. Do not use floating-point numbers for amounts. `maxFee` is
always in raw units of the fee asset; `feeAsset` defaults to the core asset.

If broadcast reports `BroadcastOutcomeUnknown`, check inclusion for that signed
transaction before attempting another send. The SDK does not retry broadcasts.

## Use BitShares

BitShares has the same transfer lifecycle. Use its signer to produce the legacy
canonical signature format required by that chain.

```ts
import { Graphene } from '@open-graphene/graphene';
import { BitSharesWifSigner } from '@open-graphene/chain-bitshares-api';

async function prepareBitSharesTransfer(wif: string, from: string, to: string) {
  const client = await Graphene.bitshares('wss://api.dex.trading/');
  const signer = new BitSharesWifSigner(wif);
  try {
    const prepared = await client.prepareTransfer({
      from,
      to,
      amount: '0.01',
      asset: 'BTS',
      maxFee: 300000n,
    });
    const signed = await prepared.sign(signer);
    console.log('Signed locally:', signed.id);
    // To submit on BitShares mainnet, while this client is still open:
    // await client.broadcast(signed);
    // await client.waitForInclusion(signed);
    return signed;
  } finally {
    signer.dispose();
    client.close();
  }
}
```

This function prepares and signs without broadcasting. For the wallet API,
`PrivateKey.fromWif(wif, 'graphene-legacy')` selects the same signing profile.
The default `PrivateKey` profile is for Swaplock.

## Combine operations in one transaction

Use `client.operations` for a single operation, or `prepareOperations` for a batch.
Factories provide default zero fees and empty extensions; preparation replaces
fees with the values returned by the node. Operation names and fields use the
protocol's snake_case spelling.

```ts
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import {
  AccountId,
  AssetId,
  bindOperationBuilders,
} from '@open-graphene/chain-swaplock-bindings';

async function prepareTwoTransfers(client: SwaplockClient) {
  const op = bindOperationBuilders((operation) => operation);
  const from = AccountId('1.2.100');
  const to = AccountId('1.2.101');
  const asset_id = AssetId('1.3.0');

  return client.prepareOperations(
    [
      op.transfer({ from, to, amount: { amount: 1n, asset_id } }),
      op.transfer({ from, to, amount: { amount: 2n, asset_id } }),
    ],
    { maxFee: 300000n, expirationSeconds: 120 },
  );
}

async function prepareOneTransfer(client: SwaplockClient) {
  return client.operations
    .transfer({
      from: AccountId('1.2.100'),
      to: AccountId('1.2.101'),
      amount: { amount: 1n, asset_id: AssetId('1.3.0') },
    })
    .maxFee(300000n)
    .expiration(120)
    .prepare();
}
```

Replace the example IDs with accounts on your chain. Other factories include
`limit_order_create`, `account_update`, `proposal_create` and Swaplock's
`data_room_create`. Their TypeScript inputs describe the required fields.
The generic path validates encoding and fees; chain-state rules are enforced by
the node. It does not perform every operation-specific balance/authority check.

## Sign with multiple keys

Pass multiple signers to a prepared transaction. The SDK deduplicates keys and
checks each returned signature. Supply all keys needed by the on-chain authority.

```ts
import { PrivateKey } from '@open-graphene/graphene';
import type { PreparedTransaction } from '@open-graphene/chain-swaplock-api';

async function signWithTwoKeys(
  prepared: PreparedTransaction,
  wifs: readonly string[],
) {
  const signers = wifs.map((wif) => PrivateKey.fromWif(wif));
  try {
    return await prepared.sign(signers);
  } finally {
    for (const signer of signers) signer.dispose();
  }
}
```

External signers can implement the `Signer` interface from
`@open-graphene/fc/signing`: expose `publicKey` bytes and an asynchronous
`signDigest(digest)` method. Generic signing does not recursively discover keys
for delegated account authorities.

## Attach an encrypted memo

Memo keys and transaction-signing keys can be different. Encrypt with the sender's
memo private key and the recipient's on-chain memo public key, then pass the memo
to `prepareTransfer`. Sign the resulting transaction with the active key as usual.

```ts
import {
  PrivateKey,
  PublicKey,
  encryptMemoWithWif,
  decryptMemoWithWif,
  uniqueNonce,
} from '@open-graphene/graphene';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import {
  MemoDataCodec,
  type MemoData,
} from '@open-graphene/chain-swaplock-bindings';

async function prepareWithMemo(
  client: SwaplockClient,
  memoWif: string,
  from: string,
  to: string,
  text: string,
) {
  const sender = await client.database.account(from);
  const recipient = await client.database.account(to);
  const memoKey = PrivateKey.fromWif(memoWif);
  try {
    if (memoKey.toPublicKey().toString('BTS') !== sender.options.memo_key) {
      throw new Error('The supplied key does not match the sender memo key');
    }
    const nonce = uniqueNonce();
    const message = await encryptMemoWithWif(
      memoWif,
      PublicKey.fromString(recipient.options.memo_key, 'BTS').bytes,
      nonce,
      new TextEncoder().encode(text),
    );
    const memo = MemoDataCodec.decode({
      from: sender.options.memo_key,
      to: recipient.options.memo_key,
      nonce,
      message,
    });
    return await client.prepareTransfer({
      from,
      to,
      amount: '0.01',
      asset: 'BTS',
      maxFee: 300000n,
      memo,
    });
  } finally {
    memoKey.dispose();
  }
}

async function readMemo(recipientMemoWif: string, memo: MemoData) {
  const plaintext = await decryptMemoWithWif(
    recipientMemoWif,
    PublicKey.fromString(memo.from, 'BTS').bytes,
    memo.nonce,
    memo.message,
  );
  return new TextDecoder().decode(plaintext);
}
```

Both current chain profiles use the `BTS` public-key prefix. `dispose()` clears
owned key bytes; JavaScript cannot erase immutable WIF strings or copies retained
by your application.

## Subscribe to changes

Watches return async iterators. They emit an initial snapshot, then refreshed
values when notices arrive. Close the stream when the screen or task is finished.

```ts
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';

async function watchFiveBlocks(client: SwaplockClient) {
  const stream = await client.database.watchDynamicGlobalProperties();
  try {
    let snapshots = 0;
    for await (const head of stream) {
      console.log('Head:', head.head_block_number);
      if (++snapshots === 5) break;
    }
  } finally {
    stream.close();
  }
}
```

The same pattern works with `watchAccount(name)`, `watchBalances(name)`,
`watchAccountOrders(name)`, `watchAsset(symbol)` and
`client.history.watchAccountHistory(name, limit, offset)`.
`client.database.subscribeMarket(baseAssetId, quoteAssetId)` returns market notices.

Streams have bounded queues. Overflow or reconnect ends the stream; obtain a new
snapshot and subscribe again. `client.reconnect()` reconnects explicitly, and
`client.setReconnectPolicy({ maxRetries: 2, delayMs: 250 })` configures retries for
eligible read calls. Writes and callback registration are never retried.

### Keep selected objects in a local cache

ChainStore tracks the object IDs you request and applies updates and deletions.
Its snapshots contain raw lossless JSON values; use generated codecs when you
need a specific typed object.

```ts
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';

async function watchObjects(client: SwaplockClient) {
  const store = await client.chainStore(['1.2.100', '2.1.0']);
  try {
    console.log('Initial account:', store.get('1.2.100'));
    for await (const snapshot of store) {
      console.log('Updated account:', snapshot.get('1.2.100'));
      break; // This example stops after the first update.
    }
  } finally {
    store.close();
  }
}
```

## Guard a Swaplock room mutation

An access precondition ties a mutation to the room state you just read. If another
transaction changes that state first, the node rejects the guarded mutation.
Read the new state and reconsider the change before preparing another transaction.

```ts
import {
  RoomAccessPrecondition,
  memberRemoveOperation,
  type SwaplockClient,
} from '@open-graphene/chain-swaplock-api';
import { DataRoomId } from '@open-graphene/chain-swaplock-bindings';

async function prepareMemberRemoval(
  client: SwaplockClient,
  callerId: string,
  roomId: string,
  memberId: string,
) {
  const state = await client.database.getDataRoomAccessState({
    room_id: DataRoomId(roomId),
  });
  if (!state) throw new Error('Room not found');

  const guard = RoomAccessPrecondition.fromSnapshot(state);
  const operation = memberRemoveOperation(callerId, roomId, memberId);
  return client.prepareOperations([guard.guard(operation)], {
    maxFee: 300000n,
  });
}
```

The caller must have the required room permission. Helpers also cover member
addition, key rotation and content-card grants; all room/card operation factories
are available in the Swaplock bindings.

## Preserve large numbers when working with JSON

Normal client calls already use checked codecs. When reading or storing protocol
JSON yourself, use the SDK's parser and serializer so large integers stay exact.
64/128-bit integers are `bigint`; binary fields are `Uint8Array` in decoded objects.

```ts
import {
  TransactionCodec,
  parseJson,
  stringifyJson,
} from '@open-graphene/chain-swaplock-bindings';

function roundTripTransaction(rawRpcJson: string) {
  const transaction = TransactionCodec.decode(parseJson(rawRpcJson));
  return stringifyJson(TransactionCodec.encode(transaction));
}
```

Native `JSON.parse` can round large integers before validation, and `JSON.stringify`
cannot serialize `bigint` directly. For a signed transaction, use
`stringifyJson(signed.toJSON())`.

## What is supported and tested?

| Capability                                           | Swaplock | BitShares |
| ---------------------------------------------------- | -------: | --------: |
| Operation variants with generated JSON bindings      |       95 |        78 |
| User operations with FC encoding and typed factories |       88 |        71 |
| Generated RPC methods                                |       47 |        32 |
| RPC methods confirmed live in the dated report       |       47 |        24 |

Each chain has seven additional **virtual operations**: records produced by the
blockchain, not operations a user can sign. FC is Graphene's binary transaction
format. All user-operation encoders were compared with Rust and native C++ using
318 baseline/rich vectors in total.

Both clients support compound transactions, memo, multiple signers, wallets,
reconnect, subscriptions and ChainStore. Swaplock transactions were also confirmed
live and irreversible. On the tested BitShares public nodes, seven crypto methods
were denied by permissions; mainnet broadcast was not performed.

This is capability parity with Rust's supported profile, not exhaustive coverage
of every payload or chain-state scenario. Unsupported populated extensions and
nonempty legacy address maps fail closed, as in Rust. See the
[dated parity report and transaction evidence](../../docs/TYPESCRIPT-RUST-PARITY-2026-09-29.md)
for exact results and limitations.

## Contributing and verification

Run these commands from this directory. Regeneration also needs the repository's
Rust toolchain and dependencies.

```sh
cargo fetch --locked
pnpm generate
pnpm generate:check
pnpm test
pnpm test:react:matrix
pnpm test:packages
pnpm exec playwright install chromium
pnpm test:browser
```

Edit the generator or source specification, then regenerate; do not hand-edit
`src/generated`. Both languages consume the specs in
`../rust/graphene-chain-*/graphene-chain-*-spec/dist/`.

Offline tests use public fixtures and do not need private keys. Live checks are
opt-in and depend on node availability and permissions:

```sh
# Read/fee-estimation smoke tests on Swaplock:
pnpm test:testnet
pnpm test:testnet:browser
# Read-only BitShares checks; no mainnet broadcast:
pnpm test:bitshares:live /tmp/bitshares-node.json
pnpm test:bitshares:live /tmp/bitshares-browser.json --browser
```

For full Swaplock RPC checks, fixture setup/cleanup and explicit transaction tests,
see the [live testing guide](../../docs/TYPESCRIPT-LIVE-TESTING.md).
The [original implementation plan](../../docs/TYPESCRIPT-PLAN.md) and
[reference fixtures](tests/fixtures/README.md) explain the design and test inputs.
