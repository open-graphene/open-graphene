# Open Graphene Rust Packages

This directory contains the active Rust runtime crates, pure SDK helper crates, generated chain binding crates, and the public `graphene` facade for Open Graphene.

## Crates

- `graphene` (`graphene`): top-level user-facing facade for chain clients and examples. Actanet is its default chain (its API sits at the crate root); BitShares and R-Squared are in `graphene::bitshares` and `graphene::rsquared`.
- `graphene-chain-api`: the high-level API source shared by every chain (sessions, queries, live subscriptions, operation builders, signing, broadcast). Not a crate itself: `graphene-chain-{actanet,bitshares,rsquared}-api` build it against their bindings. See [`graphene-chain-api`](graphene-chain-api/README.md).
- `graphene-chain-actanet-bindings`: generated Actanet protocol and wire bindings; `graphene-chain-actanet-wasm` builds the shared `graphene-chain-wasm` source (browser-side transaction digest) against them.
- `graphene-chain-rsquared-bindings`: generated R-Squared protocol, JSON/RPC and FC wire bindings. See [R-Squared](../../docs/RSQUARED.md) for generation and coverage.
- `graphene-chain-bitshares-bindings`: generated BitShares protocol and wire bindings used to keep cross-chain generation honest.
- `graphene-fc` (`open-graphene-fc`): FC serialization, signing, digest, and WIF helpers.
- `graphene-primitives` (`open-graphene-primitives`): stable chain-agnostic SDK value references and validators such as object IDs, account IDs, asset IDs, asset amounts, limit-order IDs, and operation-history IDs.
- `graphene-core` (`open-graphene-core`): pure SDK helpers such as amount conversion, transaction headers, and balance checks; re-exports SDK primitives for compatibility.
- `graphene-transport` (`open-graphene-transport`): chain-agnostic Graphene JSON-RPC request, response, error, notice, blocking WebSocket, session bootstrap, and reusable live RPC envelope helpers. See [`graphene-transport`](graphene-transport/README.md) for the live crate API.

## Current boundaries

- Generated chain binding crates own raw protocol and wire types.
- `graphene-transport` is Graphene-generic JSON-RPC transport and does not import generated chain bindings.
- `graphene-core` stays pure: no RPC, signing, broadcast, generated bindings, or chain-specific transaction construction.
- `graphene-chain-api` is the high-level surface of every chain, compiled once per chain against its bindings. It exposes both the blocking request-builder API and the typed live wrapper over `open-graphene-transport::LiveTransport`.
- Signing, broadcast, fee policy, and confirmation policy stay explicit at call sites unless a later high-level API deliberately chooses those policies.

## Typed live flow

The typed live wrapper keeps `open-graphene-transport` generic while giving callers typed subscription and broadcast-confirmation surfaces:

```rust,no_run
use std::time::Duration;
use graphene::Graphene;

# async fn example(signed: graphene::SignedTransfer) -> Result<(), Box<dyn std::error::Error>> {
let live = Graphene::builder()
    .servers(["ws://127.0.0.1:8090"])
    .prefix(graphene::PUBLIC_KEY_PREFIX)
    .build()?
    .actanet()
    .connect_live()
    .await?;

let dgp = live
    .database()
    .subscribe_dynamic_global_properties_timeout(Duration::from_secs(30))?;

let account = live
    .database()
    .account_by_id("1.2.100")
    .subscribe_timeout(Duration::from_secs(30))?;

let asset = live
    .database()
    .asset_by_id("1.3.0")
    .subscribe_timeout(Duration::from_secs(30))?;

let balances = live
    .database()
    .account_balances_by_id("1.2.100", ["1.3.0"])
    .subscribe_timeout(Duration::from_secs(30))?;

let orders = live
    .database()
    .account_orders_by_id("1.2.100")
    .subscribe_timeout(Duration::from_secs(30))?;

let history = live
    .history()?
    .account_history_by_id("1.2.100")
    .limit(20)
    .offset(0)
    .subscribe_timeout(Duration::from_secs(30))?;

let pending = live
    .network_broadcast()?
    .send_signed_transfer_with_callback(signed)?;

let confirmation = pending.wait_timeout(Duration::from_secs(30))?;
let update = dgp.next_update_timeout(Duration::from_secs(30))?;
# Ok(())
# }
```

This live wrapper currently covers dynamic-global-properties, account, asset, account-balance, account-order, and account-history subscriptions plus broadcast callback confirmations. It intentionally does not reconnect, resubscribe, or maintain a ChainStore/cache yet. Database and history live subscriptions share one Graphene `database.set_subscribe_callback` id on the WebSocket and multicast notices locally; each typed subscription filters or reconciles the shared notice stream.

## API inventory

| Surface | Blocking API shape | Live API shape | Proof example |
|---|---|---|---|
| Dynamic global properties | `actanet.database().dynamic_global_properties().get().await?` | `live.database().subscribe_dynamic_global_properties_timeout(...)` | `actanet_database_dynamic_global_properties_subscribe`, `actanet_live_database_object_subscribe` |
| Account object | `actanet.database().account_by_id("1.2.100").get().await?` | `live.database().account_by_id("1.2.100").subscribe_timeout(...)` | `actanet_database_account_subscribe`, `actanet_live_database_object_subscribe` |
| Asset object | `actanet.database().asset_by_id("1.3.0").get().await?` | `live.database().asset_by_id("1.3.0").subscribe_timeout(...)` | `actanet_database_asset_subscribe`, `actanet_live_database_object_subscribe` |
| Account balances | `actanet.database().account_balances("actanet", []).subscribe().await?` | `live.database().account_balances_by_id("1.2.100", ["1.3.0"]).subscribe_timeout(...)` | `actanet_database_account_balances_subscribe`, `actanet_live_account_activity_subscribe` |
| Account orders | `actanet.database().account_orders("actanet").subscribe().await?` | `live.database().account_orders_by_id("1.2.100").subscribe_timeout(...)` | `actanet_database_account_orders_subscribe`, `actanet_live_account_activity_subscribe` |
| Account history | `actanet.history().account_history("actanet").limit(20).offset(0).subscribe().await?` | `live.history()?.account_history_by_id("1.2.100").limit(20).offset(0).subscribe_timeout(...)` | `actanet_history_account_subscribe`, `actanet_live_history_account_subscribe` |
| Broadcast callback confirmation | `actanet.network_broadcast().broadcast_signed_transfer_with_callback_timeout(...)` | `live.network_broadcast()?.send_signed_transfer_with_callback(signed)?.wait_timeout(...)` | `actanet_operations_transfer_broadcast_with_callback`, `actanet_live_transfer_callback_stress`, `actanet_live_transfer_updates_e2e` (mutates the configured network and requires `ACTANET_ACTIVE_WIF`; nodes from `ACTANET_RPC_SERVERS`) |

Historical spike/design markdowns, the older live SDK stack, and unused shared operation-adapter helpers were removed once their useful constraints had been folded into code, crate READMEs, and GSD decisions.
