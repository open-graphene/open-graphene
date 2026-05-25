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
| `transfer` | `TransferInput` | `TransferAdapter` | `SwaplockTransferAdapter` | `BitSharesTransferAdapter` | adapter FC bytes match chain-local builder |
| `account_create` | `AccountCreateInput` | `AccountCreateAdapter` | `SwaplockAccountCreateAdapter` | `BitSharesAccountCreateAdapter` | adapter FC bytes match chain-local builder |
| `asset_issue` | `AssetIssueInput` | `AssetIssueAdapter` | `SwaplockAssetIssueAdapter` | `BitSharesAssetIssueAdapter` | adapter FC bytes match chain-local builder |
| `asset_create` | `AssetCreateInput` | `AssetCreateAdapter` | `SwaplockAssetCreateAdapter` | `BitSharesAssetCreateAdapter` | adapter FC bytes match chain-local builder |

The proof is deliberately narrow:

- The common operation input models live in `open-graphene-sdk-operations`.
- The operation adapter traits live in `open-graphene-sdk-operations`.
- The trait-based generic transaction builders live in `open-graphene-sdk-operations`.
- Each chain binding crate owns one generated-type bridge marker implementing the shared operation builder traits.
- Each chain binding crate owns its adapter structs and maps common inputs to generated chain `Transaction` values through the generic builders.
- Existing chain-local builder wrappers and operation-specific broadcast JSON renderers remain public and chain-specific.
- Tests compare generated FC bytes from common-input adapters with FC bytes from the existing chain-local builder path.

This proves the SDK seam without extracting shared generated protocol primitives and without adding generator-emitted SDK modules.

## Layering

Recommended implemented layering:

```text
open-graphene-fc
  FC encoding, signatures, WIF, digest helpers

open-graphene-sdk-primitives
  stable SDK value references and validators:
  ObjectId, AccountIdRef, AssetIdRef, AssetAmount, OperationHistoryIdRef

open-graphene-sdk-core
  pure SDK helpers:
  TransactionHeader, amount/header/balance helpers; re-exports SDK primitives for compatibility

open-graphene-sdk-operations
  common operation input models, adapter traits, and generic trait-based builders:
  TransferInput, AccountCreateInput, AssetIssueInput, AssetCreateInput,
  FeeInput, TransferAdapter, AccountCreateAdapter, AssetIssueAdapter, AssetCreateAdapter,
  GrapheneOperationBuilderTypes, TransferChainTypes, AccountCreateChainTypes,
  AssetIssueChainTypes, AssetCreateChainTypes

graphene-chain-*-bindings
  generated protocol types, chain-specific generated-type bridges, adapter structs,
  local builder wrappers, and broadcast JSON renderers:
  Operation, Transaction, operations, static variants, extension-heavy structs,
  ChainOperationBuilderTypes, ChainTransferAdapter, ChainAccountCreateAdapter,
  ChainAssetIssueAdapter, ChainAssetCreateAdapter

higher SDK or examples
  RPC, fee lookup, signing, broadcast, confirmation, CLI/env handling
```

The generator remains responsible for protocol bindings. It should not own SDK orchestration.

## Common SDK input models

Common SDK input models should describe user intent in stable Graphene terms, not in generated chain-specific operation structs.

### Shared primitive-style operation inputs

The first common operation input types live in `open-graphene-sdk-operations` without depending on generated chain bindings. They keep string fields as the storage shape so callers can still build inputs directly from CLI, environment, or RPC JSON data. Each ID-bearing input also offers a checked constructor and validation helper backed by `open-graphene-sdk-primitives` ID references.

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

The ID-bearing inputs have both infallible and checked constructors:

```rust
FeeInput::new(amount, asset_id);
FeeInput::checked(amount, asset_id)?;

AssetAmountInput::new(amount, asset_id);
AssetAmountInput::checked(amount, asset_id)?;

AccountRefInput::new(id);
AccountRefInput::checked(id)?;
```

The checked constructors validate ID kind and preserve the string-backed storage shape.

These inputs may later become stricter, but the current design intentionally separates two kinds of shared values:

- SDK primitives such as `AccountIdRef`, `AssetIdRef`, and `AssetAmount` validate or carry stable Graphene reference values and live in `open-graphene-sdk-primitives`.
- Generated protocol wrappers such as chain-local `AccountId`, `AssetId`, `Asset`, and `Price` remain generated per chain for now.

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

## Chain-specific adapter traits and generic builders

The common SDK layer defines small adapter traits over common inputs. It also provides generic transaction builders parameterized by chain-specific generated-type bridges.

There are two trait layers:

1. `GrapheneOperationBuilderTypes` captures the generated types and constructors shared by the current operation builders: transaction, operation static variant, asset, account id, asset id, future extensions, and one-operation transaction construction.
2. Per-operation chain traits add only operation-specific generated types and constructors: transfer operation, account-create operation and nested authority/options, asset-issue operation, or UIA asset-create operation and nested asset options.

