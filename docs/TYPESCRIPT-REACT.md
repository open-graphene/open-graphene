# Open Graphene for React

Typed TanStack Query hooks for **Swaplock and BitShares**, with optional live
updates through the SDK's WebSocket subscriptions.

- 39 generated read hooks for Swaplock and 24 for BitShares, each with a matching
  query-options factory for prefetching and loaders.
- Live hooks for accounts, balances, orders, history, assets and block properties.
- Swaplock room, room-access-state and content-card hooks.
- Generated preparation hooks and helpers for all 88 Swaplock and 71 BitShares
  user operations, plus separate sign, broadcast and inclusion mutations.
- Generated imperative hooks for the seven crypto RPCs and broadcast on each chain.
- Shared subscriptions, bounded reconnect retries and exact bigint/byte handling.

The chain adapters are separate packages:

| Package | Scope |
|---|---|
| `@open-graphene/chain-swaplock-react` | Swaplock hooks and operation preparation |
| `@open-graphene/chain-bitshares-react` | BitShares hooks and operation preparation |
| `@open-graphene/react-core` | Shared adapter, cache keys and lossless hydration |

Each chain package depends only on its own chain API/bindings and the shared core.
Import hooks from the matching chain package; import cache utilities from the core.
The former `@open-graphene/react/swaplock` and `/bitshares` entry points have been
replaced by these packages. In a pnpm workspace, declare the chain package with
`workspace:*`, plus React and TanStack Query. Declare `react-core` directly if you
import its cache utilities.

React and TanStack Query are peer dependencies. The core SDK does not depend on
React. Packages are not published to npm yet; build this workspace with
`pnpm install --frozen-lockfile && pnpm build` from the TypeScript root.

Use React 18.3 or 19 and TanStack Query 5.104 or later in the v5 series. Validation
currently runs with React 19.3 and TanStack Query 5.104 in Chromium.

## Set up a provider

Create the SDK client once, outside component rendering. The provider accepts an
already connected client, so your application retains control of connection and
key management. It does not close the client on unmount.

```tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Graphene } from '@open-graphene/graphene';
import { SwaplockProvider, useAccountBalances } from '@open-graphene/chain-swaplock-react';
import { CHAIN } from '@open-graphene/chain-swaplock-bindings';

const sdk = await Graphene.swaplock(
  [
    'wss://node01.swaplock.chainpool.online:8090',
    'wss://node02.swaplock.chainpool.online:8090',
  ],
  {
    expectedChainId: CHAIN.chainId,
  },
);
const queryClient = new QueryClient();

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <SwaplockProvider client={sdk}>
        <Balances />
      </SwaplockProvider>
    </QueryClientProvider>
  );
}

function Balances() {
  const balances = useAccountBalances('swaplock', { live: true });
  if (balances.isPending) return <p>Loading balances…</p>;
  if (balances.isError) return <p>{balances.error.message}</p>;
  return (
    <section>
      <p>Updates: {balances.live.status}</p>
      {balances.data.map(balance => (
        <p key={balance.asset_id}>
          {balance.asset_id}: {balance.amount.toString()} raw units
        </p>
      ))}
    </section>
  );
}
```

Call `sdk.close()` when your application no longer needs the connection. For
BitShares, use `Graphene.bitshares(...)`, `BitSharesProvider` and hooks from
`@open-graphene/chain-bitshares-react`. You can mount both providers in one application.

## Read once or keep data live

```tsx
import { useAccount, useAccountHistory } from '@open-graphene/chain-swaplock-react';

export function AccountDetails({ name }: { name: string }) {
  const account = useAccount(name, {
    enabled: name.length > 0,
    select: account => account.name,
    staleTime: 30_000,
  });
  const history = useAccountHistory(name, {
    enabled: name.length > 0,
    live: true,
    limit: 20,
    offset: 0,
  });
  return <p>{account.data}: {history.data?.items.length ?? 0} recent operations</p>;
}
```

`live` defaults to `false`. Live hooks fetch data and subscribe for refreshed
snapshots. Components with the same client, query and retry policy share one
stream, including through React Strict Mode's effect replay. When the last live
consumer unmounts, disables the hook or changes its parameters, the stream closes.
The cached data remains subject to TanStack Query's normal `gcTime`.

