# R-Squared for React

`@open-graphene/chain-rsquared-react` provides 30 generated read hooks/query
options, one imperative RPC hook and preparation hooks for all 59 user
operations. It uses its own chain API/bindings and `@open-graphene/react-core`;
it does not depend on another chain's SDK.

Live hooks cover accounts, balances, orders, history, assets and block properties.

React 18.3/19 and TanStack Query 5.104+ (v5) are peer dependencies. Build from the
TypeScript workspace; this package is not published to npm yet.

```tsx
import {
  RSquaredProvider,
  useAccountBalances,
} from '@open-graphene/chain-rsquared-react';

function Balances() {
  const result = useAccountBalances('your-account', { live: true });
  return (
    <p>
      {result.data?.length ?? 0} balances; {result.live.status}
    </p>
  );
}
```

Mount `RSquaredProvider` with an already connected `RSquaredClient` inside your
application's `QueryClientProvider`. The application owns and closes the SDK client.

See the [complete React guide](../../../../docs/TYPESCRIPT-REACT.md) for connection,
subscriptions, operation hooks, transactions, prefetching and SSR hydration.
