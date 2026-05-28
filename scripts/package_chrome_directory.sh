#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [ "$#" -ne 6 ]; then
  echo "ERROR: usage: scripts/package_chrome_directory.sh <package-name> <launcher-src> <launcher-dst> <backend-src> <backend-dst> <output-root>" >&2
  exit 1
fi

PACKAGE_NAME="$1"
LAUNCHER_SRC="$2"
LAUNCHER_DST="$3"
BACKEND_SRC="$4"
BACKEND_DST="$5"
OUTPUT_ROOT="$6"
PACKAGE_DIR="${OUTPUT_ROOT}/${PACKAGE_NAME}"

require_path() {
  local path="$1"
  if [ ! -e "$path" ]; then
    echo "ERROR: required path missing: $path" >&2
    exit 1
  fi
}

require_path "$LAUNCHER_SRC"
require_path "$BACKEND_SRC"
require_path frontend_dist
require_path public
require_path frontend_dist/_api
require_path frontend_dist/_api/pkg
require_path frontend_dist/_api/pkg/build_id
require_path plugins/dist

scripts/validate_chrome_frontend.sh frontend_dist/_api/pkg

rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR/frontend" "$PACKAGE_DIR/plugins"

cp "$LAUNCHER_SRC" "$PACKAGE_DIR/$LAUNCHER_DST"
cp "$BACKEND_SRC" "$PACKAGE_DIR/$BACKEND_DST"
cp -r frontend_dist "$PACKAGE_DIR/frontend_dist"
cp -r public "$PACKAGE_DIR/public"
cp -r frontend_dist/_api "$PACKAGE_DIR/_api"
cp -r frontend_dist/_api/pkg "$PACKAGE_DIR/frontend/pkg"
cp -r plugins/dist "$PACKAGE_DIR/plugins/dist"

chmod +x "$PACKAGE_DIR/$LAUNCHER_DST" "$PACKAGE_DIR/$BACKEND_DST" 2>/dev/null || true

scripts/validate_chrome_frontend.sh \
  "$PACKAGE_DIR/frontend_dist/_api/pkg" \
  "$PACKAGE_DIR/_api/pkg" \
  "$PACKAGE_DIR/frontend/pkg"

echo "[OK] Prepared Chrome package directory: $PACKAGE_DIR"
