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

| Package | Use it for |
|---|---|
| `@open-graphene/chain-swaplock-react` | Swaplock React hooks, live updates and operation preparation |
| `@open-graphene/chain-bitshares-react` | BitShares React hooks, live updates and operation preparation |
| `@open-graphene/react-core` | Shared React adapter and cache utilities |
| `@open-graphene/graphene` | Connect to either chain; access wallet, memo and amount helpers |
| `@open-graphene/chain-swaplock-api` | Swaplock transactions, queries and room-access helpers |
| `@open-graphene/chain-bitshares-api` | BitShares transactions and queries |
| `@open-graphene/chain-swaplock-bindings` | Swaplock protocol types, operation factories and codecs |
| `@open-graphene/chain-bitshares-bindings` | BitShares protocol types, operation factories and codecs |
| `@open-graphene/core` | Amount formatting, account-name validation and authority analysis |
| `@open-graphene/fc` | Binary serialization; `/signing`, `/memo`, `/wallet` and `/hash` exports |
| `@open-graphene/transport` | WebSocket sessions, subscriptions and ChainStore |
| `@open-graphene/codec` | Lossless JSON and checked runtime codecs |
| `@open-graphene/primitives` | IDs, timestamps and byte conversions |

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

const client = await Graphene.swaplock([
  'wss://node01.swaplock.chainpool.online:8090',
  'wss://node02.swaplock.chainpool.online:8090',
]);

try {
  const account = await client.database.account('swaplock');
  const balances = await client.database.accountBalances(account.id);
  const core = await client.database.asset('1.3.0');
  const balance = balances.find(item => item.asset_id === core.id);

  console.log(account.id, account.name);
  console.log(formatRawAmount(balance?.amount ?? 0n, core.precision), core.symbol);
} finally {
  client.close();
}
```

The client checks the chain ID. An endpoint list provides connection fallback;
`{ strategy: 'lowest-latency' }` selects by measured connection latency.
Automatic reconnect uses the selected endpoint and checks the chain ID again.

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
    const nextPage = await client.history.accountHistory(accountName, 20, page.nextOffset);
    console.log(nextPage.items);
  }

  const orders = await client.database.accountOrders(accountName);
  const head = await client.database.getDynamicGlobalProperties({});
  console.log(orders, head.head_block_number);
}
```

History takes `(accountNameOrId, limit, offset)`; `limit + offset` must be at most
98. New transactions can shift offset-based pages. To read a market ticker, call
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
  client: SwaplockClient, activeWif: string, from: string, to: string,
) {
  const signer = PrivateKey.fromWif(activeWif);
  try {
    const prepared = await client.prepareTransfer({
      from,
      to,
      amount: '0.01',       // Decimal asset units, expressed as a string.
      asset: 'BTS',
      maxFee: 300000n,      // Raw fee units: 3 BTS when precision is 5.
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
      from, to, amount: '0.01', asset: 'BTS', maxFee: 300000n,
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
  AccountId, AssetId, bindOperationBuilders,
} from '@open-graphene/chain-swaplock-bindings';

async function prepareTwoTransfers(client: SwaplockClient) {
  const op = bindOperationBuilders(operation => operation);
  const from = AccountId('1.2.100');
  const to = AccountId('1.2.101');
  const asset_id = AssetId('1.3.0');

  return client.prepareOperations([
    op.transfer({ from, to, amount: { amount: 1n, asset_id } }),
    op.transfer({ from, to, amount: { amount: 2n, asset_id } }),
  ], { maxFee: 300000n, expirationSeconds: 120 });
}

async function prepareOneTransfer(client: SwaplockClient) {
  return client.operations.transfer({
    from: AccountId('1.2.100'),
    to: AccountId('1.2.101'),
    amount: { amount: 1n, asset_id: AssetId('1.3.0') },
  }).maxFee(300000n).expiration(120).prepare();
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

async function signWithTwoKeys(prepared: PreparedTransaction, wifs: readonly string[]) {
  const signers = wifs.map(wif => PrivateKey.fromWif(wif));
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
  PrivateKey, PublicKey, encryptMemoWithWif, decryptMemoWithWif, uniqueNonce,
} from '@open-graphene/graphene';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import { MemoDataCodec, type MemoData } from '@open-graphene/chain-swaplock-bindings';

async function prepareWithMemo(
  client: SwaplockClient, memoWif: string, from: string, to: string, text: string,
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
      memoWif, PublicKey.fromString(recipient.options.memo_key, 'BTS').bytes,
      nonce, new TextEncoder().encode(text),
    );
    const memo = MemoDataCodec.decode({
      from: sender.options.memo_key, to: recipient.options.memo_key, nonce, message,
    });
    return await client.prepareTransfer({
      from, to, amount: '0.01', asset: 'BTS', maxFee: 300000n, memo,
    });
  } finally {
    memoKey.dispose();
  }
}

async function readMemo(recipientMemoWif: string, memo: MemoData) {
  const plaintext = await decryptMemoWithWif(
    recipientMemoWif, PublicKey.fromString(memo.from, 'BTS').bytes,
    memo.nonce, memo.message,
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
  RoomAccessPrecondition, memberRemoveOperation, type SwaplockClient,
} from '@open-graphene/chain-swaplock-api';
import { DataRoomId } from '@open-graphene/chain-swaplock-bindings';

async function prepareMemberRemoval(
  client: SwaplockClient, callerId: string, roomId: string, memberId: string,
) {
  const state = await client.database.getDataRoomAccessState({
    room_id: DataRoomId(roomId),
  });
  if (!state) throw new Error('Room not found');

  const guard = RoomAccessPrecondition.fromSnapshot(state);
  const operation = memberRemoveOperation(callerId, roomId, memberId);
  return client.prepareOperations([guard.guard(operation)], { maxFee: 300000n });
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
  TransactionCodec, parseJson, stringifyJson,
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

| Capability | Swaplock | BitShares |
|---|---:|---:|
| Operation variants with generated JSON bindings | 95 | 78 |
| User operations with FC encoding and typed factories | 88 | 71 |
| Generated RPC methods | 47 | 32 |
| RPC methods confirmed live in the dated report | 47 | 24 |

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
