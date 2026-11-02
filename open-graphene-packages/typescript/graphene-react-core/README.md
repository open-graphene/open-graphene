# Open Graphene React core

Shared TanStack Query adapter for the independent Actanet, BitShares and R-Squared
React packages. This package has no chain-specific SDK dependency.

Applications normally import hooks from `@open-graphene/chain-actanet-react`,
`@open-graphene/chain-bitshares-react` or `@open-graphene/chain-rsquared-react`. Import `serializeCache`, `deserializeCache`,
`grapheneQueryKey` and their shared types from this package when needed.

React and TanStack Query are peer dependencies. Build through the TypeScript
workspace; packages have not been published to npm.

See the [React integration guide](../../../docs/TYPESCRIPT-REACT.md) for providers,
shared subscriptions, transaction mutations and exact bigint/byte hydration.

## Sharing a connection across SDKs

Create one lazy `createSharedConnection({ endpoints, expectedChainId })` from
`@open-graphene/transport` for the host lifetime and provide it through the
chain's provider, for example `<ActanetConnectionProvider connection={connection}>`.
Consumers call `useActanetConnection()` and pass the result as `connection` in
their SDK's network options. Each chain package makes its own provider with
`createConnectionContext`, so two chains never share one by mistake.

Each SDK session borrows the same physical WebSocket. Closing a session cancels
only its own requests and subscriptions. The host must call `connection.close()`
on shutdown; the provider borrows it and does not close it on React remounts.
