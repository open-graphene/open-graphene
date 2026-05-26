# Open Graphene SDK Live Design

## Purpose

`open-graphene-sdk-live` is the proposed shared live SDK layer above `open-graphene-transport` and below chain-specific high-level crates such as `graphene-chain-swaplock`.

It should reduce duplicated live-chain orchestration without importing generated chain bindings or hiding policy decisions that callers must own.

The current transport crate owns:

- JSON-RPC request/response/notice primitives;
- blocking WebSocket calls;
- `GrapheneSession` API discovery and `get_chain_id`;
- Graphene-generic RPC envelopes for database, history, and broadcast calls.

The current Swaplock high-level crate owns:

- generated binding types;
- operation builders and operation JSON renderers;
- local transaction mutation such as fee application;
- WIF signing and public-key verification;
- broadcast composition;
- confirmation matching;
- live examples and environment policy.

`sdk-live` should sit between those boundaries.

## Requirements

The live SDK layer should:

- work across Graphene-family chains;
- depend on `open-graphene-transport` and SDK primitive/core crates;
- not depend on generated chain binding crates;
- not render generated operations;
- not sign transactions;
- not decide fee limits, WIF loading, confirmation requirements, or retry policy;
- expose reusable chain-state reads and common live value types;
- keep typed generated responses in chain crates until the generator emits common object types;
- make chain profiles explicit enough to validate chain identity and core constants;
- allow Swaplock to migrate incrementally without breaking its current helper API.

## Design 1: Thin read-helper crate

This design makes `open-graphene-sdk-live` a small collection of chain-agnostic read helpers over `GrapheneSession`.

### Interface sketch

```rust
pub struct LiveChainProfile {
    pub chain_id: Option<&'static str>,
    pub core_asset_id: &'static str,
    pub public_key_prefix: &'static str,
}

pub fn validate_chain_id(
    session: &GrapheneSession,
    profile: &LiveChainProfile,
) -> Result<(), LiveSdkError>;

pub fn head_block(
    session: &mut GrapheneSession,
) -> Result<HeadBlock, LiveSdkError>;

pub fn lookup_account_id(
    session: &mut GrapheneSession,
    account_name: &str,
) -> Result<AccountIdRef, LiveSdkError>;

pub fn lookup_asset_id(
    session: &mut GrapheneSession,
    symbol: &str,
) -> Result<AssetIdRef, LiveSdkError>;

pub fn account_balance(
    session: &mut GrapheneSession,
    account_id: &AccountIdRef,
    asset_id: &AssetIdRef,
) -> Result<i64, LiveSdkError>;
```

### Usage example

```rust
let profile = LiveChainProfile {
    chain_id: Some(SWAPLOCK_CHAIN_ID),
    core_asset_id: "1.3.0",
    public_key_prefix: "BTS",
};

let mut session = GrapheneSession::connect(url)?;
validate_chain_id(&session, &profile)?;

let head = head_block(&mut session)?;
let sender = lookup_account_id(&mut session, "swaplock")?;
let core_balance = account_balance(&mut session, &sender, &AssetIdRef::new("1.3.0")?)?;
```

### What it hides

- Graphene RPC envelope calls;
- common JSON shape checks for simple responses;
- conversion into SDK primitive/core types such as `HeadBlock`, `AccountIdRef`, and `AssetIdRef`.

### Trade-offs

This is easy to adopt and hard to misuse, but it does not create a strong live client abstraction. Callers still pass `GrapheneSession` everywhere and compose flows manually.

## Design 2: Live client with explicit profile

This design makes `open-graphene-sdk-live` expose one main `GrapheneLiveClient<P>` object parameterized by a profile trait.

### Interface sketch

