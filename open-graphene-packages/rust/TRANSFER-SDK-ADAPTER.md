# Manual SDK Adapter Patterns

## Reader and action

This document is for maintainers adding or reviewing operation helpers for Graphene chain binding crates.
After reading it, a maintainer should be able to add a manual SDK adapter without moving RPC, signing, fee lookup, or broadcast orchestration into the wrong layer.

## Current status

Manual SDK adapters have been proven in two generated chain binding crates: Swaplock and BitShares.
Two flows now repeat across both chains:

- transfer
- account create

The pattern is intentionally manual for now. It is a candidate for future generator support, but it should not be generated until the seam remains stable across more chain pressure, more SDK flows, or downstream demand.

## Adapter responsibility

An operation adapter does two narrow things:

1. Build a raw generated `Transaction` containing one generated operation.
2. Render a generated `SignedTransaction` into the JSON object shape expected by Graphene `broadcast_transaction` for that operation.

The adapter preserves protocol-level types. It does not introduce a parallel SDK transaction model.

## Common transaction API shape

Each operation adapter follows this shape:

```rust
pub struct SomeOperationTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    // operation-specific fields follow
}

pub fn build_some_operation_transaction(input: SomeOperationTransactionInput) -> Transaction;

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<serde_json::Value, SomeOperationJsonError>;
```

Amounts are raw chain amounts, not human decimal amounts. Decimal conversion belongs outside chain adapters.

Each `signed_transaction_json` function is intentionally operation-specific and fail-closed. A transfer renderer must not render account-create operations, and an account-create renderer must not render transfer operations.

## Transfer adapter

The transfer adapter requires generated bindings for:

```rust
generated::types::Asset
generated::types::Transaction
generated::types::SignedTransaction
generated::types::Signature
generated::operations::TransferOperation
generated::static_variants::Operation
generated::static_variants::FutureExtensions
generated::ids::AccountId
generated::ids::AssetId
```

The operation static variant must contain `TransferOperation` at Graphene wire tag `0`.

The adapter constructs a one-operation transaction with:

```rust
Operation::TransferOperation(Box::new(TransferOperation {
    fee,
    from,
    to,
    amount,
    memo: None,
    extensions: FutureExtensions::VoidT(Box::new(())),
}))
```

Memo support is intentionally absent in the current transfer adapter. A non-empty memo changes protocol requirements and should be added as its own explicit slice.

The JSON operation shape is:

```json
[
  0,
  {
    "fee": { "amount": 200000, "asset_id": "1.3.0" },
    "from": "1.2.100",
    "to": "1.2.0",
    "amount": { "amount": 100000, "asset_id": "1.3.0" },
    "memo": null,
    "extensions": []
  }
]
```

## Account-create adapter

The account-create adapter requires generated bindings for:

```rust
generated::types::AccountCreateOperationExt
generated::types::AccountOptions
generated::types::Asset
generated::types::Authority
generated::types::Transaction
generated::types::SignedTransaction
generated::types::Signature
generated::operations::AccountCreateOperation
generated::static_variants::Operation
generated::static_variants::FutureExtensions
generated::ids::AccountId
generated::ids::AssetId
```

The operation static variant must contain `AccountCreateOperation` at Graphene wire tag `5`.

The adapter input accepts generated `Authority` and `AccountOptions` values. It also accepts optional generated `AccountCreateOperationExt`, but current FC and broadcast JSON support is fail-closed to the empty extension set only. This avoids inventing a parallel account model while not pretending special authority wire support is complete.

The adapter provides small convenience constructors for the common simple case:

```rust
single_key_authority(public_key)
account_options(memo_key, voting_account_id)
empty_account_create_extensions()
```

These helpers are not policy engines. They only build simple generated values.

The adapter constructs a one-operation transaction with:

```rust
Operation::AccountCreateOperation(Box::new(AccountCreateOperation {
    fee,
    registrar,
    referrer,
    referrer_percent,
    name,
    owner,
    active,
    options,
    extensions,
}))
```

The JSON renderer hand-renders nested `Authority`, `AccountOptions`, and empty `AccountCreateOperationExt` values into the object/list shapes accepted by Graphene RPC. Non-empty account-create extensions are rejected until their FC and RPC JSON shape is explicitly implemented and verified.

## What stays outside adapters

Adapters must not do:

- RPC connection management.
- Dynamic global property lookup.
- Transaction header derivation from head block data.
- Account name lookup.
- Account name policy validation.
- Asset precision lookup.
- Human decimal amount parsing or formatting.
- Fee lookup.
- Balance checks.
- Public key derivation or validation beyond generated FC serialization.
- WIF handling or signing orchestration.
- Public key recovery or signature verification.
- Broadcast submission.
- Post-broadcast confirmation.
- Environment variable or CLI handling.

Those responsibilities belong in examples, CLIs, or higher-level SDK orchestration. Shared pure helpers may live in SDK core if they do not depend on generated chain types.

## Test expectations

Each adapter should have local tests proving:

1. The build function produces the same FC bytes as an equivalent hand-built generated transaction.
2. The JSON renderer emits the expected Graphene broadcast JSON shape.
3. The JSON renderer fails closed when given a signed transaction containing a different operation.

For account-create, tests should also prove non-empty generated extension values fail closed until their wire and RPC JSON shapes are explicitly supported.

These tests are enough for the adapter layer. Live chain tests belong to chain-specific examples or integration tooling, not to the adapter itself.

## Why this is not generated yet

The repeated Swaplock and BitShares implementations show that generator support is plausible for both transfer and account-create.
It is still deferred because manual adapters are small, the ergonomics are not fully proven, and generated SDK capabilities would freeze a public API.

Until more pressure exists, keep adapters manual, explicit, and boring.

## When to revisit generation

Revisit generator-emitted operation adapters when at least one of these is true:

- A third chain repeats the same generated contracts.
- More SDK flows repeat the same adapter boundary.
- Manual adapters start drifting in ways tests cannot easily catch.
- A downstream user needs generated capabilities across many chains.

At that point, compare generated adapters against the manual Swaplock and BitShares adapters before replacing them.
