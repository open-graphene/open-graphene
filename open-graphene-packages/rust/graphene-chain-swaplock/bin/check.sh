#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHAIN_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_PACKAGES_DIR="$(cd "${CHAIN_DIR}/.." && pwd)"
SPEC_GEN="${CHAIN_DIR}/graphene-chain-swaplock-spec/bin/gen.sh"
BINDINGS_GEN="${CHAIN_DIR}/graphene-chain-swaplock-bindings/bin/gen.sh"
BINDINGS_MANIFEST="${CHAIN_DIR}/graphene-chain-swaplock-bindings/Cargo.toml"
FC_MANIFEST="${RUST_PACKAGES_DIR}/graphene-fc/Cargo.toml"

step() {
  printf '\n==> %s\n' "$1"
}

step "Generate Swaplock spec"
"${SPEC_GEN}"

step "Generate Swaplock Rust bindings"
"${BINDINGS_GEN}"

step "Test Swaplock Rust bindings"
cargo test --manifest-path "${BINDINGS_MANIFEST}"

step "Check Swaplock Rust examples"
cargo check --manifest-path "${BINDINGS_MANIFEST}" --examples

step "Test shared graphene-fc runtime"
cargo test --manifest-path "${FC_MANIFEST}"

printf '\nSwaplock check passed.\n'
