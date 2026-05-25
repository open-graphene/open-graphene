# Common SDK Input and Chain Adapter Design

## Reader and action

This document is for maintainers deciding how to build a coherent SDK across multiple Graphene chains without turning the generator into an SDK framework.
After reading it, a maintainer should understand why shared primitive types are useful but insufficient, what common SDK input models should look like, and where chain-specific adapter code remains necessary.

This is a design spike, not an implementation commit.

## Problem

The project now has repeated manual SDK flows across Swaplock and BitShares:

| Flow | Swaplock | BitShares |
| --- | --- | --- |
| `transfer` | live proven | local proven |
| `account_create` | live proven | local proven |
| `asset_issue` | local proven | local proven |
| `asset_create` | live proven | local proven |

The repeated code has two sources:

1. **Nominal primitive duplication**: `swaplock::generated::types::Asset` and `bitshares::generated::types::Asset` are different Rust types even when their wire shape is identical.
2. **Operation construction duplication**: each chain still has its own `Operation`, `Transaction`, operation structs, static variants, and extension semantics.

Moving primitives such as `Asset` into a shared crate helps with the first problem, but it does not solve the second. A true common SDK needs common input models plus chain-specific adapters.

## Core conclusion

Do not start by extracting shared primitives solely because they look reusable.

Start by designing the SDK input boundary:

```text
common SDK input models -> chain-specific adapter -> generated chain Transaction
```

Shared protocol primitives should be introduced only where they make this input boundary simpler and safer.

## Layering

Recommended long-term layering:

```text
open-graphene-fc
  FC encoding, signatures, WIF, digest helpers

open-graphene-protocol
  optional future shared protocol primitives:
  Asset, AssetId, Price, Signature, TimePointSec, stable object-id wrappers

open-graphene-sdk-core
  common SDK input models and pure helpers:
  TransferInput, AccountCreateInput, AssetIssueInput, AssetCreateInput,
  FeeInput, TransactionHeader, amount/header/balance helpers

graphene-chain-*-bindings
  generated protocol types and chain-specific adapters:
  Operation, Transaction, operations, static variants, extension-heavy structs

higher SDK or examples
  RPC, fee lookup, signing, broadcast, confirmation, CLI/env handling
```

The generator remains responsible for protocol bindings. It should not own SDK orchestration.

## Common SDK input models

Common SDK input models should describe user intent in stable Graphene terms, not in generated chain-specific operation structs.

### Shared primitive-style inputs