A chain binding crate implements the shared bridge once for its generated types, then implements only the operation-specific traits it supports. The current chains each use one marker type for this bridge.

The resulting flow is:

```text
common input -> generic operation builder -> chain generated-type bridge -> generated Transaction
```

The adapter structs call these generic builders. They do not perform RPC, fee lookup, signing, broadcast, or confirmation.

### Transfer

```rust
pub trait TransferAdapter {
    type Transaction;
    type Error;

    fn build_transfer_transaction(input: TransferInput) -> Result<Self::Transaction, Self::Error>;
}
```

The generic transfer builder maps `TransferInput` into a generated transfer operation through `TransferChainTypes`. Memo support is intentionally absent from the common transfer input and builder.

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

The generic account-create builder maps `AccountCreateInput` into generated `Authority`, `AccountOptions`, and `AccountCreateOperation` values through `AccountCreateChainTypes`.

The common shape is deliberately the simple single-key account-create flow. Advanced authorities, vote lists, special authorities, and non-empty account-create extensions remain chain-local until explicitly designed and verified.

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

The generic asset-issue builder maps `AssetIssueInput` through `AssetIssueChainTypes`. Memo support is intentionally absent from the common input and builder.

The generic asset-create builder maps `AssetCreateInput::uia` through `AssetCreateChainTypes`. It is UIA-only by design: no bitasset options, no prediction market flag, no authority or market lists, and empty additional asset options only. The operation-specific trait names this explicitly as user-issued asset construction so the common API does not imply unsupported asset variants.

## Broadcast JSON boundary

Common SDK inputs solve transaction construction intent. Broadcast JSON still needs chain-specific operation rendering because `Operation` is chain-specific.

`open-graphene-sdk-operations` owns a common helper for the outer signed transaction shell. Chain adapters supply already-rendered operation JSON and signature bytes:

```rust
pub struct SignedTransactionJsonParts<'a> {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: &'a str,
    pub operations: Vec<serde_json::Value>,
    pub signatures: Vec<&'a [u8]>,
}

pub fn signed_transaction_broadcast_json(parts: SignedTransactionJsonParts<'_>) -> serde_json::Value;
```

But each chain still needs a renderer such as:

```rust
fn render_operation_json(operation: &crate::generated::static_variants::Operation) -> Result<Value, Error>;
```

This is expected. Shared inputs reduce duplication; they do not erase operation-specific protocol mapping.

## Where shared protocol primitives fit

The project now distinguishes SDK primitives from generated protocol primitives.

SDK primitives live in `open-graphene-sdk-primitives` and cover stable, chain-agnostic references such as:

```text
ObjectId
AccountIdRef
AssetIdRef
AssetAmount
OperationHistoryIdRef
```

These are input value and validation helpers. They do not replace generated wire types.

Generated protocol primitives may become useful later, after the input boundary is clear. First candidates would be:

```text
AssetId + Asset
Price
Signature
AccountId
TimePointSec
```

If a future shared protocol crate exposes `Asset`, the input models can become stricter:

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

However, shared generated protocol primitives are not required to prove the adapter shape. SDK primitives already cover validated user-facing references; generated protocol types should become shared only when they reduce real adapter friction without hiding fork-specific wire differences.

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

- `open-graphene-sdk-operations` owns common operation input structs, adapter traits, and generic trait-based transaction builders.
- Chain binding crates own generated-type bridge implementations and operation-specific JSON renderers.
- The generator continues to emit protocol types.
- SDK primitives are introduced for proven stable reference and validation helpers.
- Shared generated protocol primitives are introduced only for proven stable wire value types.

This avoids putting operation recipes, policy helpers, and SDK ergonomics into `open-graphene-gen-bindings-rs`.

## Implementation sequence and status

### Phase 1: design-only proof

Status: complete.

This document captured the decision to make common input models the SDK boundary before extracting shared generated protocol primitives or generating SDK adapters.

### Phase 2: operation input models only

Status: superseded by extraction.

The common operation input models and adapter traits now live in a separate crate:

```text
open-graphene-packages/rust/graphene-sdk-operations
```

Package name:

```text
open-graphene-sdk-operations
```

Module layout:

```text
src/common.rs
src/transfer.rs
src/account_create.rs
src/asset_issue.rs
src/asset_create.rs
```

Each operation has its own file. Shared value wrappers such as `FeeInput`, `AssetAmountInput`, `AccountRefInput`, `PublicKeyInput`, and `SingleKeyAuthorityInput` live in `common.rs`.

