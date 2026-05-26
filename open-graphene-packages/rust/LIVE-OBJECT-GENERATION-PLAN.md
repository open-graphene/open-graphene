# Live Object Generation Plan

## Status

This plan started as part of the SDK/live feature freeze. The first two Swaplock proof targets, `limit_order_object` and `dynamic_global_property_object`, are now complete. The account-balance read path is also complete as a typed value proof through generated `Asset`, not as an `account_balance_object` proof.

Completed `limit_order_object` proof:

- Swaplock spec generation selects `database_api::get_limit_orders` and emits `get_limit_orders -> vector<limit_order_object>`.
- Generated object metadata links `objectTypes.limit_order.structRef` to `limit_order_object`.
- Generated bindings emit `LimitOrderObject` with inherited `id`, seller, price, amount, fee, action, and optional take-profit fields.
- Generated i64 fields accept Graphene live JSON numbers or decimal strings; the first live proof exposed `for_sale` as a decimal string.
- `graphene-chain-swaplock::find_limit_order` consumes generated `LimitOrderObject` instead of `sdk-live::LimitOrderSummary`.
- The private Swaplock testnet `trading_scenario` passed after the migration, opening, finding, canceling, and observing order `1.7.16` gone.

Completed `dynamic_global_property_object` proof:

- Swaplock spec generation selects `database_api::get_dynamic_global_properties` and emits `get_dynamic_global_properties -> dynamic_global_property_object`.
- Generated object metadata links `objectTypes.dynamic_global_property.structRef` to `dynamic_global_property_object`.
- Generated bindings emit `DynamicGlobalPropertyObject` with inherited `id`, head block fields, maintenance fields, budget fields, and participation fields.
- Generated fixed-byte fields accept Graphene live JSON hex strings or byte arrays and enforce exact byte length; the live proof used `head_block_id` as a hex string.
- `graphene-chain-swaplock::head_block` consumes generated `DynamicGlobalPropertyObject` instead of `sdk-live::head_block`.
- The public Swaplock node `head_block_smoke` proof passed, returning head block `694851`.

Completed `get_account_balances -> asset` proof:

- Swaplock and BitShares C++ both declare `database_api::get_account_balances(...) -> vector<asset>`.
- `account_balance_object` exists as an internal indexed chain object, but this RPC does not return it.
- Swaplock and BitShares spec generation now select `database_api::get_account_balances` and emit `get_account_balances -> vector<asset>`.
- Generated bindings already emit `Asset`, including decimal-string or integer JSON deserialization for `amount`.
- Swaplock high-level `account_balance` consumes generated `Asset` chain-locally and preserves the public `i64` return.
- BitShares bindings include an integration fixture proving balance JSON deserializes through generated `Asset`.
- `sdk-live::account_balance` remains a minimal Graphene-generic projection; do not add a rich account-balance model there.

Completed `asset_object`, `account_object`, and `asset_dynamic_data_object` typed `get_objects` proofs for Swaplock, plus local BitShares parity for `asset_object`:

- Swaplock spec generation can now explicitly seed narrow reflected object structs via `object_structs = ["asset_object", "account_object", "asset_dynamic_data_object"]`, without seeding every object type.
- BitShares currently keeps local generated-type parity for `asset_object`; further parity is intentionally deferred while Swaplock is prioritized.
- Generated object metadata links `objectTypes.asset.structRef` to `asset_object` and `objectTypes.account.structRef` to `account_object` when those structs are selected.
- Generated bindings emit `AssetObject` with inherited `id`, symbol, precision, issuer, options, dynamic data id, optional bitasset/buyback/liquidity-pool ids, and creation fields.
- Generated bindings emit `AssetDynamicDataObject` with inherited `id`, current/confidential supply, accumulated fees, accumulated collateral fees, and fee pool fields.
- Generated bindings emit `AccountObject` with inherited `id`, authority/options fields, account statistics refs, special-authority fields, and creation metadata.
- `graphene-chain-swaplock::asset_object`, `graphene-chain-swaplock::account_object`, and `graphene-chain-swaplock::asset_dynamic_data_object` call `get_objects([id])`, preserve `null` as `None`, deserialize non-null slots into generated object structs, and fail-close when the payload id differs from the requested id.
- `active_public_key_for_account` now consumes generated `AccountObject` instead of manually traversing `serde_json::Value`.
- `extensions_type` is generated as an ordered static-variant set (`Vec<FutureExtensions>`) so live account objects with JSON `[]` deserialize correctly and FC empty-set encoding remains `0`.
- BitShares bindings include an integration fixture proving asset-object JSON deserializes through generated `AssetObject`.
- The public Swaplock node `asset_object_smoke` proof passed for `BTS -> 1.3.0`.
- The public Swaplock node `asset_dynamic_data_smoke` proof passed for `BTS -> 1.3.0 -> 2.3.0`.
- The public Swaplock node `account_object_smoke` proof passed for `committee-account -> 1.2.0`.

