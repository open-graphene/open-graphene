# Open Graphene React core

Shared TanStack Query adapter for the independent Swaplock and BitShares React
packages. This package has no chain-specific SDK dependency.

Applications normally import hooks from `@open-graphene/chain-swaplock-react` or
`@open-graphene/chain-bitshares-react`. Import `serializeCache`, `deserializeCache`,
`grapheneQueryKey` and their shared types from this package when needed.

React and TanStack Query are peer dependencies. Build through the TypeScript
workspace; packages have not been published to npm.

See the [React integration guide](../../../docs/TYPESCRIPT-REACT.md) for providers,
shared subscriptions, transaction mutations and exact bigint/byte hydration.
