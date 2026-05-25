#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHAIN_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROJECT_ROOT="$(cd "${CHAIN_DIR}/../../.." && pwd)"
BINDINGS_MANIFEST="${CHAIN_DIR}/graphene-chain-swaplock-bindings/Cargo.toml"
ENV_FILE="${PROJECT_ROOT}/.env"

if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  set +a
fi

: "${SWAPLOCK_RPC_URL:?Set SWAPLOCK_RPC_URL in .env or environment}"
: "${SWAPLOCK_ACTIVE_WIF:?Set SWAPLOCK_ACTIVE_WIF in .env or environment}"
: "${SWAPLOCK_ACCOUNT:?Set SWAPLOCK_ACCOUNT in .env or environment}"

printf 'Broadcasting Swaplock trading scenario: registrar=%s\n' "${SWAPLOCK_ACCOUNT}"
exec cargo run --manifest-path "${BINDINGS_MANIFEST}" --example trading_scenario
