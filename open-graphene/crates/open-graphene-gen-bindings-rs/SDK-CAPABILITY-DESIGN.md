# Generated SDK Adapter Capability Design

## Reader and action

This document is for maintainers deciding how `open-graphene-gen-bindings-rs` should eventually generate SDK adapter modules.
After reading it, a maintainer should know what interface shape is recommended, what is intentionally out of scope, and what must be proven before replacing the current manual adapters.

This is a design spike, not an implementation contract. The current committed implementation remains manual adapters.

## Context

Manual SDK adapters now exist across two generated chain binding crates:

| Flow | Swaplock | BitShares |
| --- | --- | --- |
| `transfer` | live proven | local proven |
| `account_create` | live proven | local proven |
| `asset_issue` | local proven | local proven |
| `asset_create` | live proven | local proven |

The repeated shape makes generated SDK adapter support plausible, but generated SDK modules would be public API. The generator should not freeze the current manual surface accidentally.

The generated raw bindings must remain protocol-level:

```text
generated/mod.rs
generated/ids.rs
generated/types.rs
generated/operations.rs
generated/static_variants.rs
generated/fc.rs
```

SDK adapters may be generated later, but only as an explicit opt-in output.

## Protocol helper boundary

The protocol binding generator may emit small helper methods on raw generated types when the helper is a mechanical spelling of an existing protocol shape. These helpers reduce repetitive Rust construction or matching without adding SDK intent, chain-state policy, RPC behavior, signing, or broadcast semantics.

Current accepted helpers are:

- `Operation::{operation_name}(value)` constructors for operation static-variant arms.
- `Operation::as_{operation_name}()` accessors for operation static-variant arms.
- `FutureExtensions::empty()` for a static variant with a `void_t` arm.
- Object ID wrapper constructors and conversions such as `AccountId::new(...)`, `AssetId::new(...)`, `From<String>`, and `From<&str>`.
- `Asset::new(amount, asset_id)` for the raw `asset` protocol struct.
- `Price::new(base, quote)` for the raw `price` protocol struct.

A new generated protocol helper is acceptable only when all of these are true:

1. It maps directly to fields or variants already present in the protocol schema.
2. It does not choose defaults except for an existing protocol empty marker such as `void_t`.
3. It does not perform validation that depends on chain state, account state, precision, fees, balances, or live RPC data.
4. It does not render broadcast JSON or encode operation-specific JSON policy.
5. It does not introduce a common SDK input model or user-intent abstraction.
6. It has generator tests and checked-in generated output for the current chain bindings.

These helpers must stay narrow. Do not generate constructors for every struct just because the generator can see fields. Extension-heavy structs, operation structs, and transaction structs often carry unsupported branches or SDK-policy choices; their construction belongs in manual adapters, shared operation builders, or an explicit generated SDK adapter profile.

The generator should continue to reject or omit unsupported protocol branches through existing fail-closed FC and JSON surfaces. Protocol helpers are not a back door for memo support, non-empty extension sets, bitasset creation, prediction markets, fee lookup, signing, broadcast, or transaction-header derivation.

## Current manual adapter boundary

Each manual adapter does exactly two things:

1. Build a raw generated `Transaction` containing one generated operation.
2. Render a generated `SignedTransaction` into Graphene `broadcast_transaction` JSON for that operation.

Adapters must not do:

- RPC connection management.
- Dynamic global property lookup.
- Transaction header derivation.
- Account or asset lookup.
- Asset precision lookup.
- Decimal amount parsing or formatting.
- Fee lookup.
- Balance checks.
- WIF handling.
- Signing orchestration.
- Public key recovery.
- Broadcast submission.
- Post-broadcast confirmation.
- Environment variable or CLI handling.

Those responsibilities belong in examples, CLIs, or higher-level SDK orchestration.

## Design A: minimal shared adapter module

Design A generates one small module, for example:

```rust
pub mod sdk_adapter {
    pub struct TransactionHeader {
        pub ref_block_num: u16,
        pub ref_block_prefix: u32,
        pub expiration: String,
    }

    pub fn transaction(
        header: TransactionHeader,
        operations: Vec<crate::generated::static_variants::Operation>,
    ) -> crate::generated::types::Transaction;

    pub fn signed_transaction_json(
        signed_transaction: &crate::generated::types::SignedTransaction,
    ) -> Result<serde_json::Value, BroadcastJsonError>;
}
```

The caller constructs generated operation values directly:

