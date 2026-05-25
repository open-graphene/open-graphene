# Manual SDK Adapter Patterns

## Reader and action

This document is for maintainers adding or reviewing operation helpers for Graphene chain binding crates.
After reading it, a maintainer should be able to add a manual SDK adapter without moving RPC, signing, fee lookup, or broadcast orchestration into the wrong layer.

## Current status

Manual SDK adapters have been proven in two generated chain binding crates: Swaplock and BitShares.
Four flows now repeat across both chains:

- transfer
- account create
- asset issue
- asset create

Each flow also has a common-input adapter wrapper using `open-graphene-sdk-operations` input models and adapter traits:

| Flow | Common input | Swaplock wrapper | BitShares wrapper |
| --- | --- | --- | --- |
| transfer | `TransferInput` | `SwaplockTransferAdapter` | `BitSharesTransferAdapter` |
| account create | `AccountCreateInput` | `SwaplockAccountCreateAdapter` | `BitSharesAccountCreateAdapter` |
| asset issue | `AssetIssueInput` | `SwaplockAssetIssueAdapter` | `BitSharesAssetIssueAdapter` |
| asset create | `AssetCreateInput` | `SwaplockAssetCreateAdapter` | `BitSharesAssetCreateAdapter` |

The wrappers are deliberately thin: they pass common SDK input values into generic operation builders from `open-graphene-sdk-operations`. Each chain supplies a generated-type bridge that tells the generic builder how to construct that chain's raw generated operation and transaction types.

The chain-local builder functions and broadcast JSON renderers remain public and operation-specific. The local builder functions preserve the old raw generated-type API shape, but for the current proven flows they delegate to the same common builder path used by the adapter structs.

The pattern is intentionally manual for now. It is a candidate for future generator support, but it should not be generated until the seam remains stable across more chain pressure, more SDK flows, or downstream demand.

## Adapter responsibility

An operation adapter does two narrow things:

1. Build a raw generated `Transaction` containing one generated operation.
2. Render a generated `SignedTransaction` into the JSON object shape expected by Graphene `broadcast_transaction` for that operation.

The transaction-building side now has two entry points:

- A chain-local builder function that accepts operation-specific raw fields or generated helper values.
- A common-input adapter wrapper that accepts the corresponding `open-graphene-sdk-operations` input model.

For the current four proven flows, both entry points converge on the same generic trait-based builder from `open-graphene-sdk-operations` and a chain-local generated-type bridge. The adapter still preserves protocol-level types. It does not introduce a parallel SDK transaction model.

## Common transaction API shape

Each operation adapter follows this local raw-input shape:

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

The build function may internally delegate to the common trait-based builder, but its public role is still to preserve a raw chain-local helper API.

Amounts are raw chain amounts, not human decimal amounts. Decimal conversion belongs outside chain adapters.

Each `signed_transaction_json` function is intentionally operation-specific and fail-closed. A transfer renderer must not render account-create operations, and an account-create renderer must not render transfer operations.

## Common-input wrapper API shape

Each operation adapter may also expose a zero-sized wrapper struct implementing the corresponding `open-graphene-sdk-operations` adapter trait:

```rust
pub struct ChainSomeOperationAdapter;

impl open_graphene_sdk_operations::SomeOperationAdapter for ChainSomeOperationAdapter {
    type Transaction = Transaction;
    type Error = std::convert::Infallible;

    fn build_some_operation_transaction(
        input: open_graphene_sdk_operations::SomeOperationInput,
    ) -> Result<Self::Transaction, Self::Error> {
        Ok(open_graphene_sdk_operations::build_some_operation_transaction_for::<
            ChainOperationBuilderTypes,
        >(input))
    }
}
```

These wrappers exist to prove a coherent cross-chain SDK input seam. They should remain boring pass-through adapters unless a real validation or ergonomics need appears.

They should not perform operation construction themselves. Operation construction should live in the generic builder plus the chain-local generated-type bridge. Broadcast JSON rendering remains operation-specific and chain-local.

## Generated-type bridge API shape

Each chain binding crate provides one zero-sized operation-builder marker for its generated protocol types. That marker implements a shared base trait for common Graphene construction primitives and per-operation traits for the operations it supports.

The base trait supplies common generated-type construction such as assets, empty future extensions, and one-operation transactions. Per-operation traits supply only operation-specific generated values, such as transfer operations, account-create authority/options, asset-issue operations, or UIA asset-create options.

This keeps generated concrete types out of `open-graphene-sdk-operations` while avoiding duplicated transaction assembly logic in every operation adapter.

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

The chain-local raw builder input accepts generated `Authority` and `AccountOptions` values. It also accepts optional generated `AccountCreateOperationExt`, but current FC and broadcast JSON support is fail-closed to the empty extension set only. This avoids inventing a parallel account model while not pretending special authority wire support is complete.

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

## Asset-issue adapter

The asset-issue adapter requires generated bindings for:

```rust
generated::types::Asset
generated::types::Transaction
generated::types::SignedTransaction
generated::types::Signature
generated::operations::AssetIssueOperation
generated::static_variants::Operation
generated::static_variants::FutureExtensions
generated::ids::AccountId
generated::ids::AssetId
```

