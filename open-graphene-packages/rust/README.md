# Open Graphene Rust Packages

This directory contains Rust runtime crates, SDK helper crates, and generated chain binding crates for Open Graphene.

## Crates

- `graphene-fc` (`open-graphene-fc`): FC serialization, signing, digest, and WIF helpers.
- `graphene-sdk-primitives` (`open-graphene-sdk-primitives`): stable chain-agnostic SDK value references and validators such as object IDs, account IDs, asset IDs, asset amounts, limit-order IDs, and operation-history IDs.
- `graphene-sdk-core` (`open-graphene-sdk-core`): pure SDK helpers such as amount conversion, transaction headers, and balance checks; re-exports SDK primitives for compatibility.
- `graphene-sdk-operations` (`open-graphene-sdk-operations`): common operation input models, adapter traits, and generic trait-based transaction builders, split one operation per module.
- `graphene-transport` (`open-graphene-transport`): chain-agnostic Graphene JSON-RPC request, response, error, notice, blocking WebSocket, session bootstrap, and reusable live RPC envelope helpers. See [`graphene-transport`](graphene-transport/README.md) for the live crate API.
- `graphene-chain-swaplock-api`: current high-level Swaplock API surface used by the top-level `graphene` facade.
- `graphene-chain-swaplock` (legacy, outside the root workspace): older Swaplock-specific SDK operation modules and live examples kept as a migration reference while missing write flows are moved to `graphene-chain-swaplock-api`.

## Current boundaries

- Generated chain binding crates own raw protocol and wire types.
- `graphene-transport` is Graphene-generic JSON-RPC transport and does not import generated chain bindings.
- `graphene-sdk-core` stays pure: no RPC, signing, broadcast, generated bindings, or chain-specific transaction construction.
- `graphene-sdk-operations` owns shared operation input models, adapter traits, and generic transaction-builder helpers.
- `graphene-chain-swaplock-api` is the current high-level Swaplock surface used by the top-level `graphene` facade.
- The older `graphene-chain-swaplock` crate remains outside the root workspace only as a legacy live-operation reference while missing write flows are migrated; it is not the public client path.
- Signing, broadcast, fee policy, and confirmation policy stay explicit at call sites unless a later high-level API deliberately chooses those policies.

Historical spike/design markdowns were removed from this directory once their useful constraints had been folded into code, crate READMEs, and GSD decisions.

## Swaplock live trading scenario

The private Swaplock testnet can run an end-to-end SDK scenario from the `graphene-chain-swaplock` crate examples. The scenario creates two temporary accounts, creates two UIA assets, funds the accounts, issues the assets, opens a deliberately unmatched limit order, and cancels it again:

```bash
open-graphene-packages/rust/graphene-chain-swaplock/bin/trading-scenario.sh
```

Required environment keys are loaded from `.env` when present:

```text
SWAPLOCK_RPC_URL
SWAPLOCK_ACTIVE_WIF
SWAPLOCK_ACCOUNT
```

Optional overrides include `SWAPLOCK_SCENARIO_ACCOUNT_A`, `SWAPLOCK_SCENARIO_ACCOUNT_B`, `SWAPLOCK_SCENARIO_ASSET_A`, `SWAPLOCK_SCENARIO_ASSET_B`, `SWAPLOCK_SCENARIO_CORE_FUNDING`, `SWAPLOCK_SCENARIO_ISSUE_AMOUNT`, `SWAPLOCK_SCENARIO_ORDER_SELL_AMOUNT`, and `SWAPLOCK_SCENARIO_ORDER_RECEIVE_AMOUNT`.