```rust
let tx = sdk_adapter::transaction(
    header,
    vec![Operation::TransferOperation(Box::new(TransferOperation {
        fee,
        from,
        to,
        amount,
        memo: None,
        extensions: FutureExtensions::VoidT(Box::new(())),
    }))],
);
```

This keeps the public API tiny and avoids generating per-operation input structs.

The hidden complexity is the JSON renderer. It still needs private, operation-specific rendering for object IDs, signatures, static variant tags, empty extension lists, `Authority`, `AccountOptions`, `AssetOptions`, and fail-closed unsupported fields.

Design A is deep and hard to misuse at the generator boundary, but it is not very ergonomic. It does not replace the repeated manual builder shapes; it only centralizes broadcast JSON rendering.

## Design B: manifest-driven capability generator

Design B adds an explicit capability manifest, such as:

```toml
[sdk]
enabled = true
out_dir = "src/sdk/generated"

[[sdk.capabilities]]
id = "transfer"
operation = "transfer_operation"
module = "transfer"

[[sdk.capabilities.input_fields]]
name = "from_id"
type = "String"

[sdk.capabilities.operation_fields]
fee = { kind = "asset", amount = "fee_amount", asset_id = "fee_asset_id" }
from = { kind = "id", type = "AccountId", input = "from_id" }
to = { kind = "id", type = "AccountId", input = "to_id" }
amount = { kind = "asset", amount = "amount", asset_id = "asset_id" }
memo = { kind = "default", value = "none", unsupported_if_present = true }
extensions = { kind = "future_extensions_void" }
```

The generator validates the manifest against the spec and emits operation modules matching the manifest.

This is flexible: new chains can override inputs, defaults, guards, and helper behavior without editing generator source. It also makes capability decisions auditable.

The cost is substantial generator complexity. The manifest becomes another API, can drift from the schema, and invites advanced extension hooks before there is evidence they are needed. It is the most extensible design but the largest public surface.

## Design C: opt-in `common-chain` profile

Design C keeps default generation unchanged and adds an explicit profile for the current common chain-package use case.

Default CLI remains protocol-only:

```bash
open-graphene-gen-bindings-rs \
  --spec ./dist/swaplock.open-graphene.json \
  --out-dir ./src/generated
```

Common adapter generation is opt-in:

```bash
open-graphene-gen-bindings-rs \
  --spec ./dist/swaplock.open-graphene.json \
  --out-dir ./src/generated \
  --sdk-out-dir ./src/sdk/generated \
  --sdk-profile common-chain
```

The library API can preserve the existing function and add a config-based variant:

```rust
pub fn generate_bindings(
    spec_path: impl AsRef<std::path::Path>,
    out_dir: impl AsRef<std::path::Path>,
) -> Result<GenerateBindingsResult>;

pub fn generate_bindings_with_config(
    spec_path: impl AsRef<std::path::Path>,
    config: GenerateBindingsConfig,
) -> Result<GenerateBindingsResult>;

pub struct GenerateBindingsConfig {
    pub generated_out_dir: std::path::PathBuf,
    pub sdk: SdkAdapterConfig,
}

pub enum SdkAdapterConfig {
    Disabled,
    CommonChain {
        out_dir: std::path::PathBuf,
        operations: CommonSdkOperations,
    },
    Explicit {
        out_dir: std::path::PathBuf,
        adapters: Vec<SdkAdapterSpec>,
    },
}

pub struct CommonSdkOperations {
    pub transfer: bool,
    pub account_create: bool,
    pub asset_issue: bool,
    pub asset_create: bool,
}
```

The generated output should go under a generated subdirectory rather than directly overwriting hand-written modules:

```text
src/sdk/generated/mod.rs
src/sdk/generated/common.rs
src/sdk/generated/transfer.rs
src/sdk/generated/account_create.rs
src/sdk/generated/asset_issue.rs
src/sdk/generated/asset_create.rs
```

The package can later choose how to re-export generated modules from `src/sdk/mod.rs`.

A common generated input shape can introduce shared primitives:

```rust
pub struct TransactionHeaderInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
}

pub struct FeeInput {
    pub amount: i64,
    pub asset_id: String,
}
```

Example generated transfer API:

```rust
pub struct TransferTransactionInput {
    pub header: TransactionHeaderInput,
    pub from_id: String,
    pub to_id: String,
    pub asset_id: String,
    pub amount: i64,
    pub fee: FeeInput,
}

pub fn build_transfer_transaction(input: TransferTransactionInput) -> Transaction;

pub fn signed_transfer_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<serde_json::Value, TransferJsonError>;
```