```rust
pub trait GrapheneChainProfile {
    const CORE_ASSET_ID: &'static str;
    const PUBLIC_KEY_PREFIX: &'static str;

    fn expected_chain_id() -> Option<&'static str> {
        None
    }
}

pub struct GrapheneLiveClient<P> {
    session: GrapheneSession,
    _profile: PhantomData<P>,
}

impl<P: GrapheneChainProfile> GrapheneLiveClient<P> {
    pub fn connect(url: &str) -> Result<Self, LiveSdkError>;
    pub fn session(&self) -> &GrapheneSession;
    pub fn session_mut(&mut self) -> &mut GrapheneSession;
    pub fn chain_id(&self) -> &str;

    pub fn head_block(&mut self) -> Result<HeadBlock, LiveSdkError>;
    pub fn lookup_account_id(&mut self, account_name: &str) -> Result<AccountIdRef, LiveSdkError>;
    pub fn lookup_asset_id(&mut self, symbol: &str) -> Result<AssetIdRef, LiveSdkError>;
    pub fn account_balance(
        &mut self,
        account_id: &AccountIdRef,
        asset_id: &AssetIdRef,
    ) -> Result<i64, LiveSdkError>;
    pub fn required_fee_json(
        &mut self,
        operation_json: serde_json::Value,
        fee_asset_id: &AssetIdRef,
    ) -> Result<serde_json::Value, LiveSdkError>;
}
```

A chain crate defines its profile:

```rust
pub struct SwaplockProfile;

impl GrapheneChainProfile for SwaplockProfile {
    const CORE_ASSET_ID: &'static str = "1.3.0";
    const PUBLIC_KEY_PREFIX: &'static str = "BTS";

    fn expected_chain_id() -> Option<&'static str> {
        Some("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
    }
}
```

### Usage example

```rust
let mut client = GrapheneLiveClient::<SwaplockProfile>::connect(url)?;
let header = client.head_block()?;
let sender = client.lookup_account_id("swaplock")?;
let balance = client.account_balance(&sender, &AssetIdRef::new(SwaplockProfile::CORE_ASSET_ID)?)?;
```

### What it hides

- session bootstrap and optional chain-id validation;
- repeated database API id plumbing;
- common read helper composition;
- conversion into SDK primitives/core types.

### Trade-offs

This creates a stronger seam and a simple default caller experience. The risk is that the client object can grow into a policy sink if we add signing, broadcasting decisions, retries, or confirmation policy too early.

## Design 3: Ports and adapters

This design exposes traits for chain-state reads and lets each chain crate provide an adapter implementation.

### Interface sketch

```rust
pub trait ChainStateReader {
    type Error;

    fn head_block(&mut self) -> Result<HeadBlock, Self::Error>;
    fn lookup_account_id(&mut self, name: &str) -> Result<AccountIdRef, Self::Error>;
    fn lookup_asset_id(&mut self, symbol: &str) -> Result<AssetIdRef, Self::Error>;
    fn account_balance(
        &mut self,
        account_id: &AccountIdRef,
        asset_id: &AssetIdRef,
    ) -> Result<i64, Self::Error>;
}

pub struct TransportChainStateReader<P> {
    session: GrapheneSession,
    _profile: PhantomData<P>,
}
```

Operation builders could then be generic over `ChainStateReader` instead of concrete transport/session types.

### Usage example

```rust
fn prepare_transfer<R: ChainStateReader>(reader: &mut R, sender_name: &str) -> Result<(), R::Error> {
    let sender = reader.lookup_account_id(sender_name)?;
    let head = reader.head_block()?;
    Ok(())
}
```

### What it hides

- concrete transport implementation;
- live vs test/mock chain-state source;
- read orchestration behind trait ports.

### Trade-offs

This is flexible and testable, but it is a heavier abstraction than current callers need. It also risks producing trait plumbing before the live SDK has enough real consumers beyond Swaplock.

## Comparison

The thin read-helper crate is the smallest interface. It is easy to add and easy to reason about, but it keeps call sites procedural and does not create a strong live SDK identity. It is more like a utility module than a layer.

