#!/usr/bin/env bash
set -euo pipefail

DEV_PRIVATE_KEY_SHA256='54933a314fa31e0a2efd77084538b40fe58c71d82d70078b18fad9d39f7cee35'

if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  echo "ERROR: TAURI_SIGNING_PRIVATE_KEY is required for release updater artifacts." >&2
  exit 1
fi

PRIVATE_KEY_SHA256="$(python3 - <<'PY'
import hashlib
import os

print(hashlib.sha256(os.environ["TAURI_SIGNING_PRIVATE_KEY"].encode()).hexdigest())
PY
)"

if [ "$PRIVATE_KEY_SHA256" = "$DEV_PRIVATE_KEY_SHA256" ]; then
  echo "ERROR: release build is using the known development updater private key." >&2
  exit 1
fi

if [ "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" = "devkey" ]; then
  echo "ERROR: release build is using the known development updater key password." >&2
  exit 1
fi

echo "[OK] release updater signing key is present and is not the known development key"
