# Open Graphene Transport

`open-graphene-transport` is the chain-agnostic JSON-RPC transport crate for Graphene-family chains.

It owns the reusable wire/session layer that sits below public chain API crates such as `graphene-chain-swaplock-api`. It does not import generated chain bindings and does not interpret chain-specific protocol objects.

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
- preserve subscription continuity across reconnect.

Those belong in generated binding crates, chain-specific SDK crates, or a future higher-level live SDK/connection manager.

## Core exports

```rust
use open_graphene_transport::{
    ApiIds,
    CallbackId,
    GrapheneSession,
    JsonRpcInbound,
    JsonRpcRequest,
    LiveTransport,
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
    Notice { callback_id: CallbackId, payload: serde_json::Value },
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

A caller that has registered a Graphene callback can block until the next notice with:

```rust
let notice = transport.next_notice()?;
```

For Graphene APIs that take a callback id as their first parameter, the transport also exposes callback-aware calls. `call_with_callback(...)` uses the JSON-RPC request id as the Graphene callback id, prepends that id to the method params, waits for the normal RPC response, then waits for the matching `notice` payload:

```rust
use serde_json::json;

let payload = transport.call_with_callback(
    network_broadcast_api_id,
    "broadcast_transaction_with_callback",
    json!([signed_transaction_json]),
)?;
```

Timeout-sensitive callers can use the bounded variant:

```rust
use std::time::Duration;

let payload = transport.call_with_callback_timeout(
    network_broadcast_api_id,
    "broadcast_transaction_with_callback",
    json!([signed_transaction_json]),
    Duration::from_secs(30),
)?;
```

This is still a blocking, single-session helper rather than a background event loop. Notices for other callback ids are buffered and left for their matching consumer.

This is only message plumbing: the transport does not know which RPC method registered the callback, which object was subscribed, or how to parse the notice payload. Chain-specific API crates own that behavior.

The raw transport does not reconnect by itself.

## Live dispatcher mode

For callers that need multiple in-flight requests and subscription notices on one WebSocket, a bootstrapped `GrapheneSession` can be consumed into a transport-only live dispatcher:

```rust
use open_graphene_transport::{CallbackId, GrapheneSession};
use serde_json::json;
use std::time::Duration;

let session = GrapheneSession::connect("wss://node.example:8090")?;
let database_api_id = session.api_ids().database;
let live = session.into_live_transport()?;

let subscription = live.subscribe_callback(CallbackId::new(42_001))?;
live.call(
    database_api_id,
    "set_subscribe_callback",
    json!([42_001, false]),
)?.wait_timeout(Duration::from_secs(10))?;

let first = live.call(database_api_id, "get_dynamic_global_properties", json!([]))?;
let second = live.call(database_api_id, "get_dynamic_global_properties", json!([]))?;
let first_value = first.wait_timeout(Duration::from_secs(10))?;
let second_value = second.wait_timeout(Duration::from_secs(10))?;
let notice_payload = subscription.next_timeout(Duration::from_secs(10))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Once the dispatcher starts, it owns the WebSocket reader. Do not continue using the consumed blocking session or transport on that connection. The dispatcher is intentionally transport-only: it routes raw JSON-RPC responses and Graphene callback notices by id, but it does not parse chain objects, reconnect, resubscribe, or maintain an object cache.

For a live smoke test against Swaplock:

```bash
cargo run -p open-graphene-transport --example live_dispatcher_smoke
```


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

For example, `graphene-chain-swaplock-api` uses these helpers behind its database, history, operations, and broadcast request builders while keeping Swaplock-specific parsing, signing policy, and confirmation matchers in the chain API crate.
