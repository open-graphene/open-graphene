# Typed `get_objects` Routing Design

## Reader and action

This document is for the next engineer or agent extending Open Graphene generated live object coverage.

After reading it, they should be able to implement the first typed `get_objects` proof without inventing object shapes, adding rich models to `open-graphene-sdk-live`, or weakening fail-closed behavior.

## Why this needs a design

Some Graphene RPC methods return one concrete type in their C++ signature:

```text
get_limit_orders -> vector<limit_order_object>
get_dynamic_global_properties -> dynamic_global_property_object
get_account_balances -> vector<asset>
```

Those methods are straightforward to type: select the RPC in the spec, ensure the returned C++ type reaches the generated type graph, generate the Rust type, and deserialize the RPC result into that type in the chain-specific crate.

`get_objects` is different. Its C++ signature returns `fc::variants`:

```text
get_objects(ids, subscribe) -> variants
```

The result is a vector of JSON variants in the same order as the requested object ids. Each slot is either:

- the object matching the corresponding id;
- `null` when the object does not exist.

The return type does not say whether slot 0 is an account, asset, order, dynamic-global-property object, or something else. The object id tells us that:

```text
1.2.x -> account_object
1.3.x -> asset_object
1.7.x -> limit_order_object
2.1.x -> dynamic_global_property_object
```

That makes typed `get_objects` a routing problem, not just a type-generation problem.

## Current boundary

`open-graphene-transport` should keep returning raw JSON values for Graphene RPC results. It owns JSON-RPC mechanics and Graphene-generic envelopes, not chain-specific object parsing.

`open-graphene-sdk-live` should remain a small Graphene-generic projection layer. It may expose stable SDK primitives such as account ids, asset ids, balances, head blocks, and simple existence checks. It must not become a hand-written object model for accounts, assets, orders, or global properties.

Chain-specific crates should parse rich live payloads with generated chain binding types, then convert to stable public SDK projections when needed.

Generated bindings should remain the authoritative home for wire-level object structs and any generated typed routing helpers.

## Recommended shape

Prefer generated, fail-closed typed helpers over a broad public cache or permissive dynamic enum.

A future generated module may expose two layers:

```rust
pub enum GeneratedObject {
    Account(AccountObject),
    Asset(AssetObject),
    LimitOrder(LimitOrderObject),
    DynamicGlobalProperty(DynamicGlobalPropertyObject),
}

pub enum ObjectLookup<T> {
    Found(T),
    Missing,
}
```

The enum is useful internally for object-id routing, but public chain crate helpers should usually expose narrower functions:

```rust
fn parse_account_object(value: serde_json::Value) -> Result<ObjectLookup<AccountObject>, Error>;
fn parse_asset_object(value: serde_json::Value) -> Result<ObjectLookup<AssetObject>, Error>;
fn parse_dynamic_global_property_object(value: serde_json::Value) -> Result<ObjectLookup<DynamicGlobalPropertyObject>, Error>;
```

The narrow helper knows the expected object type from the requested id. It should fail if the returned object's `id` does not match the requested type or exact requested id.

For a multi-id API, preserve positional semantics:

```text
requested ids: ["1.2.100", "1.3.0", "1.7.999"]
raw result:     [account,   asset,   null]
typed result:   [Found(AccountObject), Found(AssetObject), Missing]
```

Do not collapse, reorder, or silently drop missing objects.

## Fail-closed rules

Typed `get_objects` must reject ambiguous or malformed data.

It should fail when:

- the result is not an array;
- the result length differs from the requested id count;
- a non-null slot cannot be deserialized into the expected generated object type;
- a non-null object's `id` field is missing;
- the object id in the payload differs from the requested id;
- the requested id's object type has no generated struct;
- a known field has an unsupported wire type.

It should return `Missing` only when the corresponding Graphene result slot is `null`.

It should not return `Unknown(serde_json::Value)` for a typed helper. An unknown object means the caller asked for a type the generated bindings cannot model yet. That is a generator coverage gap, not valid typed data.

## Object-id routing

The generator already extracts object id metadata. Typed routing should use that metadata instead of string prefix guessing in hand-written code.

The generated data needed for routing is:

```text
object type name
object space id
object type id
Rust id wrapper type
optional reflected struct reference
```

The router should map object type metadata to a generated Rust object struct only when the spec has a reflected struct for that object.

For example:

```text
limit_order -> LimitOrderObject
dynamic_global_property -> DynamicGlobalPropertyObject
account -> AccountObject, once generated
asset -> AssetObject, once generated
```

If an object type is known only as an id space/type but has no generated struct yet, typed routing must fail closed for that object type.

## Where the code should live

The generated bindings crate should own:

- generated object structs;
- object type metadata constants;
- generated object-id routing helpers;
- generated JSON deserialization rules for object fields.

The chain-specific high-level crate should own:

- calling transport/session helpers;
- choosing which ids to request;
- converting generated objects to public SDK projections;
- user-facing error context;
- confirmation and retry policy.

`open-graphene-transport` should not import generated bindings.

`open-graphene-sdk-live` should not import generated bindings.

## First implementation slice

Do not start by generating every object or every object route.

A small first slice should prove one narrow helper end to end. Swaplock `AssetObject` is now the first such proof; the next slice should either add BitShares parity for the same object or repeat the pattern for `account_object`.

For each new object:

1. choose a currently useful object with reflected fields;
2. ensure the object struct is generated from C++ reflection;
3. generate or hand-write a temporary chain-local typed helper that parses one `get_objects` slot into that generated type;
4. preserve `null` as `Missing`;
5. verify wrong-id and wrong-type payloads fail;
6. add the same local fixture proof for Swaplock and BitShares when the shape is shared;
7. only then generalize routing.

The first helper should be intentionally narrow. Broad `GeneratedObject` routing can come after two or more object types are proven.

## What not to do

Do not infer object schemas from live JSON samples. Live samples are fixtures, not source-of-truth schema.

Do not add `serde_json::Value` fields for known object fields. If a field has a real C++ type that the resolver cannot model yet, fix the resolver or use a deliberate named representation.

Do not add rich account or asset models to `open-graphene-sdk-live`.

Do not hide missing objects by returning an empty vector.

Do not silently accept a payload whose `id` does not match the requested id.

Do not build a generic object cache as part of typed routing. Caching, subscription continuity, and reconnect/resubscribe policy are separate problems.

Do not mix this work with broadcast JSON rendering. Typed live object parsing and broadcast JSON rendering have different wire semantics.

## Reader test

A fresh reader should now know:

- why `get_objects` is harder than `get_limit_orders`;
- why object id routing is required;
- why `null` must be preserved positionally;
- why generated bindings, not `sdk-live`, should own rich object structs;
- why typed helpers should fail closed instead of returning `Unknown(Value)`;
- how to choose the first implementation slice.

If any implementation proposal violates those points, it is likely drifting back toward the manual JSON model this project is trying to retire.
