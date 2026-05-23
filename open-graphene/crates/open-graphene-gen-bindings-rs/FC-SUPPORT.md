# FC Serialization Support Boundaries

This crate generates raw Rust bindings from an Open Graphene protocol spec. FC serialization is intentionally wire-level and fail-closed: generated code should serialize only shapes whose Graphene FC semantics are known, and should return an explicit error for unsupported values rather than silently emitting plausible but wrong bytes.

## Core rules

- Preserve spec-derived shapes. Do not invent fake structs, `serde_json::Value` payloads, or string aliases to make unsupported data compile.
- Do not treat string-shaped protocol values as ordinary FC strings unless the spec type is actually `string`.
- Do not sort or canonicalize user-provided `flat_set` / `flat_map` values. Generated code verifies that values are already sorted and unique, then fails explicitly if they are not.
- Nested operation serialization must remain fallible. Errors from nested operation payloads must propagate to the caller.
- Prefer narrow, audited renderers over broad support when a static variant has special wire semantics.

## Currently supported FC surfaces

- Primitive FC encodings: bool, fixed-width integers, UTF-8 strings, bytes, fixed bytes.
- Protocol object IDs with generated expected space/type validation.
- Public keys via compressed key bytes and checksum validation.
- `time_point_sec` as Unix seconds encoded as little-endian `u32`.
- `vote_id` as packed Graphene vote ID.
- Operation static variants as `varint tag + operation payload`.
- Base `transaction` FC bytes, generated signature preimage bytes as `chain_id + transaction_fc_bytes`, and generated SHA-256 signature digest bytes when the spec provides `chain.chainId`.
- `future_extensions` empty variant support.
- Audited static variants:
  - `special_authority`
  - `htlc_hash`
  - `predicate`
  - `vesting_policy_initializer`
  - `worker_initializer`
  - `limit_order_auto_action`
  - `fee_parameters`
  - `argument_type`
- Guarded `flat_set` / `flat_map` cases used by Swaplock operation FC coverage, including protocol object IDs, public keys, scalar sets, fixed-byte sets, fee parameter sets, and selected flat maps.

## Intentionally fail-closed

These are not modeled as fake shapes and must not be serialized as ordinary strings or generic JSON:

- `Signature` FC serialization.
- `Address` FC serialization. Current authority support accepts empty `address_auths` and rejects non-empty values.
- `MemoData` in `TransferOperation`. `memo: None` is supported; `memo: Some(_)` returns an explicit unsupported-value error.
- Unknown or unaudited future static variant payloads.
- Map/set cases without confirmed ordering semantics.

## Special cases to preserve

### `HtlcHash`

Generated Rust payloads are `Vec<u8>`, but FC wire payloads are fixed-size byte arrays. The renderer must call `write_fixed_bytes` with the variant-specific length instead of serializing the vector as length-prefixed bytes.

### `fee_parameters`

`fee_parameters` is a synthetic static variant derived from operation fee parameter structs. It serializes as `varint tag + payload`, and `set<fee_parameters>` verifies sorted/unique ordering by static variant tag.

### `argument_type`

`argument_type` includes recursive restriction vectors and the `variant_assert_argument_type` pair payload. Tag 41 is represented as `(i64, Vec<Restriction>)`, matching the spec `pair<int64, vector<restriction>>`; this is not a fake shape.

### Nested `operation`

`op_wrapper.op` uses the generated `Operation` static variant. Nested operation bytes are `varint operation tag + operation payload`. Unsupported nested payloads must return the nested error rather than succeeding silently.

### Transaction signature preimage and digest

When a spec contains `chain.chainId` and a generated `Transaction` type, bindings expose `Transaction::signature_preimage_bytes()`. The preimage is exactly 32 decoded chain-id bytes followed by the transaction FC bytes. Bindings also expose `Transaction::signature_digest_bytes()`, which returns SHA-256 of that preimage. These helpers do not sign, do not serialize signatures, and must keep propagating nested operation serialization errors.

## Verification expectations

When changing FC support, run at minimum:

```bash
cargo test --manifest-path open-graphene-packages/rust/graphene-fc/Cargo.toml
cd open-graphene && cargo test -p open-graphene-gen-bindings-rs
cd open-graphene && cargo test -p open-graphene-spec-gen
open-graphene-packages/rust/graphene-chain-swaplock/bin/check.sh
```

After verification, remove any generated `target/` directories before committing.