| Hook | Matching options factory | Live source |
|---|---|---|
| `useAccount(nameOrId)` | `accountOptions` | Account notices |
| `useAccountBalances(nameOrId, { assets })` | `accountBalancesOptions` | Account/balance notices |
| `useAccountOrders(nameOrId)` | `accountOrdersOptions` | Account/order notices |
| `useAccountHistory(nameOrId, { limit, offset })` | `accountHistoryOptions` | Account notices, then page refresh |
| `useAsset(symbolOrId)` | `assetOptions` | Asset notices |
| `useDynamicGlobalProperties()` | `dynamicGlobalPropertiesOptions` | Block-property notices |
| `useRoom(roomId)` | `roomOptions` | Block notices, then room refresh; Swaplock only |
| `useRoomAccessState(roomId)` | `roomAccessStateOptions` | Block notices, then state refresh; Swaplock only |
| `useContentCard(cardId)` | `contentCardOptions` | Block notices, then card refresh; Swaplock only |

Room/card reads refresh on shared database notices, including each new block;
they are not field-level event patches. History pages are refreshed snapshots,
not an immutable infinite-history cursor. Name-based and ID-based calls use
separate cache keys even when they refer to the same account.

### Connection errors and recovery

Every hook returns the normal query result plus `live` and `restartLive()`.
`live.status` is `disabled`, `connecting`, `live`, `reconnecting` or `error`.
Subscription errors are in `live.error`; the last good data remains available.
A successful query does not imply the subscription is healthy.

Transport failures trigger reconnect and a fresh subscription. Explicit SDK
reconnects and buffer overflow trigger resubscription/resnapshot. Permission and
codec errors stop automatic recovery. Configure bounded retries on the provider:

```tsx
import type { ReactNode } from 'react';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import { SwaplockProvider } from '@open-graphene/chain-swaplock-react';

export function Connection({ client, children }: { client: SwaplockClient; children: ReactNode }) {
  return (
    <SwaplockProvider client={client} livePolicy={{ maxRetries: 3, delayMs: 250 }}>
      {children}
    </SwaplockProvider>
  );
}
```

Retries use exponential delays capped at 60 seconds. The underlying SDK also has
its own read-retry policy. Cached query calls themselves do not add another retry
loop. Call `restartLive()` after resolving a persistent subscription error.

## Generated RPC hooks and prefetching

Each eligible RPC read gets two exports, named from its API and method. For
example, `database.get_accounts` becomes `useDatabaseGetAccounts` and
`databaseGetAccountsOptions`. The same protocol types and runtime codecs are used
by the SDK and by these generated hooks.

```tsx
import { QueryClient } from '@tanstack/react-query';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import { databaseGetAccountsOptions, useDatabaseGetAccounts } from '@open-graphene/chain-swaplock-react';

export async function preload(client: SwaplockClient, cache: QueryClient) {
  await cache.prefetchQuery(databaseGetAccountsOptions(client, {
    account_names_or_ids: ['swaplock'],
  }));
}

export function AccountName() {
  const result = useDatabaseGetAccounts({ account_names_or_ids: ['swaplock'] });
  return <p>{result.data?.[0]?.name}</p>;
}
```

Generated reads cover `get_*`, `lookup_*` and `list_*` methods in database, history
and orders APIs. They do not expose `live: true`; use the mapped live hooks above.
`subscribe: true` is rejected in cached RPC reads because registration must have
an explicit lifecycle. Crypto operations and broadcast are generated as imperative mutations rather
than queries. For example, `useCryptoBlind()` accepts its typed RPC parameters
through `mutateAsync`; `useNetworkBroadcastBroadcastTransaction()` accepts
`{ trx: signedTransaction }` and uses the SDK broadcast path. Both have retries
disabled. Crypto variables/results can contain sensitive blinding material:
exclude mutations from persistence and manage their lifetime in your application.

The options factories also work with `fetchQuery`, `ensureQueryData` and TanStack
Router loaders. Use the same SDK instance and params as the component for cache
reuse. Friendly helpers and raw generated RPC hooks have separate cache keys.

