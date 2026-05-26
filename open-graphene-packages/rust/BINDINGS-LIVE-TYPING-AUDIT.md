# Binding Usage and Live Typing Audit

## Status

This is a stop-the-line architecture checkpoint. New SDK/live features are intentionally frozen until the binding and live-typing boundaries are made coherent.

The immediate question is why the current live SDK and Swaplock helper layer still parse many RPC payloads through `serde_json::Value` even though `graphene-chain-swaplock-bindings` exists and was created to type Graphene payload serialization and deserialization.

## Short answer

The generated Swaplock bindings are strongest at the protocol transaction layer, and now have two proven live RPC object paths: `limit_order_object` for Swaplock order lookup and `dynamic_global_property_object` for Swaplock head-block reads. Swaplock and BitShares account-balance reads are also proven through the real RPC wire shape, `get_account_balances -> vector<asset>`, using generated `Asset`.

They cover generated operation structs, generated transaction structs, static variants, object id wrappers, `Asset`, `Price`, FC serialization, generated `LimitOrderObject`, and generated `DynamicGlobalPropertyObject`. They do not yet provide complete generated Rust structs for most other database/app objects returned by live RPC calls, such as account objects, asset objects, or rich account-balance object paths.

The current `sdk-live` `serde_json::Value` parsing exists because most generated live response object types do not yet exist. Some of the current hand-written parsing is a justified minimal projection into SDK primitives; richer response modeling would become duplicated manual bindings and should stop until the generator catches up.

## Current generated binding coverage

The generated Swaplock bindings currently provide:

- protocol operation structs such as `TransferOperation`, `AssetIssueOperation`, `AssetCreateOperation`, `LimitOrderCreateOperation`, and `LimitOrderCancelOperation`;
- generated protocol value structs such as `Asset`, `Price`, `Authority`, `Transaction`, and `SignedTransaction`;
- generated static variants such as `Operation`, `OperationResult`, and `FutureExtensions`;
- generated object id wrappers such as `AccountId`, `AssetId`, `LimitOrderId`, and many others;
- generated `LimitOrderObject` for the first typed live RPC object proof;
- generated `DynamicGlobalPropertyObject` for the second typed live RPC object proof;
- generated fixed-byte JSON deserialization for Graphene hex strings or byte arrays with exact length checks;
- generated `Asset` for account-balance reads through the real `get_account_balances -> vector<asset>` RPC shape on Swaplock and BitShares;
- FC serialization for generated ids, assets, operations, transactions, signatures, and supported protocol values;
- serde support for many raw protocol structs, live object structs currently in the spec graph, and static variants.

That is enough to build, mutate, sign, and FC-serialize local transactions with generated types, to consume Swaplock `get_limit_orders` through the generated `LimitOrderObject` path, to consume Swaplock `get_dynamic_global_properties` through the generated `DynamicGlobalPropertyObject` path, and to consume Swaplock/BitShares account balances through generated `Asset` values.

## What generated bindings do not yet cover

The generated bindings do not yet provide complete Rust structs for most database/app objects returned by RPC methods:

- `account_object`;
- `asset_object`;
- `account_balance_object` as a rich chain object, unless a real reflected RPC return path is selected;
- richer `operation_history_object` usage across all live flows;
- typed unions for `get_objects` results.

`limit_order_object` and `dynamic_global_property_object` are the first live object exceptions. Swaplock spec generation selects `get_limit_orders` and `get_dynamic_global_properties`, generated bindings emit `LimitOrderObject` and `DynamicGlobalPropertyObject`, and `graphene-chain-swaplock` uses those generated types for order lookup and head-block reads. Account-balance reads are a separate typed-value proof rather than an object proof: both Swaplock and BitShares select `get_account_balances`, whose real C++ signature returns `vector<asset>`, and generated bindings deserialize those balance entries as `Asset`. This is a proof of direction, not broad live-object completion.

The generated spec contains `objectTypes`, so it knows object id spaces and type ids such as `account -> 1.2.x`, `asset -> 1.3.x`, `limit_order -> 1.7.x`, and `dynamic_global_property -> 2.1.x`. That is not the same as having generated response structs with fields.

The generated RPC method coverage is also incomplete. Current runtime code uses Graphene calls such as:

- `lookup_accounts`;
- `lookup_asset_symbols`;
- `get_account_balances`;
- `get_required_fees`;
- `get_limit_orders`;
- `broadcast_transaction`.

Those are currently hand-wrapped in `open-graphene-transport` and interpreted above transport because the generator/spec layer does not yet emit typed RPC clients. Some response value types are now generated and used chain-locally, such as `get_account_balances -> Asset`, `get_limit_orders -> LimitOrderObject`, and `get_dynamic_global_properties -> DynamicGlobalPropertyObject`.

## Where generated bindings are used correctly today

`graphene-chain-swaplock` correctly uses generated bindings for local protocol construction and signing:

- operation builders produce generated `Transaction` values;
- fee application mutates generated operation variants in a generated `Transaction`;
- signing uses generated transaction FC bytes and generated signatures;
- operation modules render chain-local broadcast JSON from generated `SignedTransaction` values;
- live wrappers convert SDK primitive results back into generated Swaplock protocol types when their public API requires generated types.

