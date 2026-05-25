#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHAIN_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROJECT_ROOT="$(cd "${CHAIN_DIR}/../../.." && pwd)"
CHAIN_MANIFEST="${CHAIN_DIR}/graphene-chain-swaplock/Cargo.toml"
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

ASSET_SYMBOL="${SWAPLOCK_ASSET_SYMBOL:-auto-generated}"
echo "Broadcasting Swaplock asset_create: issuer=${SWAPLOCK_ACCOUNT} symbol=${ASSET_SYMBOL}"

exec cargo run --manifest-path "${CHAIN_MANIFEST}" --example asset_create_preview
