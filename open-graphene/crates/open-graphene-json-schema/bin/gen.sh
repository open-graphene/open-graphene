#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${CRATE_DIR}/dist"
OUT_FILE="${DIST_DIR}/schema.json"

mkdir -p "${DIST_DIR}"

cargo run --quiet \
  --manifest-path "${CRATE_DIR}/Cargo.toml" \
  --example emit-schema \
  > "${OUT_FILE}"

printf 'Wrote %s\n' "${OUT_FILE}"
