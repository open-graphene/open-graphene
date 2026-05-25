# Common SDK Input and Chain Adapter Design

## Reader and action

This document is for maintainers deciding how to build a coherent SDK across multiple Graphene chains without turning the generator into an SDK framework.
After reading it, a maintainer should understand why shared primitive types are useful but insufficient, what common SDK input models should look like, and where chain-specific adapter code remains necessary.

This document started as a design spike. It now also records the implementation proof that common SDK inputs and chain-specific adapter traits work across the current two-chain, four-flow matrix.

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

## Implementation proof status

The common input and chain adapter seam is now implemented for the current two-chain, four-flow SDK matrix.

| Flow | Common input | Adapter trait | Swaplock adapter | BitShares adapter | Proof |
| --- | --- | --- | --- | --- | --- |
| `transfer` | `TransferInput` | `TransferAdapter` | `SwaplockTransferAdapter` | `BitSharesTransferAdapter` | adapter FC bytes match manual builder |
| `account_create` | `AccountCreateInput` | `AccountCreateAdapter` | `SwaplockAccountCreateAdapter` | `BitSharesAccountCreateAdapter` | adapter FC bytes match manual builder |
| `asset_issue` | `AssetIssueInput` | `AssetIssueAdapter` | `SwaplockAssetIssueAdapter` | `BitSharesAssetIssueAdapter` | adapter FC bytes match manual builder |
| `asset_create` | `AssetCreateInput` | `AssetCreateAdapter` | `SwaplockAssetCreateAdapter` | `BitSharesAssetCreateAdapter` | adapter FC bytes match manual builder |

The proof is deliberately narrow:

- The common input models live in `open-graphene-sdk-core`.
- The adapter traits live in `open-graphene-sdk-core`.
- Each chain binding crate owns its adapter structs and maps common inputs to generated chain `Transaction` values.
- Existing manual builders and operation-specific broadcast JSON renderers remain the source of chain-specific protocol construction.
- Tests compare generated FC bytes from common-input adapters with FC bytes from the existing manual builders.

This proves the SDK seam without extracting shared protocol primitives and without adding generator-emitted SDK modules.

## Layering

Recommended implemented layering:

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

The common SDK layer defines small adapter traits over common inputs, but does not implement them generically.

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

## Implementation sequence and status

### Phase 1: design-only proof

Status: complete.

This document captured the decision to make common input models the SDK boundary before extracting shared protocol primitives or generating SDK adapters.

### Phase 2: input models only

Status: complete.

`open-graphene-sdk-core` now contains:

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
SingleKeyAuthorityInput
TransferInput
AccountCreateInput
AssetIssueInput
AssetCreateInput
TransferAdapter
AccountCreateAdapter
AssetIssueAdapter
AssetCreateAdapter
```

### Phase 3: implement transfer adapter pair

Status: complete.

Swaplock and BitShares implement `TransferAdapter` by mapping `TransferInput` into their existing generated transfer transaction builders. Tests prove the adapter transactions produce the same FC bytes as the manual builder path.

### Phase 4: expand to account_create

Status: complete.

Swaplock and BitShares implement `AccountCreateAdapter` by mapping `AccountCreateInput` into generated `Authority`, `AccountOptions`, and `AccountCreateOperation` values through the existing manual helpers. This proves the common input model handles nested generated structs without forcing those structs into shared primitives.

### Phase 5: expand to asset_issue and asset_create

Status: complete.

Swaplock and BitShares implement `AssetIssueAdapter` and `AssetCreateAdapter`. The asset-create adapter remains UIA-only and preserves the existing fail-closed behavior for bitassets, prediction markets, non-empty lists, and extension-heavy options.

### Phase 6: decide the next seam

Status: next decision.

The current evidence says common input models are the right SDK seam. The next decision should be one of:

1. **Ergonomics:** add caller-facing constructors/helpers around the existing common inputs and adapter structs.
2. **Validation:** add optional common input validation before mapping to generated chain types.
3. **Broadcast JSON shell reuse:** factor only the outer signed-transaction JSON shell if duplication remains obvious.
4. **Shared primitives:** extract `AssetId + Asset` only if adapter ergonomics or validation clearly improve.
5. **Generator support:** keep deferred until a third chain or downstream demand makes manual wrappers too costly.

## Success criteria for this architecture

- Common input models are stable across Swaplock and BitShares for all four current flows.
- Chain-specific adapter implementations remain thin and obvious.
- No RPC/signing/broadcast behavior enters `open-graphene-sdk-core` adapter traits.
- No generator SDK surface is required to remove most duplicated intent modeling.
- Shared primitives are introduced only where they reduce real adapter friction.

## Open questions

1. Should input IDs remain strings for now, or should a later `open-graphene-protocol` crate introduce typed shared `AccountId` and `AssetId` values?
2. Should common inputs gain validation methods, or should validation remain caller-driven through existing `AccountIdRef` and `AssetIdRef` helpers?
3. Should only the outer signed-transaction broadcast JSON shell be shared, while operation JSON stays chain-specific?
4. Should adapter structs get ergonomic inherent methods so callers do not need to import the adapter traits explicitly?

## Recommendation

Use common SDK input models plus chain-specific adapters as the proven SDK seam.

Do not extract shared protocol primitives yet. Do not implement generated SDK adapters yet.

The next implementation slice should improve ergonomics or validation around the existing common inputs and adapter structs, not change the protocol model. Shared primitives should be revisited only when a concrete adapter friction point needs them.
