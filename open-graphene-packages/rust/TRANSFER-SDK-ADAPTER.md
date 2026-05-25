# Transfer SDK Adapter Pattern

## Reader and action

This document is for maintainers adding or reviewing a transfer helper for another Graphene chain binding crate.
After reading it, a maintainer should be able to add a manual transfer adapter without moving RPC, signing, fee lookup, or broadcast orchestration into the wrong layer.

## Current status

The transfer adapter pattern has been proven in two generated chain binding crates: Swaplock and BitShares.
Both adapters expose the same public shape and are backed by each chain's generated protocol types.

The pattern is intentionally manual for now. It is a candidate for future generator support, but it should not be generated until the seam remains stable across more chain pressure or more SDK flows.

## Adapter responsibility

A transfer adapter does two narrow things:

1. Build a raw generated `Transaction` containing one `transfer_operation`.
2. Render a generated `SignedTransaction` into the JSON object shape expected by Graphene `broadcast_transaction`.

The adapter preserves protocol-level types. It does not introduce a parallel SDK transaction model.

## Public API shape

Each chain adapter exposes this shape:

```rust
pub struct TransferTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    pub from_id: String,
    pub to_id: String,
    pub asset_id: String,
    pub amount: i64,
    pub fee_amount: i64,
    pub fee_asset_id: String,
}

pub fn build_transfer_transaction(input: TransferTransactionInput) -> Transaction;

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<serde_json::Value, TransferJsonError>;
```

`amount` and `fee_amount` are raw chain amounts, not human decimal amounts. Decimal conversion belongs outside the chain adapter.

## Required generated contract

A chain can implement this adapter when its generated bindings provide:

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

## Transaction construction

The adapter constructs:

```rust
Transaction {
    ref_block_num,
    ref_block_prefix,
    expiration,
    operations: vec![Operation::TransferOperation(Box::new(TransferOperation {
        fee,
        from,
        to,
        amount,
        memo: None,
        extensions: FutureExtensions::VoidT(Box::new(())),
    }))],
    extensions: FutureExtensions::VoidT(Box::new(())),
}
```

Memo support is intentionally absent in the current adapter. A non-empty memo changes protocol requirements and should be added as its own explicit slice.

## Broadcast JSON rendering

The adapter renders only transfer operations. If the signed transaction contains another operation, rendering fails closed with `TransferJsonError::UnsupportedOperation`.

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

Signatures render as lowercase hex strings from the generated `Signature` raw byte payload.

## What stays outside the adapter

The adapter must not do:

- RPC connection management.
- Dynamic global property lookup.
- Transaction header derivation from head block data.
- Account name lookup.
- Asset precision lookup.
- Human decimal amount parsing or formatting.
- Fee lookup.
- Balance checks.
- WIF handling or signing orchestration.
- Public key recovery or signature verification.
- Broadcast submission.
- Post-broadcast confirmation.
- Environment variable or CLI handling.

Those responsibilities belong in examples, CLIs, or higher-level SDK orchestration. Shared pure helpers may live in SDK core if they do not depend on generated chain types.

## Test expectations

Each adapter should have local tests proving:

1. `build_transfer_transaction` produces the same FC bytes as an equivalent hand-built generated transaction.
2. `signed_transaction_json` renders the expected Graphene broadcast JSON shape.

These tests are enough for the adapter layer. Live transfer tests belong to chain-specific examples or integration tooling, not to the adapter itself.

## Why this is not generated yet

The repeated Swaplock and BitShares implementations show that generator support is plausible.
It is still deferred because only one SDK flow has been proven. Generating SDK capabilities too early risks freezing an accidental API before memo handling, other operation families, or a third chain tests the seam.

Until that pressure exists, keep adapters manual, small, and boring.

## When to revisit generation

Revisit generator-emitted transfer adapters when at least one of these is true:

- A third chain repeats the same generated contract.
- A second SDK flow repeats the same adapter boundary.
- Manual adapters start drifting in ways tests cannot easily catch.
- A downstream user needs generated capabilities across many chains.

At that point, compare a generated adapter against the manual Swaplock and BitShares adapters before replacing them.