## Create a room with a generated hook

Every nonvirtual operation has a preparation hook generated from its protocol
fields. You do not need to assemble the operation tuple yourself:

```tsx
import { AccountId } from '@open-graphene/chain-swaplock-bindings';
import { usePrepareDataRoomCreate } from '@open-graphene/chain-swaplock-react';

export function CreateRoomButton({ ownerId }: { ownerId: string }) {
  const createRoom = usePrepareDataRoomCreate({ maxFee: 300000n });

  async function prepareRoom() {
    const prepared = await createRoom.mutateAsync({
      owner: AccountId(ownerId),
      name: 'Project documents',
      description: 'A room for the project team',
      subject: [0, {}],
      extensions: { write_policy: 1 },
    });
    // Hand prepared to your wallet/signing step, then broadcast explicitly.
    console.log('Prepared operations:', prepared.transaction.operations);
  }

  return (
    <div>
      <button disabled={createRoom.isPending} onClick={() => { void prepareRoom().catch(() => {}); }}>
        Prepare room
      </button>
      {createRoom.error && <p>{createRoom.error.message}</p>}
    </div>
  );
}
```

Preparation builds the operation, prices fees and checks the maximum fee. It
returns `PreparedTransaction`; it does not submit the transaction. Fee and
extensions inputs are optional because the existing operation factories provide
their defaults. Other required protocol fields remain required. Encryption,
permission choices and room-key envelopes stay under application control.

Hook names follow the operation name: `data_room_create` →
`usePrepareDataRoomCreate`, `data_room_member_add` → `usePrepareDataRoomMemberAdd`,
`content_card_create` → `usePrepareContentCardCreate`, and `limit_order_create` →
`usePrepareLimitOrderCreate`. For typed raw transfer inputs, use
`usePrepareTransferOperation`; the existing `usePrepareTransfer` convenience hook
continues to resolve names and decimal amounts.

The same generator emits helpers for use outside React components:

```ts
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import { AccountId } from '@open-graphene/chain-swaplock-bindings';
import { prepareDataRoomCreate } from '@open-graphene/chain-swaplock-react';

export function prepareRoom(client: SwaplockClient, ownerId: string) {
  return prepareDataRoomCreate(client, {
    owner: AccountId(ownerId),
    name: 'Project documents',
    description: '',
    subject: [0, {}],
    extensions: { write_policy: 1 },
  }, { maxFee: 300000n });
}
```

`generatedPrepareOperations` lists the available operation names. Virtual
operations are excluded because only the blockchain produces them.

## Prepare, sign and send a transaction

Mutations are separate so the application can review fees or request a wallet
signature before submission. Signers are supplied to the signing hook, not stored
in mutation variables. They remain owned and disposed by your application.

```tsx
import type { Signer } from '@open-graphene/fc/signing';
import {
  usePrepareTransfer, useSignTransaction,
  useBroadcastTransaction, useWaitForInclusion,
} from '@open-graphene/chain-swaplock-react';

export function SendButton({ signer, from, to }: { signer: Signer; from: string; to: string }) {
  const prepare = usePrepareTransfer();
  const sign = useSignTransaction(signer);
  const broadcast = useBroadcastTransaction();
  const inclusion = useWaitForInclusion();
  const busy = prepare.isPending || sign.isPending || broadcast.isPending || inclusion.isPending;
  const error = prepare.error ?? sign.error ?? broadcast.error ?? inclusion.error;

  async function send() {
    const prepared = await prepare.mutateAsync({
      from, to, amount: '0.01', asset: 'BTS', maxFee: 300000n,
    });
    const signed = await sign.mutateAsync(prepared);
    await broadcast.mutateAsync(signed);
    await inclusion.mutateAsync(signed);
  }

  return (
    <div>
      <button disabled={busy} onClick={() => { void send().catch(() => {}); }}>Send 0.01 BTS</button>
      {error && <p>{error.message}</p>}
      {inclusion.data && <p>Included in block {inclusion.data.blockNumber}</p>}
    </div>
  );
}
```