The crate exports:

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
GrapheneOperationBuilderTypes
TransferChainTypes
AccountCreateChainTypes
AssetIssueChainTypes
AssetCreateChainTypes
build_transfer_transaction_for(...)
build_account_create_transaction_for(...)
build_asset_issue_transaction_for(...)
build_asset_create_transaction_for(...)
```

### Phase 3: implement transfer adapter pair

Status: complete and superseded by generic builder extraction.

Swaplock and BitShares implement `TransferAdapter`. The adapter path now maps `TransferInput` through the generic transfer builder and each chain's generated-type bridge. Tests prove the adapter transactions produce the same FC bytes as the chain-local builder path.

### Phase 4: expand to account_create

Status: complete and superseded by generic builder extraction.

Swaplock and BitShares implement `AccountCreateAdapter`. The adapter path now maps `AccountCreateInput` through the generic account-create builder and each chain's generated-type bridge. This proves the common input model handles nested generated authority/options construction without forcing those structs into shared primitives.

### Phase 5: expand to asset_issue and asset_create

Status: complete and superseded by generic builder extraction.

Swaplock and BitShares implement `AssetIssueAdapter` and `AssetCreateAdapter`. The adapter path now maps both inputs through generic builders and each chain's generated-type bridge. The asset-create builder remains UIA-only and preserves the existing fail-closed behavior for bitassets, prediction markets, non-empty lists, and extension-heavy options.

### Phase 6: add caller-facing input constructors

Status: complete.

`open-graphene-sdk-operations` now exposes ergonomic constructors for the supported common operation input shapes:

```text
TransferInput::new(...)
AccountCreateInput::simple(...)
AssetIssueInput::new(...)
AssetCreateInput::uia(...)
```

These constructors only wrap raw caller fields into existing common input structs. They do not add RPC, fee lookup, chain-state validation, generated protocol types, or new wire capabilities.

### Phase 7: extract generic trait-based builders

Status: complete.

`open-graphene-sdk-operations` now owns generic transaction builders for all four proven flows. These builders are parameterized by chain-specific generated-type bridge traits rather than by generated concrete types.

The shared base trait captures common Graphene operation-building primitives. Per-operation traits add only the generated types and constructors needed for that operation. Swaplock and BitShares each provide one operation-builder marker type that implements the base trait once and the operation-specific traits for the supported flows.

The builder layer changes the implementation path but not the responsibility boundary:

```text
common input -> generic builder -> chain generated-type bridge -> generated Transaction
```

The outer signed-transaction broadcast JSON shell is shared by `open-graphene-sdk-operations::signed_transaction_broadcast_json`. Operation JSON rendering remains chain-local and operation-specific.

### Phase 8: add validation and adapter-call ergonomics

Status: complete.

The common input models remain string-backed, but ID-bearing inputs now have checked constructors that validate IDs through `open-graphene-sdk-primitives`. This gives callers an opt-in validation path without making direct CLI, environment, or RPC JSON construction harder.

The chain adapter structs also expose inherent build methods that delegate to their adapter trait implementations. Normal chain-package callers can call `ChainTransferAdapter::build_transfer_transaction(input)` without importing the corresponding common adapter trait, while generic code can still use the traits directly.

### Phase 9: share the broadcast JSON shell

Status: complete.

`open-graphene-sdk-operations` now exposes `signed_transaction_broadcast_json` and `SignedTransactionJsonParts` for the common Graphene signed-transaction broadcast envelope. Chain-local renderers still produce operation JSON first, so unsupported operations, non-empty extension sets, memo payloads, and unsupported asset variants remain fail-closed in the chain binding crates.

### Phase 10: decide the next seam

Status: next decision.

The current evidence says common input models plus trait-based builders are the right SDK seam. The next decision should be one of:

1. **Builder ergonomics:** reduce any remaining boilerplate in chain generated-type bridge implementations if more operations are added.
2. **Shared generated protocol primitives:** extract `AssetId + Asset` only if adapter ergonomics or validation clearly improve.
3. **Generator support:** keep deferred until a third chain or downstream demand makes manual bridges too costly.

## Success criteria for this architecture

- Common input models and trait-based builders are stable across Swaplock and BitShares for all four current flows.
- Chain-specific adapter implementations remain thin and obvious.
- No RPC/signing/broadcast behavior enters `open-graphene-sdk-operations` adapter traits.
- No generator SDK surface is required to remove most duplicated intent modeling.
- Shared primitives are introduced only where they reduce real adapter friction.

## Open questions

1. Should operation input fields keep storing strings plus validation helpers, or should a future breaking revision store `AccountIdRef` and `AssetIdRef` directly?

## Recommendation

Use common SDK input models plus trait-based builders as the proven SDK seam.

Do not extract shared generated protocol primitives yet. Do not implement generated SDK adapters yet.

The next implementation slice should improve builder ergonomics or revisit shared generated protocol primitives around the existing common inputs and adapter structs, not change the protocol model. Shared generated protocol primitives should be revisited only when a concrete adapter friction point needs them.
