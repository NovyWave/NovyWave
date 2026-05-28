#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "ERROR: usage: scripts/validate_chrome_frontend.sh <frontend-copy>..." >&2
  exit 1
fi

for root in "$@"; do
  if [ ! -e "$root" ]; then
    echo "ERROR: required Chrome frontend path missing: $root" >&2
    exit 1
  fi

  if grep -R "window.__TAURI__" "$root" >/dev/null 2>&1; then
    echo "ERROR: Chrome package contains Tauri frontend bindings under: $root" >&2
    exit 1
  fi

  if grep -R "127.0.0.1:8082" "$root" >/dev/null 2>&1; then
    echo "ERROR: Chrome package contains dev-server port 8082 under: $root" >&2
    exit 1
  fi
done

echo "[OK] Chrome frontend validation passed for: $*"
