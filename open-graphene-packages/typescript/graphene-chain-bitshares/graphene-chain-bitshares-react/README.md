# BitShares for React

`@open-graphene/chain-bitshares-react` provides 24 generated read hooks/query
options, eight imperative RPC hooks and preparation hooks for all 71 user
operations. It uses its own chain API/bindings and `@open-graphene/react-core`;
it does not depend on the other chain's SDK.

Live hooks cover accounts, balances, orders, history, assets and block properties.

React 18.3/19 and TanStack Query 5.104+ (v5) are peer dependencies. Build from the
TypeScript workspace; this package is not published to npm yet.

```tsx
import { BitSharesProvider, useAccountBalances } from '@open-graphene/chain-bitshares-react';

function Balances() {
  const result = useAccountBalances('your-account', { live: true });
  return <p>{result.data?.length ?? 0} balances; {result.live.status}</p>;
}
```

Mount `BitSharesProvider` with an already connected `BitSharesClient` inside your
application's `QueryClientProvider`. The application owns and closes the SDK client.

See the [complete React guide](../../../../docs/TYPESCRIPT-REACT.md) for connection,
subscriptions, operation hooks, transactions, prefetching and SSR hydration.
