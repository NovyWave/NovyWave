#!/usr/bin/env bash
set -euo pipefail

DIST_DIR="${1:-plugins/dist}"
EXPECTED=(
  "hello_world_plugin.wasm"
  "reload_watcher_plugin.wasm"
  "files_discovery_plugin.wasm"
)

for artifact in "${EXPECTED[@]}"; do
  path="${DIST_DIR}/${artifact}"
  if [ ! -s "$path" ]; then
    echo "ERROR: expected plugin artifact is missing or empty: $path" >&2
    echo "Plugin dist contents:" >&2
    find "$DIST_DIR" -maxdepth 1 -type f -printf '%p %s bytes\n' 2>/dev/null | sort >&2 || true
    exit 1
  fi
  size="$(wc -c < "$path")"
  echo "[OK] plugin artifact: $path (${size} bytes)"
done
