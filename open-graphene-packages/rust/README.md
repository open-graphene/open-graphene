# Open Graphene Rust Packages

This directory contains the active Rust runtime crates, pure SDK helper crates, generated chain binding crates, and the public `graphene` facade for Open Graphene.

## Crates

- `graphene` (`graphene`): top-level user-facing facade for chain clients and examples.
- `graphene-chain-swaplock-api`: current high-level Swaplock API surface used by the top-level `graphene` facade.
- `graphene-chain-swaplock-bindings`: generated Swaplock protocol and wire bindings.
- `graphene-chain-bitshares-bindings`: generated BitShares protocol and wire bindings used to keep cross-chain generation honest.
- `graphene-fc` (`open-graphene-fc`): FC serialization, signing, digest, and WIF helpers.
- `graphene-primitives` (`open-graphene-primitives`): stable chain-agnostic SDK value references and validators such as object IDs, account IDs, asset IDs, asset amounts, limit-order IDs, and operation-history IDs.
- `graphene-core` (`open-graphene-core`): pure SDK helpers such as amount conversion, transaction headers, and balance checks; re-exports SDK primitives for compatibility.
- `graphene-transport` (`open-graphene-transport`): chain-agnostic Graphene JSON-RPC request, response, error, notice, blocking WebSocket, session bootstrap, and reusable live RPC envelope helpers. See [`graphene-transport`](graphene-transport/README.md) for the live crate API.

## Current boundaries

- Generated chain binding crates own raw protocol and wire types.
- `graphene-transport` is Graphene-generic JSON-RPC transport and does not import generated chain bindings.
- `graphene-core` stays pure: no RPC, signing, broadcast, generated bindings, or chain-specific transaction construction.
- `graphene-chain-swaplock-api` is the current high-level Swaplock surface used by the top-level `graphene` facade.
- Signing, broadcast, fee policy, and confirmation policy stay explicit at call sites unless a later high-level API deliberately chooses those policies.

Historical spike/design markdowns, the older Swaplock live SDK stack, and unused shared operation-adapter helpers were removed once their useful constraints had been folded into code, crate READMEs, and GSD decisions.
