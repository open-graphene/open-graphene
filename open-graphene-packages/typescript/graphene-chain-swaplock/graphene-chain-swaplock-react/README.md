# Swaplock for React

`@open-graphene/chain-swaplock-react` provides 39 generated read hooks/query
options, eight imperative RPC hooks and preparation hooks for all 88 user
operations. It uses its own chain API/bindings and `@open-graphene/react-core`;
it does not depend on the other chain's SDK.

Live hooks cover accounts, balances, orders, history, assets and block properties.
Swaplock also has live room, access-state and content-card hooks.

React 18.3/19 and TanStack Query 5.104+ (v5) are peer dependencies. Build from the
TypeScript workspace; this package is not published to npm yet.

```tsx
import {
  SwaplockProvider,
  useAccountBalances,
} from '@open-graphene/chain-swaplock-react';

function Balances() {
  const result = useAccountBalances('your-account', { live: true });
  return (
    <p>
      {result.data?.length ?? 0} balances; {result.live.status}
    </p>
  );
}
```

Mount `SwaplockProvider` with an already connected `SwaplockClient` inside your
application's `QueryClientProvider`. The application owns and closes the SDK client.

Use `usePrepareDataRoomCreate({ maxFee: 300000n })` to prepare a room directly
from its typed fields. Other operations have matching generated `usePrepare…`
hooks. Signing and broadcasting are separate explicit steps.

See the [complete React guide](../../../../docs/TYPESCRIPT-REACT.md) for connection,
subscriptions, operation hooks, transactions, prefetching and SSR hydration.
