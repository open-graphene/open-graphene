# open-graphene

SDK tooling for [Graphene](https://github.com/cryptonomex/graphene)-based blockchains
(BitShares family). A machine-readable protocol specification is extracted from a
chain's C++ sources, and language bindings are generated from that specification.

Swaplock has the broadest SDK coverage. Native TypeScript APIs support operation construction, FC encoding,
multisignature transactions, encrypted memo and live subscriptions for **Swaplock and BitShares**; both consume generated bindings.
The Acta and R-Squared crates remain empty placeholders.

## Pipeline

```
blockchains/<chain>/<chain>-core          (vendored C++ sources, not tracked here)
        │
        │  open-graphene-spec-gen  (extracts FC_REFLECT/struct/enum/RPC facts)
        ▼
graphene-chain-<chain>-spec/dist/<chain>.open-graphene.json   (committed spec)
        │
        │  open-graphene-gen-bindings-rs  (spec → Rust modules)
        ▼
graphene-chain-<chain>-bindings/src/generated/*.rs            (committed, do not edit)
```

Regenerate for a chain (example: Swaplock):

```sh
open-graphene-packages/rust/graphene-chain-swaplock/graphene-chain-swaplock-spec/bin/gen.sh
open-graphene-packages/rust/graphene-chain-swaplock/graphene-chain-swaplock-bindings/bin/gen.sh
```

Both the spec JSON and the generated Rust sources are committed; CI regenerates
them and fails if the committed output drifts from the generators.

The same specs now feed `open-graphene-gen-bindings-ts`. The native TypeScript
implementation includes generated types, JSON/RPC bindings and all nonvirtual
operation FC encoders, plus compound transactions verified on the Swaplock testnet. See the
[TypeScript workspace](open-graphene-packages/typescript/README.md) for setup,
tests and the [Rust parity report](docs/TYPESCRIPT-RUST-PARITY-2026-09-29.md).

## Repository layout

| Path | Purpose |
|---|---|
| `open-graphene/crates/open-graphene-spec-gen` | C++ → protocol-spec extractor (config-driven per chain) |
| `open-graphene/crates/open-graphene-json-schema` | The spec's typed IR + published JSON Schema (`dist/schema.json`) |
| `open-graphene/crates/open-graphene-gen-bindings-rs` | Spec → Rust bindings generator |
| `open-graphene/crates/open-graphene-gen-bindings-ts` | Spec → native TypeScript bindings generator |
| `open-graphene/crates/open-graphene-codegen-common` | Shared protocol compatibility rules for generators |
| `open-graphene-packages/typescript` | Native TypeScript workspace; bindings, RPC, transactions, wallets and subscriptions |
| `open-graphene-packages/rust/graphene` | SDK facade (currently Swaplock-only) |
| `open-graphene-packages/rust/graphene-chain-swaplock` | Swaplock spec, generated bindings, and hand-written API layer |
| `open-graphene-packages/rust/graphene-chain-bitshares` | BitShares spec + bindings (cross-chain sanity check) |
| `open-graphene-packages/rust/graphene-fc` | FC wire primitives: keys, signing, base58, memo crypto |
| `open-graphene-packages/rust/graphene-core` | Pure chain helpers: amounts, tx headers, name validation |
| `open-graphene-packages/rust/graphene-transport` | WebSocket JSON-RPC transport (session + live dispatcher) |
| `open-graphene-packages/rust/graphene-primitives` | Dependency-free ID newtypes |
| `blockchains/` | Vendored upstream repos, managed via `mirror.toml` (untracked) |

Design notes live next to the code they describe, e.g.
`open-graphene/crates/open-graphene-gen-bindings-rs/FC-SUPPORT.md` (fail-closed FC
serialization policy) and `docs/DECISIONS.md` (append-only decisions register).

## Development

```sh
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

The generated `src/generated/` modules must never be edited by hand — change the
generator (or the spec extractor) and rerun the `gen.sh` scripts instead.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