`usePrepareOperations()` accepts `{ operations, options }` for batches, room
mutations and all other supported operations. `useSignTransaction([signerA,
signerB])` supports multiple signatures. BitShares callers must use its canonical
signing profile (`BitSharesWifSigner` or `PrivateKey.fromWif(wif, 'graphene-legacy')`).

All mutation retries are explicitly disabled, including when the application's
QueryClient defaults enable them. Inclusion invalidates this client's scoped
queries; broadcast alone does not imply an on-chain state change. An ambiguous
broadcast must be reconciled by transaction ID before another send. Block inclusion
is not a guarantee of irreversibility.

## Cache identity, SSR and persistence

Cache keys include the chain ID, SDK client scope, query name and canonical
parameters. By default, each SDK client gets a distinct random scope. Parameter
encoding preserves bigint and bytes and distinguishes them from strings/arrays.

For deliberate server-to-browser hydration, choose a stable scope and use it in
both options factories and the matching provider. Create a QueryClient per server
request. Never reuse a scope across different authorization contexts.

```ts
import { QueryClient, dehydrate, hydrate, type DehydratedState } from '@tanstack/react-query';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import { accountOptions } from '@open-graphene/chain-swaplock-react';
import { serializeCache, deserializeCache } from '@open-graphene/react-core';

export async function serverPayload(client: SwaplockClient) {
  const cache = new QueryClient();
  try {
    await cache.prefetchQuery(accountOptions(client, 'swaplock', 'public-swaplock'));
    return serializeCache(dehydrate(cache, { shouldDehydrateMutation: () => false }));
  } finally { cache.clear(); }
}

export function restore(cache: QueryClient, payload: string) {
  hydrate(cache, deserializeCache(payload) as DehydratedState);
}
```

On the browser provider, pass `scope="public-swaplock"`. Live subscriptions start
in effects, not during server rendering. The serializer preserves successful
protocol query data, including bigint and Uint8Array; it rejects arbitrary class
instances. Pass its output through your framework's safe data transport rather
than interpolating it into an HTML script tag. Exclude mutations from persistence.

## Development and validation

From the TypeScript workspace root:

```sh
pnpm generate
pnpm generate:check
pnpm test
pnpm test:react:matrix
pnpm test:react:browser
# Read-only browser checks against both Swaplock and both BitShares endpoints:
pnpm test:react:live
```

`generate-react.mjs` reads the shared chain IR and emits the files under
each chain React package's `src/generated`. Do not edit those files by hand. Live mappings and lifecycle
handling are maintained in the React adapter; no duplicate protocol types or
serializers are introduced.

Tests cover complete RPC inventories, all 159 operation preparations against
independent Rust/C++ vectors, selected result types, cache isolation,
lossless hydration, shared streams, Strict Mode, pending-open cleanup, stale-read
races, disabled hooks, parameter/client changes, reconnect, buffer overflow,
retry limits, broadcast retry suppression and inclusion invalidation. Live tests
observe consecutive blocks through actual React hooks without sending transactions.

See the [SDK guide](../open-graphene-packages/typescript/README.md) for operation construction, memo and room guards.

On 2026-09-30, live React checks observed successive blocks on both Swaplock RPC
nodes and both public BitShares endpoints. No transactions were broadcast.
See the [recorded live results](TYPESCRIPT-REACT-LIVE-2026-09-30.json).
Before the package split, the expanded suite passed 54 Node tests and Chromium
checks, including all 159 generated preparation hooks. After the split, 55 Node
tests and ten documentation examples passed; dependency-graph and browser-bundle
checks confirmed chain isolation. The local Chromium rerun was blocked by the
sandbox's macOS MachPortRendezvous permission restriction.

The React DOM matrix subsequently passed on React 18.3.1 and 19.3.0, including
all 159 generated preparation hooks and the create/sign/broadcast/read/delete
room pipeline against simulated RPC. This complements the Chromium suite; it
does not prove live transaction execution. The new opt-in room test was blocked
at the testnet connection in this environment. See the
[release and live transaction guide](TYPESCRIPT-RELEASE.md) for commands and
transaction journal handling.
