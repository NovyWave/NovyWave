#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <tauri-linux-package.deb|tauri-linux-package.rpm>" >&2
  exit 2
fi

PACKAGE_PATH="$1"
if [ ! -f "$PACKAGE_PATH" ]; then
  echo "ERROR: package file not found: $PACKAGE_PATH" >&2
  exit 1
fi
PACKAGE_ABS="$(cd "$(dirname "$PACKAGE_PATH")" && pwd)/$(basename "$PACKAGE_PATH")"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/novywave-linux-package-smoke.XXXXXX")"
cleanup() {
  if [ -z "${NOVYWAVE_SMOKE_KEEP_LOGS:-}" ]; then
    rm -rf "$WORK_DIR"
  else
    echo "Linux package smoke logs kept at: $WORK_DIR"
  fi
}
trap cleanup EXIT

EXTRACT_DIR="$WORK_DIR/extract"
mkdir -p "$EXTRACT_DIR"

case "$PACKAGE_PATH" in
  *.deb)
    if ! command -v dpkg-deb >/dev/null 2>&1; then
      echo "ERROR: dpkg-deb is required to smoke deb packages" >&2
      exit 1
    fi
    dpkg-deb -x "$PACKAGE_PATH" "$EXTRACT_DIR"
    ;;
  *.rpm)
    if ! command -v rpm2cpio >/dev/null 2>&1 || ! command -v cpio >/dev/null 2>&1; then
      echo "ERROR: rpm2cpio and cpio are required to smoke rpm packages" >&2
      exit 1
    fi
    RPM_CPIO="$WORK_DIR/package.cpio"
    RPM2CPIO_STATUS=0
    rpm2cpio "$PACKAGE_ABS" >"$RPM_CPIO" || RPM2CPIO_STATUS=$?
    if [ ! -s "$RPM_CPIO" ]; then
      echo "ERROR: rpm2cpio did not produce a cpio payload for $PACKAGE_PATH" >&2
      exit 1
    fi
    (cd "$EXTRACT_DIR" && cpio -idm --quiet < "$RPM_CPIO")
    if [ "$RPM2CPIO_STATUS" -ne 0 ]; then
      echo "WARNING: rpm2cpio exited with $RPM2CPIO_STATUS after producing a readable payload" >&2
    fi
    ;;
  *)
    echo "ERROR: unsupported Linux package type: $PACKAGE_PATH" >&2
    exit 2
    ;;
esac

APP_PATH="$EXTRACT_DIR/usr/bin/novywave"
if [ ! -x "$APP_PATH" ]; then
  echo "ERROR: extracted package does not contain executable usr/bin/novywave" >&2
  echo "Extracted files:" >&2
  find "$EXTRACT_DIR" -maxdepth 6 -type f | sort | sed -n '1,160p' >&2
  exit 1
fi

echo "Smoking extracted Linux package executable: $APP_PATH"
scripts/smoke_tauri_package.sh "$APP_PATH"