## Reader and expected action

This document is for the next internal engineer or agent working on Open Graphene code generation. The `limit_order_object` and `dynamic_global_property_object` proofs are done; use their implementations as reference paths for the next generated live object.

The plan remains useful as the template for the next generated live object. Future work should broaden this generated-object pattern; it should not add rich fields to `sdk-live` projections. Balance reads should be treated as typed protocol-value coverage, not as a reason to invent an `account_balance_object` RPC shape.

The expected repeatable pattern is:

1. extend spec generation so the target RPC response reaches a reflected object struct;
2. extend Rust binding generation only when a known wire type needs explicit modeling;
3. prove that the chain crate can deserialize live RPC payloads into generated types;
4. keep `sdk-live` as a minimal projection/orchestration layer rather than a hand-written object binding layer.

## Why `limit_order_object` first

`limit_order_object` is the best first live object proof because:

- its lifecycle is already exercised on the private Swaplock testnet;
- the API method `get_limit_orders` returns a vector of these objects directly;
- current code already has a small hand-written projection, `LimitOrderSummary`, that can be replaced or backed by generated typing;
- the object includes nested protocol values such as `price`, `asset`, optional ids, and static-variant vectors, so it is a meaningful generator test without starting from the largest object in the chain.

Do not start with a broad object system. Prove one object end to end first.

## Source facts

The Swaplock C++ chain defines `limit_order_object` as a chain object derived from Graphene's abstract object base.

The reflected fields are:

```text
expiration
seller
for_sale
sell_price
filled_amount
deferred_fee
deferred_paid_fee
is_settled_debt
on_fill
take_profit_order_id
```

The object also inherits the base object `id` field through `graphene::db::object`.

The relevant reflected C++ shape is:

```cpp
FC_REFLECT_DERIVED_NO_TYPENAME(
    graphene::chain::limit_order_object,
    (graphene::db::object),
    (expiration)(seller)(for_sale)(sell_price)(filled_amount)(deferred_fee)(deferred_paid_fee)
    (is_settled_debt)(on_fill)(take_profit_order_id)
)
```

The live RPC API returns the object directly:

```cpp
vector<limit_order_object> get_limit_orders(...)
```

That means the Rust type should be generated from the reflected object definition, not inferred from current JSON samples.

## Target generated Rust shape

The first generated type should be chain-specific and live-object-specific. A plausible output is:

```rust
pub struct LimitOrderObject {
    pub id: crate::generated::ids::LimitOrderId,
    pub expiration: String,
    pub seller: crate::generated::ids::AccountId,
    pub for_sale: i64,
    pub sell_price: crate::generated::types::Price,
    pub filled_amount: String,
    pub deferred_fee: i64,
    pub deferred_paid_fee: crate::generated::types::Asset,
    pub is_settled_debt: bool,
    pub on_fill: Vec<crate::generated::static_variants::LimitOrderAutoAction>,
    pub take_profit_order_id: Option<crate::generated::ids::LimitOrderId>,
}
```

This sketch must be validated against the existing type resolver. Two fields require care:

- `filled_amount` is `fc::uint128_t`; if the generator does not yet have a safe numeric representation for JSON, use a deliberate string/newtype strategy and document it;
- `on_fill` contains static variants and must reuse the generated `LimitOrderAutoAction` type, not a raw `Value` placeholder.

The generated object module should be separate from protocol operation structs if that keeps the boundary clearer. For example, a future generated namespace could distinguish:

```text
generated::types       protocol value types
generated::operations  protocol operation types
generated::objects     app/database object types
```

Do not put high-level SDK behavior in the bindings crate. The bindings crate should remain generated protocol/live wire types only.

## Spec generation changes

The current spec has object type ids, but not enough object struct typing for live RPC responses. The generator milestone should add a link from object type metadata to the reflected object struct.

### Required spec model

For object types that have reflected fields, emit metadata equivalent to:

```json
{
  "objectType": "limit_order",
  "cppAlias": "limit_order_id_type",
  "objectSpace": 1,
  "typeId": 7,
  "structRef": "limit_order_object"
}
```

And emit a corresponding reflected struct entry:

```json
{
  "name": "limit_order_object",
  "sourceName": "graphene::chain::limit_order_object",
  "base": "graphene::db::object",
  "objectType": "limit_order",
  "fields": [
    { "name": "id", "type": { "kind": "protocol_object_id", "objectType": "limit_order" }, "inherited": true },
    { "name": "expiration", "type": { "kind": "time_point_sec" } },
    { "name": "seller", "type": { "kind": "protocol_object_id", "objectType": "account" } },
    { "name": "for_sale", "type": { "kind": "share_type" } },
    { "name": "sell_price", "type": { "kind": "ref", "name": "price" } },
    { "name": "filled_amount", "type": { "kind": "uint128" } },
    { "name": "deferred_fee", "type": { "kind": "share_type" } },
    { "name": "deferred_paid_fee", "type": { "kind": "ref", "name": "asset" } },
    { "name": "is_settled_debt", "type": { "kind": "bool" } },
    { "name": "on_fill", "type": { "kind": "vector", "inner": { "kind": "static_variant", "name": "limit_order_auto_action" } } },
    { "name": "take_profit_order_id", "type": { "kind": "optional", "inner": { "kind": "protocol_object_id", "objectType": "limit_order" } } }
  ]
}
```

The exact JSON shape should match existing spec conventions. The important requirement is that the spec distinguishes object id metadata from object field metadata and links them explicitly.

### Extraction strategy

The extraction path should use real C++ reflection metadata:

- detect `FC_REFLECT_DERIVED_NO_TYPENAME` for object classes;
- capture the derived class name;
- capture the base reflected class;
- capture reflected field names in order;
- resolve each field's C++ type from the class declaration;
- connect the reflected class to the object type id extracted from `abstract_object<..., space, type>` or the existing object type map;
- include inherited `id` explicitly in the generated Rust object model.

Do not infer fields from live JSON. Live JSON is useful for fixtures, not source-of-truth schema.

### Type resolver requirements

The resolver must support at least the field types present in `limit_order_object`:

- `time_point_sec` -> string JSON/time wrapper strategy already used elsewhere;
- `account_id_type` -> generated `AccountId`;
- `share_type` -> `i64` or existing share type mapping;
- `price` -> generated `Price`;
- `fc::uint128_t` -> deliberate JSON-safe representation;
- `asset` -> generated `Asset`;
- `bool` -> `bool`;
- `vector<limit_order_auto_action>` -> `Vec<LimitOrderAutoAction>`;
- `optional<limit_order_id_type>` -> `Option<LimitOrderId>`.

If any of these cannot be resolved, the generator should fail closed with an explicit unsupported-type report instead of emitting `serde_json::Value`.

## Binding generation changes

The Rust binding generator should emit live object structs only when the spec has verified reflected fields.

### Output module

Prefer a new generated module for app/database objects:

```rust
pub mod objects;
```

The root generated re-exports can expose `LimitOrderObject`, but keeping an `objects` module makes it clear that this is not an operation struct.

### Serde behavior

The generated object should support deserializing Graphene API JSON. It should not imply FC serialization unless the object is actually needed for FC bytes and that serialization has been proven.

For the first proof, JSON deserialization is the important capability.

### No fake placeholders

Do not generate `serde_json::Value` fields for known protocol fields. If `filled_amount` or `on_fill` cannot be modeled correctly, stop and fix the resolver or represent the type deliberately as a named newtype.

Allowed temporary deliberate type examples:

```rust
pub struct Uint128Json(pub String);
```

Not allowed:

```rust
pub filled_amount: serde_json::Value
pub on_fill: Vec<serde_json::Value>
```

### Object id validation

Generated object ids should use generated id wrappers, but current generated id wrappers are string newtypes. SDK primitive refs validate object id space/type more strictly. The chain crate can convert from generated id wrappers to SDK primitive refs at its public seam.

Do not move SDK primitive validation into the generated binding crate unless that becomes a deliberate cross-crate design decision.

## Swaplock integration plan

The first integration should be narrow.

### Current behavior to preserve

Current order helper behavior:

- caller asks for orders by base/quote pair;
- helper scans returned orders for a seller id;
- helper returns the matching order id as a string;
- lifecycle polling remains in the Swaplock high-level crate.

This public behavior should not change in the first generated object proof.

### Proposed flow after generated object exists

```text
transport Value
  -> chain-specific typed adapter
  -> Vec<generated::objects::LimitOrderObject>
  -> Swaplock helper filters by generated seller id
  -> public API returns existing String order id
```

The chain-specific typed adapter can live in the high-level Swaplock crate or a generated helper module. Do not put this adapter in `sdk-live` if it imports Swaplock generated bindings.

### Relationship to `sdk-live`

