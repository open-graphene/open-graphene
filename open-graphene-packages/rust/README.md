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
- `graphene-chain-swaplock-api` is the current high-level Swaplock surface used by the top-level `graphene` facade. It exposes both the blocking request-builder API and the first typed live wrapper over `open-graphene-transport::LiveTransport`.
- Signing, broadcast, fee policy, and confirmation policy stay explicit at call sites unless a later high-level API deliberately chooses those policies.

## Typed Swaplock live flow

The first typed live wrapper keeps `open-graphene-transport` generic while giving Swaplock callers typed subscription and broadcast-confirmation surfaces:

```rust,no_run
use std::time::Duration;
use graphene::Graphene;

# async fn example(signed: graphene::SignedTransfer) -> Result<(), Box<dyn std::error::Error>> {
let live = Graphene::builder()
    .servers([
        "wss://node01.swaplock.chainpool.online:8090",
        "wss://node02.swaplock.chainpool.online:8090",
    ])
    .chain_id("95acb1f01e4afd0dd8b61f99377911a2f0ac6e6420925e01630f1d3580c89758")
    .prefix("BTS")
    .build()?
    .swaplock()
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

## Swaplock API inventory

| Surface | Blocking API shape | Live API shape | Proof example |
|---|---|---|---|
| Dynamic global properties | `swaplock.database().dynamic_global_properties().get().await?` | `live.database().subscribe_dynamic_global_properties_timeout(...)` | `swaplock_database_dynamic_global_properties_subscribe`, `swaplock_live_database_object_subscribe` |
| Account object | `swaplock.database().account_by_id("1.2.100").get().await?` | `live.database().account_by_id("1.2.100").subscribe_timeout(...)` | `swaplock_database_account_subscribe`, `swaplock_live_database_object_subscribe` |
| Asset object | `swaplock.database().asset_by_id("1.3.0").get().await?` | `live.database().asset_by_id("1.3.0").subscribe_timeout(...)` | `swaplock_database_asset_subscribe`, `swaplock_live_database_object_subscribe` |
| Account balances | `swaplock.database().account_balances("swaplock", []).subscribe().await?` | `live.database().account_balances_by_id("1.2.100", ["1.3.0"]).subscribe_timeout(...)` | `swaplock_database_account_balances_subscribe`, `swaplock_live_account_activity_subscribe` |
| Account orders | `swaplock.database().account_orders("swaplock").subscribe().await?` | `live.database().account_orders_by_id("1.2.100").subscribe_timeout(...)` | `swaplock_database_account_orders_subscribe`, `swaplock_live_account_activity_subscribe` |
| Account history | `swaplock.history().account_history("swaplock").limit(20).offset(0).subscribe().await?` | `live.history()?.account_history_by_id("1.2.100").limit(20).offset(0).subscribe_timeout(...)` | `swaplock_history_account_subscribe`, `swaplock_live_history_account_subscribe` |
| Broadcast callback confirmation | `swaplock.network_broadcast().broadcast_signed_transfer_with_callback_timeout(...)` | `live.network_broadcast()?.send_signed_transfer_with_callback(signed)?.wait_timeout(...)` | `swaplock_operations_transfer_broadcast_with_callback`, `swaplock_live_transfer_callback_stress`, `swaplock_live_transfer_updates_e2e` (mutates configured testnet and requires `SWAPLOCK_ACTIVE_WIF`) |

Historical spike/design markdowns, the older Swaplock live SDK stack, and unused shared operation-adapter helpers were removed once their useful constraints had been folded into code, crate READMEs, and GSD decisions.
