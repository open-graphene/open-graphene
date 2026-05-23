#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHAIN_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BINDINGS_MANIFEST="${CHAIN_DIR}/graphene-chain-swaplock-bindings/Cargo.toml"

usage() {
  cat <<'USAGE'
Usage:
  transfer.sh <to-account> [amount]
  SWAPLOCK_TO_ACCOUNT=<to-account> [SWAPLOCK_TRANSFER_AMOUNT=amount] transfer.sh

Broadcasts a signed transfer on the configured Swaplock testnet.

Required environment:
  SWAPLOCK_RPC_URL       WebSocket endpoint, wss:// by default
  SWAPLOCK_ACTIVE_WIF    Active private key WIF; never printed
  SWAPLOCK_ACCOUNT       Sender account name

Inputs:
  <to-account> / SWAPLOCK_TO_ACCOUNT
      Receiver account. Must differ from SWAPLOCK_ACCOUNT.
  [amount] / SWAPLOCK_TRANSFER_AMOUNT
      Human-readable amount, converted using asset precision. Default: 1
  SWAPLOCK_ASSET_ID
      Asset id. Default: 1.3.0
  SWAPLOCK_MAX_FEE
      Max raw fee guard. Default: 1000000

Example:
  transfer.sh committee-account 1

This script performs a real network_broadcast.broadcast_transaction call.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if (( $# > 2 )); then
  usage >&2
  exit 2
fi

if (( $# >= 1 )); then
  export SWAPLOCK_TO_ACCOUNT="$1"
fi

if (( $# >= 2 )); then
  export SWAPLOCK_TRANSFER_AMOUNT="$2"
fi

if [[ -z "${SWAPLOCK_TO_ACCOUNT:-}" ]]; then
  printf 'error: receiver is required; pass <to-account> or set SWAPLOCK_TO_ACCOUNT\n\n' >&2
  usage >&2
  exit 2
fi

printf 'Broadcasting Swaplock transfer: %s -> %s amount=%s asset=%s\n' \
  "${SWAPLOCK_ACCOUNT:-<missing SWAPLOCK_ACCOUNT>}" \
  "${SWAPLOCK_TO_ACCOUNT}" \
  "${SWAPLOCK_TRANSFER_AMOUNT:-1}" \
  "${SWAPLOCK_ASSET_ID:-1.3.0}"

exec cargo run --manifest-path "${BINDINGS_MANIFEST}" --example signed_transfer_preview
