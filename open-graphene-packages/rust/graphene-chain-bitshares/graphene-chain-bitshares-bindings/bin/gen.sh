#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CHAIN_DIR="$(cd "${PACKAGE_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PACKAGE_DIR}/../../../.." && pwd)"
OPEN_GRAPHENE_DIR="${REPO_ROOT}/open-graphene"
SPEC_FILE="${CHAIN_DIR}/graphene-chain-bitshares-spec/dist/bitshares.open-graphene.json"
OUT_DIR="${PACKAGE_DIR}/src/generated"

if [[ ! -f "${SPEC_FILE}" ]]; then
  echo "Missing BitShares spec: ${SPEC_FILE}" >&2
  echo "Run: ${CHAIN_DIR}/graphene-chain-bitshares-spec/bin/gen.sh" >&2
  exit 1
fi

cd "${OPEN_GRAPHENE_DIR}"
exec cargo run -p open-graphene-gen-bindings-rs -- --spec "${SPEC_FILE}" --out-dir "${OUT_DIR}"