The first common input types can live in `open-graphene-sdk-core` without depending on generated chain bindings:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeInput {
    pub amount: i64,
    pub asset_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetAmountInput {
    pub amount: i64,
    pub asset_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountRefInput {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKeyInput {
    pub value: String,
}
```

These can later change to use `open-graphene-protocol::{Asset, AssetId, AccountId}` once the shared primitive crate exists. Starting with strings keeps the first design independent from the primitive extraction.

`TransactionHeader` already exists in `open-graphene-sdk-core` and should be reused:

```rust
pub struct TransactionHeader {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
}
```

### Transfer input

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferInput {
    pub header: TransactionHeader,
    pub from: AccountRefInput,
    pub to: AccountRefInput,
    pub amount: AssetAmountInput,
    pub fee: FeeInput,
}
```

No memo in the common input yet. Memo support is a separate proof because it requires key agreement, memo data shape, FC support, and broadcast JSON support.

### Account-create input

Account creation should not expose generated `Authority` or `AccountOptions` in the common input model. Those are protocol structs that may become extension-heavy.

Use a smaller intent model first:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleKeyAuthorityInput {
    pub public_key: PublicKeyInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub registrar: AccountRefInput,
    pub referrer: AccountRefInput,
    pub referrer_percent: u16,
    pub name: String,
    pub owner: SingleKeyAuthorityInput,
    pub active: SingleKeyAuthorityInput,
    pub memo_key: PublicKeyInput,
    pub voting_account: AccountRefInput,
}
```

This covers the currently proven simple-account flow. Advanced authorities, account auths, vote lists, special authorities, and extension fields stay out until explicitly proven.

### Asset-issue input

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIssueInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub issuer: AccountRefInput,
    pub issue_to_account: AccountRefInput,
    pub asset_to_issue: AssetAmountInput,
}
```

No memo yet; same reason as transfer.

### Asset-create input

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub issuer: AccountRefInput,
    pub symbol: String,
    pub precision: u8,
    pub max_supply: i64,
    pub description: String,
}
```

This is UIA-only. BitAssets, prediction markets, market fees, whitelists, blacklists, and additional asset options stay out of the common input until they are separately designed and verified.

## Chain-specific adapter traits

The common SDK layer should define small adapter traits over common inputs, but not implement them generically.

### Transfer

```rust
pub trait TransferAdapter {
    type Transaction;
    type Error;

    fn build_transfer_transaction(input: TransferInput) -> Result<Self::Transaction, Self::Error>;
}
```

A chain binding crate implements this by mapping `TransferInput` to its generated types:

```rust
pub struct SwaplockTransferAdapter;

impl TransferAdapter for SwaplockTransferAdapter {
    type Transaction = crate::generated::types::Transaction;
    type Error = SwaplockSdkError;

    fn build_transfer_transaction(input: TransferInput) -> Result<Self::Transaction, Self::Error> {
        // map shared input to generated::operations::TransferOperation
    }
}
```

### Account create

```rust
pub trait AccountCreateAdapter {
    type Transaction;
    type Error;

    fn build_account_create_transaction(
        input: AccountCreateInput,
    ) -> Result<Self::Transaction, Self::Error>;
}
```

The implementation owns the conversion from `SingleKeyAuthorityInput` into generated `Authority` and from `memo_key`/`voting_account` into generated `AccountOptions`.

### Asset issue and asset create

```rust
pub trait AssetIssueAdapter {
    type Transaction;
    type Error;

    fn build_asset_issue_transaction(input: AssetIssueInput) -> Result<Self::Transaction, Self::Error>;
}

pub trait AssetCreateAdapter {
    type Transaction;
    type Error;

    fn build_asset_create_transaction(input: AssetCreateInput) -> Result<Self::Transaction, Self::Error>;
}
```

These traits do not include RPC, fee lookup, signing, broadcast, or confirmation.

## Broadcast JSON boundary

Common SDK inputs solve transaction construction intent. Broadcast JSON still needs chain-specific operation rendering because `Operation` is chain-specific.

A common helper can render the outer signed transaction shell if a chain adapter supplies operation JSON and signatures:

```rust
pub struct SignedTransactionJsonParts {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    pub operations: Vec<serde_json::Value>,
    pub signatures: Vec<Vec<u8>>,
}

pub fn signed_transaction_json(parts: SignedTransactionJsonParts) -> serde_json::Value;
```

But each chain still needs a renderer such as:

```rust
fn render_operation_json(operation: &crate::generated::static_variants::Operation) -> Result<Value, Error>;
```

This is expected. Shared inputs reduce duplication; they do not erase operation-specific protocol mapping.

## Where shared protocol primitives fit

Shared primitives become useful after the input boundary is clear.

First candidates:

```text
AssetId + Asset
Price
Signature
AccountId
TimePointSec
```

If `open-graphene-protocol::Asset` exists, the input models can become stricter:

```rust
pub struct TransferInput {
    pub header: TransactionHeader,
    pub from: AccountId,
    pub to: AccountId,
    pub amount: Asset,
    pub fee: Asset,
}
```

The chain-specific generated modules can preserve old public paths through re-exports:

```rust
// generated/types.rs
pub use open_graphene_protocol::Asset;

// generated/ids.rs
pub use open_graphene_protocol::AssetId;
```

However, shared primitives are not required to prove the adapter shape. They should follow, not lead, the SDK input model.

## What remains chain-specific

Keep these generated per chain:

```text
Operation
Transaction
SignedTransaction
operation structs
static variants
FutureExtensions
AccountOptions
AssetOptions
Authority
AccountCreateOperationExt
AdditionalAssetOptions
BitassetOptions
MemoData
```

Some of these may later prove shareable, but treating them as shared too early risks hiding fork-specific extension semantics.

## Why this avoids the generator SDK trap

The generator does not need to understand SDK intent models.

Instead:

- `open-graphene-sdk-core` owns common input structs and adapter traits.
- Chain binding crates own implementations that map common input into generated protocol types.
- The generator continues to emit protocol types.
- Shared primitives are introduced only for proven stable value types.

This avoids putting operation recipes, policy helpers, and SDK ergonomics into `open-graphene-gen-bindings-rs`.

## Implementation sequence

### Phase 1: design-only proof

Add this document and decide whether common input models are the intended SDK boundary.

### Phase 2: input models only

Add to `open-graphene-sdk-core`:

```text
input.rs
adapter.rs
```

with:

```text
FeeInput
AssetAmountInput
AccountRefInput
PublicKeyInput
TransferInput
AccountCreateInput
AssetIssueInput
AssetCreateInput
TransferAdapter
AccountCreateAdapter
AssetIssueAdapter
AssetCreateAdapter
```

No generated chain crate changes yet.

### Phase 3: implement one adapter pair

Implement `TransferAdapter` for Swaplock and BitShares using existing manual transfer code.

Success criteria:

- Existing manual transfer tests still pass.
- New shared-input transfer tests pass for both chains.
- No generator changes.

### Phase 4: expand to account_create

Implement `AccountCreateAdapter` for Swaplock and BitShares using `SingleKeyAuthorityInput` and simple account options.

This verifies whether the common input model handles an operation with nested generated structs without forcing those structs to become shared primitives.

### Phase 5: decide shared primitives

Only after Phases 2-4, decide whether `AssetId + Asset` should move into `open-graphene-protocol`.

The measurable reason should be concrete, for example:

- too much duplicated conversion code,
- common JSON helpers want typed `Asset`,
- adapter traits become clearer with shared `Asset` values.

## Success criteria for this architecture

- Common input models are stable across Swaplock and BitShares for all four current flows.
- Chain-specific adapter implementations remain thin and obvious.
- No RPC/signing/broadcast behavior enters `open-graphene-sdk-core` adapter traits.
- No generator SDK surface is required to remove most duplicated intent modeling.
- Shared primitives are introduced only where they reduce real adapter friction.

## Open questions

1. Should adapter traits live in `open-graphene-sdk-core`, or should `sdk-core` only expose input structs while each chain exposes free functions?
2. Should input IDs remain strings at first, or should we create `open-graphene-protocol` before implementing adapter traits?
3. Should broadcast JSON shell helpers live next to input models, or stay in chain-specific adapters until more duplication is measured?
4. Should common inputs include transaction headers directly, or should operation-only inputs be separate from transaction builders?

## Recommendation

Use common SDK input models plus chain-specific adapters as the next architectural step.

Do not extract shared protocol primitives yet. Do not implement generated SDK adapters yet.

The next implementation slice should add only common input models and adapter traits to `open-graphene-sdk-core`, then prove the shape by implementing `TransferAdapter` for Swaplock and BitShares.
