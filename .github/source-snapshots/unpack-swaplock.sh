#!/usr/bin/env bash
set -euo pipefail
snapshot_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$snapshot_dir/../.." && pwd)"
revision="${1:?Pass the pinned Swaplock commit}"
[[ "$revision" =~ ^[0-9a-f]{40}$ ]]
archive="swaplock-$revision.tar.gz"
(cd "$snapshot_dir" && sha256sum -c "$archive.sha256")
mkdir -p "$repo_root/blockchains/swaplock/swaplock-core"
tar -xzf "$snapshot_dir/$archive" -C "$repo_root/blockchains/swaplock/swaplock-core"
