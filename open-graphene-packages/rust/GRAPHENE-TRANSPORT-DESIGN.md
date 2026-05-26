# Graphene Transport Design

## Purpose

`open-graphene-transport` will be the chain-agnostic JSON-RPC transport layer for Graphene-family chains.

It should cover the role that `bitsharesjs-ws` plays in the original BitShares JavaScript SDK: connection/session management, Graphene API discovery, request/response IDs, and WebSocket notice routing. It must not build transactions, sign transactions, choose fees, or know generated chain types.

The first implementation should support blocking WebSocket calls because current live examples use `wss://` nodes. HTTP JSON-RPC should be supported later as a call-only backend behind the same request model.

## Non-goals

The transport layer must not:

- import generated chain bindings;
- build or render operations;
- sign transactions;
- choose fee assets or max-fee policy;
- decide confirmation policy;
- maintain a chain object cache;
- hide reconnect gaps in subscriptions;
- implement endpoint fallback inside the raw connection type.

Those belong in higher layers such as a future live SDK crate, chain-specific operation modules, or an optional connection manager.

## Wire model

Graphene WebSocket and HTTP JSON-RPC calls use the same call payload shape:

```json
{
  "id": 1,
  "method": "call",
  "params": [api_id, "method_name", [args]]
}
```

A normal response is keyed by the same `id`:

```json
{
  "id": 1,
  "result": ...
}
```

An error response is also keyed by `id`:

```json
{
  "id": 1,
  "error": ...
}
```

A WebSocket subscription notice is not a normal response. It arrives as:

```json
{
  "method": "notice",
  "params": [callback_id, payload]
}
```

The transport must parse those as a distinct inbound message class.

## Core types

The first crate should expose small, testable protocol types before adding a full session abstraction.

```rust
pub struct JsonRpcRequest {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

pub enum JsonRpcInbound {
    Response {
        id: u64,
        result: serde_json::Value,
    },
    Error {
        id: u64,
        error: serde_json::Value,
    },
    Notice {
        callback_id: u64,
        payload: serde_json::Value,
    },
}
```

Malformed messages should fail closed with contextual errors. Unknown response IDs should not be silently treated as successful calls.

## Transport traits

The transport abstraction should be split so HTTP can share call semantics without pretending to support notices.

```rust
pub trait JsonRpcTransport {
    fn call_raw(
        &mut self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TransportError>;
}

pub trait NoticeTransport: JsonRpcTransport {
    fn next_notice(&mut self) -> Result<Notice, TransportError>;
}
```

`HttpTransport` can implement `JsonRpcTransport` only. `WebSocketTransport` can implement both.

## WebSocket transport

The blocking WebSocket transport should own exactly one live socket.

```rust
pub struct WebSocketTransport {
    next_id: u64,
    // websocket field omitted here
}

impl WebSocketTransport {
    pub fn connect(url: &str) -> Result<Self, TransportError>;

    pub fn call(
        &mut self,
        api_id: u64,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TransportError>;
}
```

`call` must allocate a request id, send the JSON-RPC request, and wait for the matching response. If notices arrive while waiting for a response, they must not be discarded. The first implementation may buffer them for later `next_notice`; a later async implementation can route them to subscription streams.

On connection close:

- the in-flight call returns `TransportError::ConnectionClosed`;
- buffered subscriptions are considered stale;
- the raw transport does not reconnect by itself.

## Session API discovery

A session is a successfully bootstrapped Graphene connection.

```rust
pub struct ApiIds {
    pub database: u64,
    pub history: Option<u64>,
    pub network_broadcast: Option<u64>,
}

pub struct GrapheneSession<T> {
    transport: T,
    api_ids: ApiIds,
    chain_id: String,
}
```

Bootstrap sequence:

1. connect transport;
2. `call([1, "login", ["", ""]])`;
3. discover `database` by calling API `1`;
4. discover optional `history` by calling API `1`;
5. discover optional `network_broadcast` by calling API `1`;
6. call `database.get_chain_id`;
7. expose typed accessors for the discovered API ids and raw module calls.

The session should not hardcode numeric database/history/broadcast ids.

## API module calls

The session should provide raw module-call helpers equivalent to `GrapheneApi.exec` in `bitsharesjs-ws`:

```rust
impl<T: JsonRpcTransport> GrapheneSession<T> {
    pub fn database_call(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TransportError>;

    pub fn history_call(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TransportError>;

    pub fn network_broadcast_call(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TransportError>;
}
```

Higher layers can wrap these with typed helpers such as `head_block`, `lookup_account_id`, `get_account_history`, or `broadcast_transaction`.

## Subscription semantics

Subscriptions are WebSocket-only.

The raw callback id allocated for a subscription is valid only for the current socket. If the socket closes, the callback id is dead and the subscription is stale.

Rust should use explicit handles rather than JavaScript-style function identity:

```rust
pub struct SubscriptionHandle {
    pub id: SubscriptionId,
    pub callback_id: CallbackId,
    pub intent: SubscriptionIntent,
}

pub enum SubscriptionIntent {
    DatabaseObjects {
        subscribe_to_new: bool,
    },
    Market {
        base: String,
        quote: String,
    },
    BlockApplied,
    PendingTransactions,
    BroadcastCallback,
}
```

A close event must be visible to the caller. The transport must not pretend a subscription survived reconnect.

Recommended close behavior for the first implementation:

```text
connection closed -> pending call fails
connection closed -> subscription handle becomes stale
connection closed -> caller must reconnect and resubscribe
```

A later connection manager may store subscription intents and resubscribe after reconnect, but it must also signal that a data gap may exist.

## Reconnect and fallback

Reconnect is not part of the raw transport.

A later `GrapheneConnectionManager` can own:

- endpoint list;
- retry and fallback policy;
- active subscription intents;
- reconnect status events;
- resubscribe attempts;
- explicit `ResyncRequired` events.

That separation mirrors the original BitShares SDK split between raw websocket transport and endpoint connection management.

## HTTP transport

HTTP JSON-RPC should be added after the WebSocket call/session path works.

HTTP supports:

- `login`;
- API discovery;
- database calls;
- history calls;
- network broadcast calls;
- fee/head/account/asset lookup;
- `broadcast_transaction`.

HTTP does not support live notices in the same way as WebSocket, so it should not implement `NoticeTransport`.

## Implementation order

1. Add `open-graphene-transport` crate with JSON-RPC request/inbound parsing and tests.
2. Add blocking `WebSocketTransport::connect` and `call`.
3. Add `GrapheneSession` bootstrap with login, API discovery, and chain id lookup.
4. Use `GrapheneSession` from current Swaplock helpers without changing operation/signing policy.
5. Add notice parsing and buffering.
6. Add subscription handles and stale-on-close semantics.
7. Add HTTP call-only transport.
8. Add optional connection manager only after raw close/subscription behavior is proven.

## Design pressure to revisit later

Revisit this design when one of these becomes true:

- async runtime support is required by downstream users;
- broadcast callbacks become required for live transaction confirmation;
- object cache support is added;
- multiple endpoints need automatic fallback;
- HTTP nodes become a first-class target;
- subscriptions need automatic resubscribe with resync hooks.
