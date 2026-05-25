#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHAIN_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROJECT_ROOT="$(cd "${CHAIN_DIR}/../../.." && pwd)"
CHAIN_MANIFEST="${CHAIN_DIR}/graphene-chain-swaplock/Cargo.toml"

if [[ -f "${PROJECT_ROOT}/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "${PROJECT_ROOT}/.env"
  set +a
fi

export SWAPLOCK_RPC_URL="${SWAPLOCK_RPC_URL:-wss://node02.swaplock.chainpool.online:8090}"
export SWAPLOCK_ACCOUNT="${SWAPLOCK_ACCOUNT:-swaplock}"
export SWAPLOCK_TO_ACCOUNT="${SWAPLOCK_TO_ACCOUNT:-committee-account}"
export SWAPLOCK_TRANSFER_AMOUNT="${SWAPLOCK_TRANSFER_AMOUNT:-1}"
export SWAPLOCK_ASSET_ID="${SWAPLOCK_ASSET_ID:-1.3.0}"
export SWAPLOCK_MAX_FEE="${SWAPLOCK_MAX_FEE:-1000000}"

if [[ -z "${SWAPLOCK_ACTIVE_WIF:-}" ]]; then
  printf 'error: SWAPLOCK_ACTIVE_WIF is not set; expected it in environment or %s/.env\n' "${PROJECT_ROOT}" >&2
  exit 1
fi

printf 'Broadcasting Swaplock transfer: %s -> %s amount=%s asset=%s\n' \
  "${SWAPLOCK_ACCOUNT}" \
  "${SWAPLOCK_TO_ACCOUNT}" \
  "${SWAPLOCK_TRANSFER_AMOUNT}" \
  "${SWAPLOCK_ASSET_ID}"

exec cargo run --manifest-path "${CHAIN_MANIFEST}" --example signed_transfer_preview
