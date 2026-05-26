# Open Graphene Transport

`open-graphene-transport` is the chain-agnostic JSON-RPC transport crate for Graphene-family chains.

It owns the reusable wire/session layer that sits below chain SDK crates such as `graphene-chain-swaplock`. It does not import generated chain bindings and does not interpret chain-specific protocol objects.

## What this crate provides

- JSON-RPC request and inbound message primitives for Graphene's `call` envelope.
- A blocking WebSocket transport for `ws://` and `wss://` nodes.
- A bootstrapped `GrapheneSession` that logs in, discovers API ids, and reads `database.get_chain_id`.
- Small Graphene-generic RPC helper functions for common database, history, and broadcast calls.

## What this crate does not provide

This crate intentionally does not:

- build transactions;
- render operation JSON;
- sign transactions;
- choose fee assets or fee policy;
- broadcast as a policy decision;
- parse generated chain objects into generated binding types;
- perform chain-specific validation;
- decide confirmation policy;
- maintain an object cache;
- reconnect automatically;
- preserve subscription continuity across reconnect;
- implement subscriptions yet.

Those belong in generated binding crates, chain-specific SDK crates, or a future higher-level live SDK/connection manager.

## Core exports

```rust
use open_graphene_transport::{
    ApiIds,
    GrapheneSession,
    JsonRpcInbound,
    JsonRpcRequest,
    TransportError,
    WebSocketTransport,
};
```

`JsonRpcRequest::graphene_call(...)` builds Graphene JSON-RPC calls shaped like:

```json
{
  "id": 1,
  "method": "call",
  "params": [0, "get_objects", [["2.1.0"]]]
}
```

`JsonRpcInbound` separates normal responses, error responses, and WebSocket notices:

```rust
pub enum JsonRpcInbound {
    Response { id: u64, result: serde_json::Value },
    Error { id: u64, error: serde_json::Value },
    Notice { callback_id: u64, payload: serde_json::Value },
}
```

Malformed inbound messages fail closed through `TransportError`.

## Blocking WebSocket transport

```rust
use open_graphene_transport::WebSocketTransport;
use serde_json::json;

let mut transport = WebSocketTransport::connect("wss://node.example:8090")?;
let result = transport.call(0, "get_objects", json!([["2.1.0"]]))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`WebSocketTransport::call(...)` allocates a request id, sends one Graphene `call`, and waits for the matching response. Notices received while waiting are buffered and can be inspected later with:

```rust
transport.buffered_notice_count();
transport.next_buffered_notice();
```

The raw transport does not reconnect by itself.

## Graphene session bootstrap

Most callers should start with `GrapheneSession`:

```rust
use open_graphene_transport::GrapheneSession;

let mut session = GrapheneSession::connect("wss://node.example:8090")?;
println!("chain_id={}", session.chain_id());
println!("database_api_id={}", session.api_ids().database);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`GrapheneSession::connect(...)` performs:

1. WebSocket connect;
2. `login("", "")` through API `1`;
3. required `database` API discovery;
4. optional `history` API discovery;
5. optional `network_broadcast` API discovery;
6. `database.get_chain_id`.

The session does not hardcode database/history/broadcast API ids.

For a smoke test against a live node:

```bash
SWAPLOCK_RPC_URL=wss://node02.swaplock.chainpool.online:8090 \
  cargo run --manifest-path open-graphene-packages/rust/graphene-transport/Cargo.toml \
  --example session_smoke
```

The example prints the chain id and discovered API ids. It does not require signing keys.

## Graphene-generic helper functions

The helper functions return raw `serde_json::Value`. They only wrap stable Graphene RPC method envelopes; they do not parse chain objects.

### Database API helpers

```rust
use open_graphene_transport::{
    get_account_balances,
    get_limit_orders,
    get_objects,
    get_required_fees,
    lookup_accounts,
    lookup_asset_symbols,
};
```

Available helpers:

```rust
get_objects(&mut session, ["2.1.0"])?;
get_required_fees(&mut session, operations_json, "1.3.0")?;
lookup_accounts(&mut session, "alice", 1)?;
lookup_asset_symbols(&mut session, ["BTS"])?;
get_account_balances(&mut session, "1.2.100", ["1.3.0"])?;
get_limit_orders(&mut session, "1.3.0", "1.3.1", 100)?;
```

Chain crates remain responsible for interpreting returned account, asset, balance, fee, order, and dynamic global property objects.

### History API helper

```rust
use open_graphene_transport::{get_account_history, AccountHistoryQuery};

let query = AccountHistoryQuery::recent("1.2.100");
let history = get_account_history(&mut session, &query)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`AccountHistoryQuery::recent(account)` preserves the Graphene wire shape currently used by the live Swaplock helpers:

```json
["1.2.100", "1.11.0", 20, "1.11.0"]
```

Chain crates remain responsible for matching operation-history entries to chain-specific operation payloads.

### Network broadcast API helper

```rust
use open_graphene_transport::broadcast_transaction;

broadcast_transaction(&mut session, signed_transaction_json)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

This helper only calls `network_broadcast.broadcast_transaction([transaction_json])`. It does not sign, choose fees, wait for confirmation, or decide whether broadcasting is appropriate.

## Close and reconnect semantics

If the WebSocket closes:

- the pending call fails with `TransportError::ConnectionClosed` or a WebSocket error;
- buffered notices are connection-local;
- API ids discovered by the session are stale with respect to any new socket;
- subscription handles, once added, must be treated as stale;
- the caller must reconnect and resubscribe explicitly.

The raw transport must not pretend subscription continuity survived a reconnect. A future connection manager may own reconnect/resubscribe policy, but it must surface that a data gap may exist.

## Relationship to chain SDK crates

A chain SDK crate should use this crate for transport/session/RPC envelopes, then keep chain-local behavior in the chain crate:

- generated type parsing;
- operation JSON rendering;
- transaction construction;
- signing;
- fee application policy;
- broadcast orchestration;
- confirmation matchers;
- object-specific validation and error messages.

For example, `graphene-chain-swaplock` wraps these helpers behind its existing `GrapheneRpc` facade while retaining Swaplock-specific parsing and live-operation helpers.
