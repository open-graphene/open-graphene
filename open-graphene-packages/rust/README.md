# Open Graphene Rust Packages

This directory contains Rust runtime crates, SDK helper crates, and generated chain binding crates for Open Graphene.

## Crates

- `graphene-fc` (`open-graphene-fc`): FC serialization, signing, digest, and WIF helpers.
- `graphene-sdk-primitives` (`open-graphene-sdk-primitives`): stable chain-agnostic SDK value references and validators such as object IDs, account IDs, asset IDs, asset amounts, limit-order IDs, and operation-history IDs.
- `graphene-sdk-core` (`open-graphene-sdk-core`): pure SDK helpers such as amount conversion, transaction headers, and balance checks; re-exports SDK primitives for compatibility.
- `graphene-sdk-operations` (`open-graphene-sdk-operations`): common operation input models, adapter traits, and generic trait-based transaction builders, split one operation per module.
- `graphene-sdk-live` (`open-graphene-sdk-live`): shared read-only live SDK client above transport and below chain-specific crates; exposes chain profiles, chain-id validation, session access, and common chain-state reads. See [`graphene-sdk-live`](graphene-sdk-live/README.md) for the current live SDK API.
- `graphene-transport` (`open-graphene-transport`): chain-agnostic Graphene JSON-RPC request, response, error, notice, blocking WebSocket, session bootstrap, and reusable live RPC envelope helpers. See [`graphene-transport`](graphene-transport/README.md) for the live crate API.
- `graphene-chain-swaplock`: Swaplock-specific SDK operation modules and examples; depends on `graphene-chain-swaplock-bindings` for generated protocol types, while the bindings crate itself contains no high-level SDK operation code. See [`graphene-chain-swaplock`](graphene-chain-swaplock/graphene-chain-swaplock/README.md) for the public helper API boundary and live-operation flow.

## Architecture notes

- [Manual SDK Adapter Patterns](TRANSFER-SDK-ADAPTER.md) describes the operation helper boundary used by chain binding crates and the current trait-builder pattern.
- [Common SDK Input and Chain Adapter Design](SDK-INPUT-ADAPTER-DESIGN.md) describes the shared input-model seam and how chain-specific generated types plug into common operation builders.
- [SDK Builder Bridge Ergonomics Spike](SDK-BUILDER-BRIDGE-ERGONOMICS.md) measures current Swaplock/BitShares bridge duplication and records why the explicit generated-type bridge remains preferable for now.
- [Limit Order Create SDK Design Spike](LIMIT-ORDER-CREATE-SDK-DESIGN.md) compares raw protocol-shaped, price-based, and builder-style input designs and records the implemented raw `limit_order_create` flow.
- [Generated SDK Adapter Capability Design](../../open-graphene/crates/open-graphene-gen-bindings-rs/SDK-CAPABILITY-DESIGN.md) records the opt-in generated SDK adapter direction and the boundary for small protocol-level generated helpers.
- [Graphene Transport Design](GRAPHENE-TRANSPORT-DESIGN.md) describes the chain-agnostic JSON-RPC transport layer, WebSocket notice semantics, HTTP call-only direction, and reconnect boundaries.
- [Open Graphene Transport API](graphene-transport/README.md) documents the current live `open-graphene-transport` crate API, including `GrapheneSession`, blocking WebSocket calls, database/history/broadcast helpers, and non-goals.
- [Open Graphene SDK Live API](graphene-sdk-live/README.md) documents the current `open-graphene-sdk-live` API, including chain profiles, `GrapheneLiveClient`, read helpers, facade integration, and non-goals.
- [Open Graphene SDK Live Design](SDK-LIVE-DESIGN.md) defines the proposed shared live SDK boundary above transport and below chain-specific generated bindings/signing/operation code.
- [Binding Usage and Live Typing Audit](BINDINGS-LIVE-TYPING-AUDIT.md) freezes new SDK/live feature growth and records why current bindings cover protocol transactions but not yet full typed live RPC payloads.
- [Live Object Generation Plan](LIVE-OBJECT-GENERATION-PLAN.md) defines the first generator milestone for typed live RPC objects, starting with `limit_order_object`.
- [Typed `get_objects` Routing Design](TYPED-GET-OBJECTS-DESIGN.md) explains why `get_objects` needs object-id routing, positional `null` handling, and fail-closed generated helpers for object proofs such as `asset_object` and `account_object`.

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
