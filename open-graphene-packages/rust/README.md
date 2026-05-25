# Open Graphene Rust Packages

This directory contains Rust runtime crates, SDK helper crates, and generated chain binding crates for Open Graphene.

## Crates

- `graphene-fc` (`open-graphene-fc`): FC serialization, signing, digest, and WIF helpers.
- `graphene-sdk-core` (`open-graphene-sdk-core`): pure SDK helpers such as amount conversion, transaction headers, balance checks, and object id refs.
- `graphene-sdk-operations` (`open-graphene-sdk-operations`): common operation input models, adapter traits, and generic trait-based transaction builders, split one operation per module.

## Architecture notes

- [Manual SDK Adapter Patterns](TRANSFER-SDK-ADAPTER.md) describes the operation helper boundary used by chain binding crates and the current trait-builder pattern.
- [Common SDK Input and Chain Adapter Design](SDK-INPUT-ADAPTER-DESIGN.md) describes the shared input-model seam and how chain-specific generated types plug into common operation builders.
- [Generated SDK Adapter Capability Design](../../open-graphene/crates/open-graphene-gen-bindings-rs/SDK-CAPABILITY-DESIGN.md) records the opt-in generated SDK adapter direction and the boundary for small protocol-level generated helpers.
