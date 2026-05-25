# Limit Order Create SDK Design Spike

## Reader and action

This document designs the common SDK interface for a future `limit_order_create` flow.

After reading it, a maintainer should know which input shape to implement, which abstractions to avoid for now, and which policy remains outside the pure adapter layer.

## Problem

`limit_order_create` is the first candidate SDK flow that introduces real market-order semantics. Existing proven flows are simpler:

```text
transfer
account_create
asset_issue
asset_create
limit_order_cancel
```

`limit_order_create` adds:

- two asset legs: `amount_to_sell` and `min_to_receive`,
- an order expiration distinct from transaction expiration,
- `fill_or_kill`,
- potential caller confusion around market direction and price,
- future live-test risk if orders are accidentally placed with bad market parameters.

The common SDK layer must still preserve current boundaries:

- no generated chain types in `open-graphene-sdk-operations`,
- no RPC, signing, fee lookup, broadcast, or confirmation,
- no order book lookup or market-policy decisions,
- no human decimal or symbol-resolution layer,
- operation JSON remains chain-local and fail-closed.

## Design A: raw protocol-shaped input

This input mirrors the protocol fields, but uses existing common SDK input wrappers.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOrderCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub seller: AccountRefInput,
    pub amount_to_sell: AssetAmountInput,
    pub min_to_receive: AssetAmountInput,
    pub expiration: String,
    pub fill_or_kill: bool,
}

impl LimitOrderCreateInput {
    pub fn new(
        header: TransactionHeader,
        fee: FeeInput,
        seller_id: impl Into<String>,
        amount_to_sell: AssetAmountInput,
        min_to_receive: AssetAmountInput,
        expiration: impl Into<String>,
        fill_or_kill: bool,
    ) -> Self;
}
```

Generic builder seam:

```rust
pub trait LimitOrderCreateAdapter {
    type Transaction;
    type Error;