The live client design is deeper. A caller connects once, gets profile validation once, and then calls chain-state helpers through a single object. It hides API id plumbing and gives future live SDK work a natural home. The main misuse risk is scope creep: if signing, fee policy, retries, and confirmation are added to the client, it becomes a policy-heavy black box.

The ports-and-adapters design is the most flexible, especially for tests and future non-WebSocket backends, but it is too abstract for the current stage. The project has one high-level live chain crate today, and most pressure is still about moving proven helper seams, not creating a broad trait hierarchy.

## Recommendation

Use Design 2 as the public shape, constrained by Design 1's narrow scope.

That means create `open-graphene-sdk-live` with:

- `GrapheneChainProfile`;
- `GrapheneLiveClient<P>`;
- read-only live helpers for common chain state;
- explicit escape hatch to `session()` / `session_mut()`;
- no signing;
- no transaction building;
- no operation JSON rendering;
- no broadcast orchestration;
- no confirmation policy;
- no reconnect manager.

This gives callers a real live SDK object without hiding sensitive policy. Chain crates can define small profile structs and migrate their current `GrapheneRpc` facade incrementally.

## First implementation slice

The first code slice should be intentionally small:

1. create crate `graphene-sdk-live` with package name `open-graphene-sdk-live`;
2. depend on:
   - `open-graphene-transport`;
   - `open-graphene-sdk-core`;
   - `open-graphene-sdk-primitives`;
   - `serde_json`;
   - `thiserror`;
3. define:
   - `GrapheneChainProfile`;
   - `GrapheneLiveClient<P>`;
   - `LiveSdkError`;
4. implement:
   - `connect`;
   - `chain_id`;
   - `session` / `session_mut`;
   - profile chain-id validation;
   - `head_block` using `get_objects(["2.1.0"])` and `open_graphene_sdk_core::HeadBlock`;
5. add unit tests for chain-id validation and dynamic-global-property parsing;
6. do not migrate Swaplock call sites yet.

A later slice can migrate Swaplock's `head_block`, then account/asset/balance/fee/order helpers one at a time.

## Typed generated bindings note

`open-graphene-sdk-live` should not import generated chain bindings. It should return SDK primitive/core types or raw JSON where generated protocol object types are unavailable.

Chain crates should deserialize to generated binding types where those types exist and match the RPC wire response. For example, `OperationHistoryObject` is a good candidate for a later Swaplock `history_api` typed-response refactor. Database object responses such as account objects, asset objects, dynamic global properties, balances, and limit orders require generated object types before they can be fully typed with generated bindings.

## Required fee boundary checkpoint

Required-fee reads are allowed in `open-graphene-sdk-live` only at the Graphene-generic RPC and fee-amount boundary.

A future helper should take already-rendered operation JSON and a requested fee asset id:

```rust
pub fn required_fee_for_operation_json(
    session: &mut GrapheneSession,
    operation_json: serde_json::Value,
    fee_asset_id: &AssetIdRef,
) -> Result<AssetAmount, LiveSdkError>;
```

The helper may:

- call Graphene `database.get_required_fees([operation_json], fee_asset_id)`;
- parse the first returned fee object as `AssetAmount`;
- accept fee `amount` as either a JSON integer or decimal string;
- validate the returned `asset_id` as `AssetIdRef` and require it to match the requested fee asset;
- fail closed on missing fee entries, missing fields, malformed amounts, or asset mismatches.

The helper must not:

- build transactions;
- inspect generated operation variants;
- render operation JSON;
- choose the operation to fee from a transaction;
- choose fee asset policy;
- apply the fee to a generated transaction;
- return generated binding types such as a chain-specific `Asset`.

For Swaplock, the existing `database_api::required_fee` wrapper should keep building the unsigned transaction mirror, calling the chain-local renderer, extracting the first operation JSON, and converting the returned `AssetAmount` back into the generated Swaplock `Asset` expected by current callers. This preserves the chain crate API while moving only the generic RPC/fee parsing into `sdk-live`.