Do not expand `sdk-live` to a full `LimitOrderObject` model.

There are two acceptable post-proof directions:

1. Keep `sdk-live::limit_orders` as a minimal Graphene-generic projection and add a chain-specific typed path separately.
2. Deprecate or stop expanding `sdk-live::limit_orders` once chain-specific typed adapters become the canonical path for rich objects.

The key rule is that rich live object typing belongs to generated chain bindings or chain-specific adapters, not to manually maintained shared SDK structs.

## Test plan

### Spec generation tests

Add fixture coverage proving that object reflection extraction captures:

- object type id and object space;
- reflected field order;
- inherited object id;
- `limit_order_object` field types;
- `structRef` from object type metadata to object struct metadata.

### Binding generation tests

Add tests proving generated `LimitOrderObject` has the expected Rust field names and types.

Add serde JSON tests for a realistic limit order payload containing:

- `id`;
- `expiration`;
- `seller`;
- `for_sale`;
- `sell_price.base` and `sell_price.quote`;
- `filled_amount`;
- `deferred_fee`;
- `deferred_paid_fee`;
- `is_settled_debt`;
- empty `on_fill`;
- `take_profit_order_id: null`.

### Regression tests against current behavior

The existing Swaplock behavior should continue to pass:

- order lookup returns the same matching order id;
- order gone polling still observes `[null]` after cancel;
- trading scenario still creates, finds, cancels, and observes the order lifecycle.

### Live proof

After the generated object proof is integrated, rerun the private testnet trading scenario.

The runtime proof should show:

- `limit_order_create` broadcast succeeds;
- typed `get_limit_orders` response deserializes into `LimitOrderObject`;
- seller filtering finds the created order;
- `limit_order_cancel` broadcast succeeds;
- order existence check observes cancellation.

Do not add a new live scenario just for this if the existing trading scenario already covers it.

## Implementation sequence

### Step 1: Add object reflection to spec-gen

Implement extraction for reflected chain/app object structs. Start with `FC_REFLECT_DERIVED_NO_TYPENAME` and `limit_order_object`.

Acceptance criteria:

- generated spec links `objectType: limit_order` to `structRef: limit_order_object`;
- generated spec contains all reflected fields and inherited `id`;
- unsupported field types fail closed.

### Step 2: Emit generated object structs

Extend Rust binding generation with an `objects` module and generate `LimitOrderObject`.

Acceptance criteria:

- generated code compiles;
- JSON deserialization tests pass;
- no known field is represented as raw `serde_json::Value`.

### Step 3: Add typed Swaplock adapter

Add a chain-specific adapter that deserializes `get_limit_orders` response into `Vec<LimitOrderObject>`.

Acceptance criteria:

- existing public Swaplock helper signatures remain unchanged;
- order lookup behavior matches current behavior;
- existing local tests pass.

### Step 4: Live proof

Run the existing trading scenario on the private Swaplock testnet.

Acceptance criteria:

- created order is found through generated object deserialization;
- cancellation is observed;
- no secrets are printed;
- build artifacts are removed afterward.

### Step 5: Keep or deprecate the minimal `sdk-live` projection

After the typed chain adapter worked, the project chose to freeze `sdk-live::limit_orders` as a minimal historical/shared projection. Swaplock rich order lookup now uses generated `LimitOrderObject` directly.

Do not add fields to `LimitOrderSummary`. If another chain needs rich order fields, generate and consume that chain's object type instead.

## Non-goals

This milestone should not:

- generate every app/database object at once;
- replace all manual broadcast JSON renderers;
- add new SDK operation flows;
- add an order book or market API;
- move generated bindings into `sdk-live`;
- add a generic object cache;
- invent object schemas from live JSON samples.

## Follow-up milestones

After `limit_order_object`, `dynamic_global_property_object`, `asset_object`, `account_object`, and `asset_dynamic_data_object` are proven on Swaplock, likely next generated live objects are:

1. `asset_bitasset_data_object`, because `AssetObject.bitasset_data_id` points at it for bitassets and richer asset reads should remain generated and chain-local;
2. `operation_history_object`, because confirmation matching is still JSON-heavy and operation-specific;
3. additional object structs only when a real Swaplock RPC/helper needs them.

Before implementing additional typed `get_objects` helpers, read `TYPED-GET-OBJECTS-DESIGN.md`. Those objects likely enter through `get_objects`, which requires positional `null` handling and object-id-based routing rather than a simple concrete RPC return type.

A separate milestone should handle generated Graphene broadcast JSON rendering. That work should not be mixed with live object generation, because it has different semantics and different failure modes.