This is a coherent use of current bindings: generated protocol types are the local transaction truth.

## Where `serde_json::Value` remains today

`serde_json::Value` remains in three distinct places. They should not be treated the same.

### 1. Transport envelope layer

`open-graphene-transport` returns `serde_json::Value` for Graphene JSON-RPC results. This is acceptable because transport is chain-generic and should not depend on generated chain bindings.

### 2. Minimal shared live projections

`open-graphene-sdk-live` currently parses small live responses into SDK primitive/core types:

- `HeadBlock` as a legacy/shared projection for generic callers;
- `AccountIdRef`;
- `AssetIdRef`;
- `AssetAmount`;
- frozen `LimitOrderSummary { id, seller }` for legacy/shared callers;
- `bool` for `limit_order_exists`.

`account_balance` is intentionally a minimal shared projection over the real Graphene API shape, `database.get_account_balances(account, [asset]) -> vector<asset>`. Swaplock and BitShares generated bindings now both cover that response as generated `Asset`; chain-specific crates should use generated `Asset` when they need chain-local parsing. Do not model this RPC as `account_balance_object` unless a different real RPC method returns that object.

This is acceptable only as minimal Graphene-generic projection. It should not grow into a complete manually modeled object layer. Swaplock rich order lookup has moved to the generated `LimitOrderObject` path, and Swaplock head-block reads have moved to the generated `DynamicGlobalPropertyObject` path in the chain-specific crate.

### 3. Chain-local broadcast JSON renderers

`graphene-chain-swaplock` operation modules manually render signed transactions and operations into Graphene broadcast JSON. This is currently necessary because generated serde is not yet a proven broadcast JSON renderer.

## Why generated serde cannot simply replace broadcast JSON renderers yet

Generated `Serialize`/`Deserialize` is useful, but it is not currently equivalent to Graphene broadcast JSON semantics.

Known mismatches and risks:

- `Signature(pub Vec<u8>)` is a byte vector at the protocol level, while broadcast JSON requires hex string signatures;
- some extension/static-variant values serialize as Graphene tagged variants such as `[0, null]`, while live broadcast payloads for empty extensions use `[]`;
- operation static variants need `[tag, payload]`, but nested extension and signature shapes are context-sensitive;
- FC/preimage bytes, generated serde JSON, and broadcast JSON are related but not identical wire surfaces.

Therefore, replacing chain-local renderers with `serde_json::to_value(&signed_transaction)` would be unsafe until the generator emits an explicit Graphene broadcast JSON rendering capability with tests against live-accepted payloads.

## Boundary rules from this checkpoint

### Transport

`open-graphene-transport` owns:

- JSON-RPC request/response mechanics;
- WebSocket calls;
- API discovery;
- Graphene-generic RPC envelopes.

It must not import generated chain bindings or parse generated chain objects.

### SDK Live

`open-graphene-sdk-live` owns:

- shared read-only live helpers above transport;
- chain-id profile validation;
- small SDK primitive/core projections;
- fail-closed shape checks for minimal responses.

It must not:

- import generated chain bindings;
- sign, broadcast, build transactions, render operations, or apply fees;
- choose fee/retry/confirmation policy;
- grow rich manually modeled app/database object types.

### Chain-specific high-level crates

`graphene-chain-swaplock` owns:

- chain profile constants;
- chain-specific helper APIs;
- generated binding usage;
- operation builders and chain-local broadcast JSON renderers;
- signing and broadcast composition;
- confirmation policy and examples;
- conversion between SDK primitives and generated chain protocol types where needed.

### Generated bindings

`graphene-chain-swaplock-bindings` should stay protocol-only today, but its generator should be extended before we add richer live typing.

## Freeze rules

Until the typing plan is implemented, freeze new SDK/live features.

Allowed work:

- documentation;
- audits;
- tests proving existing assumptions;
- generator design;
- small refactors that reduce inconsistency without expanding public capability.

Disallowed work for now:

- new operation flows;
- new rich `sdk-live` response models;
- new order book or market APIs;
- more live helpers that model app objects manually;
- replacing broadcast renderers with generated serde without a dedicated renderer capability.

## Required generator capabilities

The next generator-oriented milestone should focus on these capabilities before adding more live SDK surface.

### 1. Generated app/database object structs

Generate Rust structs for the app/database objects actually used by live flows first:

- `AccountObject`;
- `AssetObject`;
- `OperationHistoryObject`.

Do not list `AccountBalanceObject` as the next target merely because balance reads exist. The currently used `get_account_balances` RPC returns `vector<asset>`, not `account_balance_object`, and that path is already covered by generated `Asset` for Swaplock and BitShares. `AccountBalanceObject` should only become a generated live-object target if a real reflected RPC return path exposes it.

`LimitOrderObject` and `DynamicGlobalPropertyObject` are already proven for Swaplock. Future object work should follow those patterns: enter the type graph through a real reflected RPC return, keep generated bindings as wire types, and convert to SDK primitives at the chain-specific seam.