    fn build_limit_order_create_transaction(
        input: LimitOrderCreateInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

pub trait LimitOrderCreateChainTypes: GrapheneOperationBuilderTypes {
    type LimitOrderCreateOperation;

    fn limit_order_create_operation(
        fee: Self::Asset,
        seller: Self::AccountId,
        amount_to_sell: Self::Asset,
        min_to_receive: Self::Asset,
        expiration: String,
        fill_or_kill: bool,
        extensions: Self::FutureExtensions,
    ) -> Self::LimitOrderCreateOperation;

    fn operation_limit_order_create(
        operation: Self::LimitOrderCreateOperation,
    ) -> Self::Operation;
}
```

Usage:

```rust
let input = LimitOrderCreateInput::new(
    header,
    FeeInput::new(200_000, "1.3.0"),
    "1.2.100",
    AssetAmountInput::new(10_000, "1.3.121"),
    AssetAmountInput::new(50_000, "1.3.0"),
    "2026-05-26T12:01:00",
    false,
);
```

### What this keeps internal

- mapping `FeeInput` to generated chain-local `Asset`,
- mapping `AccountRefInput` to generated chain-local `AccountId`,
- mapping both `AssetAmountInput` legs to generated chain-local `Asset`,
- setting empty future extensions,
- wrapping the operation in generated chain-local `Operation`,
- building generated chain-local `Transaction`.

### Trade-offs

Pros:

- Minimal new API surface.
- Closest to the protocol and current adapter style.
- No price math, rounding, or market terminology in the common layer.
- Keeps all generated protocol types chain-local.
- Explicitly separates transaction expiration from order expiration.
- Makes `fill_or_kill` caller-controlled and visible.

Cons:

- Caller must understand `amount_to_sell` vs `min_to_receive`.
- Caller can invert market direction.
- No ergonomic "sell X at price Y" abstraction.
- No decimal/precision help.

## Design B: market and price abstraction

This input tries to express trading intent instead of raw protocol legs.

```rust
pub struct LimitOrderCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub seller: AccountRefInput,
    pub market: MarketInput,
    pub side: OrderSide,
    pub quantity: AssetAmountInput,
    pub price: LimitPriceInput,
    pub expiration: String,
    pub fill_or_kill: bool,
}

pub struct MarketInput {
    pub base_asset_id: String,
    pub quote_asset_id: String,
}

pub enum OrderSide {
    BuyBase,
    SellBase,
}

pub struct LimitPriceInput {
    pub base: AssetAmountInput,
    pub quote: AssetAmountInput,
}
```

Usage:

```rust
let input = LimitOrderCreateInput::sell_base(
    header,
    FeeInput::core(200_000),
    "1.2.100",
    MarketInput::new("1.3.1", "1.3.0"),
    100_000,
    LimitPriceInput::quote_per_base(25_000, "1.3.0", 100_000, "1.3.1"),
    "2026-05-25T12:30:00",
);
```

### What this keeps internal

- deriving `amount_to_sell` and `min_to_receive` from market side, quantity, and price,
- checking asset IDs inside the price match the market pair,
- integer ratio math,
- rounding direction for buy vs sell.

### Trade-offs

Pros:

- More ergonomic for callers.
- Reduces common mistakes around swapping sell/receive legs.
- Better foundation for a future higher-level trading SDK.

Cons:

- Forces common SDK layer to own price math and rounding semantics.
- Adds a larger error surface.
- Requires careful base/quote terminology.
- Starts to become market policy rather than pure transaction construction.
- Still cannot resolve symbols, precision, order book state, or slippage without higher layers.

## Design C: typestate builder over raw legs

This keeps raw protocol legs but uses a builder to force explicit fields.

```rust
let input = LimitOrderCreateInput::builder(header)
    .fee(FeeInput::core(200_000))
    .seller("1.2.100")
    .sell(100_000, "1.3.0")
    .receive_at_least(50_000, "1.3.121")
    .expires_at("2026-05-25T12:30:00")
    .allow_partial_fill()
    .build();
```

The builder would only expose `build()` after fee, seller, sell amount, receive amount, order expiration, and fill policy are provided.

### What this keeps internal

- compile-time completeness checks for common fields,
- mapping `allow_partial_fill()` / `fill_or_kill()` to wire `bool`,
- normal generated-type bridge mapping remains unchanged.

### Trade-offs

Pros:

- Reduces accidental missing-field misuse.
- Makes order expiration and fill policy explicit at call sites.
- Stays raw-leg based and avoids price math.

Cons:

- Much larger type surface than current SDK inputs.
- Typestate marker types are noisy in docs and compiler errors.
- If fields stay public for consistency with existing inputs, the builder is only advisory.
- If fields become private, this breaks the current simple input style.

## Comparison

Design B is the most ergonomic, but it crosses the most dangerous boundary. A shared `MarketInput` and `LimitPriceInput` would make the common operations crate responsible for interpreting price direction and rounding. That is not chain-specific generated protocol code, but it is still trading semantics. The current SDK layer has intentionally avoided that level of policy.

Design C improves call-site safety without price abstraction, but it is too heavy compared with the current SDK input pattern. Existing inputs are simple public structs plus convenience constructors. A typestate builder would be a different API style for one operation, and making it truly safe would require private fields or getters.

Design A is less ergonomic, but it is honest. Graphene's protocol operation is defined in terms of `amount_to_sell` and `min_to_receive`, and the current SDK layer is a pure transaction-construction seam. Keeping those two legs explicit avoids hidden rounding, avoids base/quote confusion inside common code, and preserves a small interface that downstream higher-level SDKs can wrap later.

## Recommendation

Use Design A for the next implementation.

The first `limit_order_create` adapter should be raw protocol-shaped:

```rust
pub struct LimitOrderCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub seller: AccountRefInput,
    pub amount_to_sell: AssetAmountInput,
    pub min_to_receive: AssetAmountInput,
    pub expiration: String,
    pub fill_or_kill: bool,
}
```

Do not introduce `PriceInput`, `MarketInput`, `OrderSide`, or a typestate builder in the first implementation. Those can be layered later on top of the raw input once local FC/JSON proofs and possibly a live create/cancel proof exist.

## Implementation boundary for the future flow

A future `limit_order_create` implementation should add:

- `open-graphene-sdk-operations/src/limit_order_create.rs`,
- `LimitOrderCreateInput`, `LimitOrderCreateAdapter`, `LimitOrderCreateChainTypes`, and `build_limit_order_create_transaction_for`,
- chain-local `limit_order_create.rs` modules for Swaplock and BitShares,
- local raw builder wrappers,
- operation-specific signed transaction JSON renderers,
- tests comparing common-input adapter FC bytes to local builder FC bytes,
- tests for Graphene broadcast JSON shape and wrong-operation rejection.

It should not add:

- RPC lookup,
- fee estimation,
- balance checks,
- market-pair validation,
- order-book inspection,
- human decimal parsing,
- symbol lookup,
- price abstraction,
- signing,
- broadcast,
- confirmation.

## Live proof note

Do not combine the first `limit_order_create` implementation with a live order placement. Local FC/JSON proof should land first. A later live proof can deliberately create a safe tiny order and then cancel it using the already-proven `limit_order_cancel` flow.
