#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PACKAGE_DIR}/../../../.." && pwd)"
OPEN_GRAPHENE_DIR="${REPO_ROOT}/open-graphene"
CONFIG_FILE="${PACKAGE_DIR}/open-graphene.toml"

if [[ ! -f "${CONFIG_FILE}" ]]; then
  echo "Missing generator config: ${CONFIG_FILE}" >&2
  exit 1
fi

cd "${OPEN_GRAPHENE_DIR}"
exec cargo run -p open-graphene-spec-gen -- --config "${CONFIG_FILE}"