The operation static variant must contain `AssetIssueOperation` at Graphene wire tag `14`.

The adapter constructs a one-operation transaction with:

```rust
Operation::AssetIssueOperation(Box::new(AssetIssueOperation {
    fee,
    issuer,
    asset_to_issue,
    issue_to_account,
    memo: None,
    extensions: FutureExtensions::VoidT(Box::new(())),
}))
```

Memo support is intentionally absent in the current asset-issue adapter. Issuer permissions and supply mutation are chain-state concerns and stay outside the adapter.

The JSON operation shape is:

```json
[
  14,
  {
    "fee": { "amount": 200000, "asset_id": "1.3.0" },
    "issuer": "1.2.100",
    "asset_to_issue": { "amount": 100000, "asset_id": "1.3.1" },
    "issue_to_account": "1.2.101",
    "memo": null,
    "extensions": []
  }
]
```

## Asset-create adapter

The asset-create adapter requires generated bindings for:

```rust
generated::types::AdditionalAssetOptions
generated::types::Asset
generated::types::AssetOptions
generated::types::Price
generated::types::Transaction
generated::types::SignedTransaction
generated::types::Signature
generated::operations::AssetCreateOperation
generated::static_variants::Operation
generated::static_variants::FutureExtensions
generated::ids::AccountId
generated::ids::AssetId
```

The operation static variant must contain `AssetCreateOperation` at Graphene wire tag `10`.

The first adapter shape is UIA-only. It constructs a one-operation transaction with:

```rust
Operation::AssetCreateOperation(Box::new(AssetCreateOperation {
    fee,
    issuer,
    symbol,
    precision,
    common_options,
    bitasset_opts: None,
    is_prediction_market: false,
    extensions: FutureExtensions::VoidT(Box::new(())),
}))
```

`common_options.core_exchange_rate` is required protocol data. For asset creation, Graphene expects a placeholder price using the future asset instance placeholder in the quote side; the minimal adapter uses `1 CORE / 1 asset(1.3.1)` and the chain overwrites the new asset id when applying the operation.

`common_options.extensions` is an `additional_asset_options` extension set. Empty additional asset options serialize as `varint(0)` in FC and render as `[]` in broadcast JSON. Non-empty additional asset options are rejected until their FC and RPC JSON shape is explicitly implemented and verified.

The adapter rejects bitasset options, prediction markets, non-empty authority or market lists, non-empty operation extensions, and non-empty additional asset options. Issuer permissions, fee lookup, supply policy, and symbol policy remain chain-state concerns outside the adapter.

The JSON operation shape is:

```json
[
  10,
  {
    "fee": { "amount": 500000, "asset_id": "1.3.0" },
    "issuer": "1.2.100",
    "symbol": "OGT12345",
    "precision": 5,
    "common_options": {
      "max_supply": 1000000000000,
      "market_fee_percent": 0,
      "max_market_fee": 0,
      "issuer_permissions": 0,
      "flags": 0,
      "core_exchange_rate": {
        "base": { "amount": 1, "asset_id": "1.3.0" },
        "quote": { "amount": 1, "asset_id": "1.3.1" }
      },
      "whitelist_authorities": [],
      "blacklist_authorities": [],
      "whitelist_markets": [],
      "blacklist_markets": [],
      "description": "open-graphene live asset_create proof",
      "extensions": []
    },
    "bitasset_opts": null,
    "is_prediction_market": false,
    "extensions": []
  }
]
```

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
2. The common-input wrapper produces the same FC bytes as the chain-local builder path.
3. The JSON renderer emits the expected Graphene broadcast JSON shape.
4. The JSON renderer fails closed when given a signed transaction containing a different operation.

For account-create, tests should also prove non-empty generated extension values fail closed until their wire and RPC JSON shapes are explicitly supported. For asset-issue, tests should prove memo JSON fails closed until memo broadcast semantics are explicitly supported. For asset-create, tests should prove bitasset options and non-empty additional asset options fail closed until those asset variants are explicitly supported.

These tests are enough for the adapter layer. Live chain tests belong to chain-specific examples or integration tooling, not to the adapter itself.

## Why this is not generated yet

The repeated Swaplock and BitShares implementations show that generator support is plausible for transfer, account-create, asset-issue, and asset-create.
The common-input wrappers and generic builders now show that the stable seam is not generator-emitted SDK code or shared protocol primitives; it is a shared caller input model plus explicit generated-type bridges in each chain crate.

Generation is still deferred because manual adapters are small, the ergonomics are not fully proven, and generated SDK capabilities would freeze a public API.

Until more pressure exists, keep adapters manual, explicit, and boring.

## When to revisit generation

Revisit generator-emitted operation adapters when at least one of these is true:

- A third chain repeats the same generated contracts.
- More SDK flows repeat the same adapter boundary.
- Manual adapters start drifting in ways tests cannot easily catch.
- A downstream user needs generated capabilities across many chains.

At that point, compare generated adapters against the manual Swaplock and BitShares adapters before replacing them.