Example caller experience:

```rust
let transaction = build_transfer_transaction(TransferTransactionInput {
    header,
    from_id: "1.2.100".to_string(),
    to_id: "1.2.101".to_string(),
    asset_id: "1.3.0".to_string(),
    amount: 100_000,
    fee: FeeInput { amount: 200_000, asset_id: "1.3.0".to_string() },
});

let json = signed_transfer_transaction_json(&signed_transaction)?;
```

The common profile is curated, not fully generic. It may generate only proven safe branches:

- `transfer`: `memo: None`, empty extensions.
- `account_create`: generated `Authority` and `AccountOptions` input, empty account-create extension set only.
- `asset_issue`: `memo: None`, empty extensions.
- `asset_create`: UIA-only, no bitasset options, no prediction market, no non-empty authority or market lists, no non-empty additional asset options.

Advanced branches require explicit opt-in, later:

```toml
[sdk]
profile = "explicit"
out_dir = "src/sdk/generated"

[[sdk.adapters]]
operation = "asset_create_operation"
mode = "transaction_and_json"

[[sdk.adapters.field_overrides]]
field = "bitasset_opts"
exposure = "input_optional_with_default"
```

## Comparison

Design A has the smallest public API and the best separation from operation-specific ergonomic choices. It is attractive if the only duplicated concern we want to remove is broadcast JSON rendering. Its weakness is caller experience: users still construct verbose generated operation structs, and it does not meaningfully replace the manual per-flow adapter API that chain packages now use.

Design B is the most flexible and the most explicit. It can express custom fields, guards, helpers, and future extension points without changing generator source. Its weakness is that it creates a second schema language for SDK adapters. That is premature while only four flows have been proven and the unsupported branches are still intentionally narrow.

Design C fits the current evidence best. It keeps default protocol generation pure, gives chain packages a simple opt-in for the four common proven flows, and leaves a future `Explicit` mode for more advanced adapter shapes. Its main trade-off is that the generator will contain curated knowledge for common Graphene operations, but that knowledge already exists in the manual adapters and is now backed by tests and live Swaplock proofs.

## Recommendation

Adopt Design C first: an opt-in `common-chain` SDK adapter profile emitted into `src/sdk/generated`, with default generation unchanged.

Do not implement the `Explicit` manifest mode yet. Define the enum/config shape so the direction is clear, but only support `Disabled` and `CommonChain` in the first implementation slice.

This gives a low-friction path to replace duplicated manual adapters while keeping generated SDK behavior deliberately narrow and fail-closed.

## Implementation guardrails for a future slice

- Do not overwrite hand-written `src/sdk/*.rs` files by default.
- Emit generated SDK files under `src/sdk/generated/`.
- Preserve raw generated bindings as protocol-level modules.
- Generate only operations present in the spec and operation static variant.
- Use operation tags from the spec, not a hardcoded global table.
- Validate required generated types before emitting an adapter.
- Fail generation with clear diagnostics when requested common operations are absent.
- Keep all RPC, signing, fee lookup, balance checks, and broadcast orchestration outside generated adapters.
- Preserve fail-closed JSON rendering for unproven fields.
- Golden-test generated output against current manual Swaplock and BitShares adapter behavior before replacing manual modules.

## Proven unsupported branches to preserve

The first generated adapters must reject or omit these exactly as the manual adapters do:

- `transfer.memo: Some(_)`
- non-empty `transfer.extensions`
- non-empty `account_create_operation_ext`
- `asset_issue.memo: Some(_)`
- non-empty `asset_issue.extensions`
- `asset_create.bitasset_opts: Some(_)`
- `asset_create.is_prediction_market: true`
- non-empty `asset_create.common_options.whitelist_authorities`
- non-empty `asset_create.common_options.blacklist_authorities`
- non-empty `asset_create.common_options.whitelist_markets`
- non-empty `asset_create.common_options.blacklist_markets`
- non-empty `asset_create.common_options.extensions`
- non-empty transaction extensions

## Notes from live asset_create proof

The Swaplock live asset-create proof exposed two protocol details that generated SDK adapters must respect:

1. `asset_options.core_exchange_rate` is required protocol data and appears in `FC_REFLECT`; the spec generator must retain fields with function-call default initializers.
2. `additional_asset_options` is a Graphene extension set. Empty values FC-serialize as `varint(0)` and render as `[]` in broadcast JSON. Serializing the struct as several optional fields creates a digest mismatch and invalid signatures.
