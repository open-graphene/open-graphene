# Open Graphene SDK Live

`open-graphene-sdk-live` is the shared read-only live SDK layer for Graphene-family chains.

It sits above `open-graphene-transport` and below chain-specific crates such as `graphene-chain-swaplock`. It owns reusable live chain-state reads that can be expressed with Graphene-generic RPC envelopes and SDK primitive/core types.

## What this crate provides

Core types:

```rust
use open_graphene_sdk_live::{
    GrapheneChainProfile,
    GrapheneLiveClient,
    LiveSdkError,
};
```

Current read helpers:

```rust
use open_graphene_sdk_live::{
    head_block,
    lookup_account_id,
    lookup_account_id_optional,
    lookup_asset_id,
    lookup_asset_id_optional,
    account_balance,
};
```

The helpers use `open_graphene_transport::GrapheneSession` and return shared SDK types where possible:

- `head_block(...) -> open_graphene_sdk_core::HeadBlock`;
- `lookup_account_id(...) -> open_graphene_sdk_primitives::AccountIdRef`;
- `lookup_asset_id(...) -> open_graphene_sdk_primitives::AssetIdRef`;
- `account_balance(...) -> open_graphene_sdk_primitives::AssetAmount`.

## Chain profiles

A chain crate defines a small profile so the live client can validate chain identity and expose common constants without importing generated bindings.

```rust
use open_graphene_sdk_live::GrapheneChainProfile;

pub struct SwaplockProfile;

impl GrapheneChainProfile for SwaplockProfile {
    const CORE_ASSET_ID: &'static str = "1.3.0";
    const PUBLIC_KEY_PREFIX: &'static str = "BTS";

    fn expected_chain_id() -> Option<&'static str> {
        Some("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
    }
}
```

If `expected_chain_id()` returns `Some(...)`, `GrapheneLiveClient::connect(...)` and `GrapheneLiveClient::from_session(...)` fail closed when the node reports a different chain id.

## Client usage

```rust
use open_graphene_sdk_live::GrapheneLiveClient;

let mut client = GrapheneLiveClient::<SwaplockProfile>::connect(url)?;

let head = client.head_block()?;
let account_id = client.lookup_account_id("swaplock")?;
let asset_id = client.lookup_asset_id("BTS")?;
let balance = client.account_balance(&account_id, &asset_id)?;

println!("chain_id={}", client.chain_id());
println!("head={} {}", head.number, head.id);
println!("account_id={account_id}");
println!("asset_id={asset_id}");
println!("balance={} {}", balance.amount, balance.asset_id);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`session()` and `session_mut()` are intentionally exposed as escape hatches for chain crates that need lower-level transport calls while migrating incrementally.

## Facade integration pattern

Chain crates that already wrap `GrapheneSession` can use the free helpers without giving ownership of the session to `GrapheneLiveClient`.

```rust
pub fn head_block(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
) -> Result<open_graphene_sdk_core::HeadBlock, Box<dyn std::error::Error>> {
    let expected = rpc.database_api_id()?;
    if expected != database_api_id {
        return Err(format!(
            "database API id mismatch: expected {expected}, got {database_api_id}"
        )
        .into());
    }

    Ok(open_graphene_sdk_live::head_block(rpc.session_mut())?)
}
```

This is the current Swaplock migration pattern: preserve the public chain-crate API and delegate only the Graphene-generic read/parsing work to `sdk-live`.

## Lookup semantics

### Account lookup

`lookup_account_id_optional(session, name)` calls Graphene `database.lookup_accounts(name, 1)` and expects a response shaped like:

```json
[["account-name", "1.2.x"]]
```

It returns `Ok(None)` when the response is empty or when the returned name is not an exact match. It fail-closes on malformed pairs and validates returned ids as `AccountIdRef`.

`lookup_account_id(session, name)` wraps the optional helper and returns `LiveSdkError::AccountNotFound` when no exact match exists.

### Asset lookup

`lookup_asset_id_optional(session, symbol)` calls Graphene `database.lookup_asset_symbols([symbol])` and expects a response shaped like:

```json
[{"id": "1.3.x", "symbol": "SYMBOL"}]
```

It returns `Ok(None)` for an empty response, a first `null` entry, or a non-exact symbol. It fail-closes on malformed objects and validates returned ids as `AssetIdRef`.

`lookup_asset_id(session, symbol)` wraps the optional helper and returns `LiveSdkError::AssetNotFound` when no exact match exists.

## Balance semantics

`account_balance(session, account_id, asset_id)` calls Graphene `database.get_account_balances(account_id, [asset_id])` and expects a response shaped like:

```json
[{"amount": "12345", "asset_id": "1.3.x"}]
```

The `amount` field may be a JSON integer or a decimal string. The helper fail-closes when the response has no first balance object, the `asset_id` is missing or different from the requested `AssetIdRef`, or the amount is malformed. It returns `open_graphene_sdk_primitives::AssetAmount`.

## Live smoke example

The crate includes a read-only smoke example that exercises the current live client API without WIFs, signing, or broadcast.

```bash
GRAPHENE_RPC_URL=wss://node02.swaplock.chainpool.online:8090 \
GRAPHENE_ACCOUNT_NAME=swaplock \
GRAPHENE_ASSET_SYMBOL=BTS \
cargo run --manifest-path open-graphene-packages/rust/graphene-sdk-live/Cargo.toml \
  --example session_smoke
```

The example connects, prints the chain id and head block, resolves the account and asset ids, and reads the account balance for that asset.

## What this crate does not provide

This crate intentionally does not:

- import generated chain binding crates;
- parse chain-specific generated object types;
- build transactions;
- render operation JSON;
- sign transactions;
- load WIFs or other secrets;
- choose fee assets or max-fee policy;
- broadcast transactions;
- wait for confirmations;
- decide retry policy;
- maintain an object cache;
- reconnect automatically;
- preserve subscription continuity across reconnect.

Those belong in generated binding crates, chain-specific SDK crates, callers, or a future connection manager with explicit policy.

## Current implementation status

Implemented today:

- `GrapheneChainProfile`;
- `GrapheneLiveClient<P>`;
- `head_block`;
- `lookup_account_id` / `lookup_account_id_optional`;
- `lookup_asset_id` / `lookup_asset_id_optional`;
- `account_balance`;
- Swaplock integration for head-block, account lookup, asset lookup, and account balance reads while preserving existing Swaplock helper signatures.

Likely next candidates are fee reads and order reads, but those have more response-shape nuance and should be moved in small slices.

## Planned fee helper boundary

The next intended `sdk-live` fee helper should stop at the Graphene-generic fee read boundary:

```rust
pub fn required_fee_for_operation_json(
    session: &mut GrapheneSession,
    operation_json: serde_json::Value,
    fee_asset_id: &AssetIdRef,
) -> Result<AssetAmount, LiveSdkError>;
```

It should call `database.get_required_fees([operation_json], fee_asset_id)` and parse the first returned fee object into `AssetAmount` with the same amount and asset-id validation used by balance parsing.

It should not render operations, inspect generated transaction types, choose fee policy, apply fees, sign, broadcast, or return generated chain binding types. Chain crates such as `graphene-chain-swaplock` should keep their current transaction/renderer wrapper and convert the returned `AssetAmount` into the generated chain `Asset` type needed by existing callers.