### 2. Typed object result union

Generate a typed `get_objects` result representation for known object types, preserving `null` for missing objects.

A future shape could be:

```rust
pub enum ProtocolObject {
    Account(AccountObject),
    Asset(AssetObject),
    LimitOrder(LimitOrderObject),
    DynamicGlobalProperty(DynamicGlobalPropertyObject),
    // ...
}
```

Exact design should follow the extracted spec and real Graphene object spaces. See `TYPED-GET-OBJECTS-DESIGN.md` for the routing, positional `null`, and fail-closed helper rules that should guide the first implementation slice.

### 3. Typed RPC method metadata/client generation

Extend spec generation and binding generation for the RPC methods currently used by runtime code:

- `get_objects`;
- `lookup_accounts`;
- `lookup_asset_symbols`;
- `get_account_balances`;
- `get_required_fees`;
- `get_limit_orders`;
- `get_account_history`;
- `broadcast_transaction`.

The generated surface does not have to replace `open-graphene-transport`, but it should provide typed request/response adapters at the chain-specific layer.

### 4. Explicit Graphene broadcast JSON renderer

Generate a dedicated broadcast JSON renderer rather than relying on generic serde.

It must handle:

- `SignedTransaction` to `broadcast_transaction` JSON;
- operations as `[tag, payload]`;
- signatures as hex strings;
- empty extension fields as the live-accepted Graphene JSON shape;
- fail-closed unsupported extension variants or unsupported operation shapes.

This capability should be tested against current manually accepted Swaplock payloads before replacing manual renderers.

### 5. Conversion bridge to SDK primitives

Once generated app objects exist, chain crates should convert from generated chain objects to stable SDK primitives/projections at their public seam, rather than having `sdk-live` invent rich object models.

## Recommended cleanup path

### Phase 0: Freeze

Stop adding new SDK/live features. Commit this audit and use it as the boundary document.

### Phase 1: Generator proof for live objects

Status: complete for Swaplock `limit_order_object` and `dynamic_global_property_object`.

The limit-order proof covers:

- Swaplock spec generation selects `database_api::get_limit_orders` and emits `get_limit_orders -> vector<limit_order_object>`;
- generated object metadata links `objectTypes.limit_order.structRef` to `limit_order_object`;
- generated bindings emit `LimitOrderObject` with inherited `id`, seller, price, amount, fee, action, and optional take-profit fields;
- generated i64 fields accept Graphene live JSON numbers or decimal strings;
- `graphene-chain-swaplock::find_limit_order` deserializes live `get_limit_orders` results into generated `LimitOrderObject` instead of using `sdk-live::LimitOrderSummary`;
- private testnet `trading_scenario` passed, opening, finding, canceling, and observing order `1.7.16` gone.

The dynamic-global-property proof covers:

- Swaplock spec generation selects `database_api::get_dynamic_global_properties` and emits `get_dynamic_global_properties -> dynamic_global_property_object`;
- generated object metadata links `objectTypes.dynamic_global_property.structRef` to `dynamic_global_property_object`;
- generated bindings emit `DynamicGlobalPropertyObject` with inherited `id`, head block fields, maintenance fields, budget fields, and participation fields;
- generated fixed-byte fields accept Graphene live JSON hex strings or byte arrays and enforce exact byte length;
- `graphene-chain-swaplock::head_block` deserializes live `get_dynamic_global_properties` results into generated `DynamicGlobalPropertyObject` instead of using the shared `sdk-live::head_block` parser;
- read-only `head_block_smoke` live proof passed against the public Swaplock node, returning head block `694851`.

Remaining work after these proofs is broadening the pattern to more objects, not adding fields to `sdk-live` projections.

### Phase 2: Broadcast JSON renderer proof

Pick one operation, likely `transfer`, and generate a dedicated broadcast JSON renderer that exactly matches the existing manual renderer and live-accepted payloads.

Goal:

- prove `generated SignedTransaction -> broadcast JSON` without hand-written operation JSON;
- keep FC serialization unchanged;
- compare generated renderer output against existing manual renderer tests;
- run one live transfer proof.

### Phase 3: Replace manual renderers incrementally

After generated renderer proof works, replace operation renderers one by one:

- transfer;
- account create;
- asset issue;
- asset create;
- limit order create;
- limit order cancel.

### Phase 4: Revisit `sdk-live`

Once chain-specific typed RPC adapters exist, revisit whether `sdk-live` should keep current projections or become a thinner orchestration layer over chain-provided typed adapters.

## Decision checkpoint

The current code should be considered a successful spike/proof, not the final architecture.

Keep current commits because they prove:

- transport/session/API discovery;
- shared read helper seams;
- Swaplock facade migration pattern;
- live fee reads;
- live limit-order lifecycle reads;
- live signing/broadcast/confirmation flows.

But stop adding feature surface until generated binding coverage catches up to the architecture promise.

The central decision is:

> Generated bindings remain the authoritative home for protocol and future typed live payloads. `sdk-live` may expose minimal shared projections only. Rich app/database object typing and broadcast JSON rendering should be generated or chain-local, not manually rebuilt in `sdk-live`.
