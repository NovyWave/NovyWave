#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <chrome-mode.AppImage>" >&2
  exit 2
fi

APPIMAGE="$1"
if [ ! -x "$APPIMAGE" ]; then
  echo "ERROR: Chrome AppImage is not executable: $APPIMAGE" >&2
  exit 1
fi

if [ -z "${DISPLAY:-}" ] && [ -z "${NOVYWAVE_SMOKE_UNDER_XVFB:-}" ]; then
  if command -v xvfb-run >/dev/null 2>&1; then
    export NOVYWAVE_SMOKE_UNDER_XVFB=1
    exec xvfb-run -a "$0" "$APPIMAGE"
  fi
fi

find_browser() {
  for name in google-chrome-stable google-chrome chromium chromium-browser microsoft-edge-stable brave-browser; do
    if command -v "$name" >/dev/null 2>&1; then
      command -v "$name"
      return 0
    fi
  done
  return 1
}

CHROME="${NOVYWAVE_CHROME:-}"
if [ -z "$CHROME" ]; then
  CHROME="$(find_browser || true)"
fi
if [ -z "$CHROME" ]; then
  echo "ERROR: no Chromium-based browser found for live Chrome AppImage smoke" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/novywave-chrome-live-smoke.XXXXXX")"
APP_LOG="$WORK_DIR/chrome-appimage.log"
CDP_PORT="${NOVYWAVE_CHROME_CDP_PORT:-$(python3 - <<'PY'
import socket
with socket.socket() as s:
    s.bind(("127.0.0.1", 0))
    print(s.getsockname()[1])
PY
)}"

cleanup() {
  if [ -n "${APP_PID:-}" ] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
  pkill -f -- "--remote-debugging-port=$CDP_PORT" 2>/dev/null || true
  if [ -z "${NOVYWAVE_SMOKE_KEEP_LOGS:-}" ]; then
    rm -rf "$WORK_DIR"
  else
    echo "Chrome live smoke logs kept at: $WORK_DIR"
  fi
}
trap cleanup EXIT

export NOVYWAVE_CHROME="$CHROME"
export NOVYWAVE_CHROME_CDP_PORT="$CDP_PORT"
export XDG_CONFIG_HOME="$WORK_DIR/config"
export XDG_CACHE_HOME="$WORK_DIR/cache"
export XDG_DATA_HOME="$WORK_DIR/data"
export APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}"

"$APPIMAGE" >"$APP_LOG" 2>&1 &
APP_PID=$!

CDP_URL="http://127.0.0.1:${CDP_PORT}"
TIMEOUT_SECONDS="${NOVYWAVE_SMOKE_TIMEOUT_SECONDS:-45}"

curl_cdp() {
  curl --fail --silent --show-error --max-time 5 "$@"
}

deadline=$((SECONDS + TIMEOUT_SECONDS))
pages_json=""
while [ "$SECONDS" -lt "$deadline" ]; do
  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "ERROR: Chrome AppImage exited before Chrome DevTools became ready" >&2
    cat "$APP_LOG" >&2 || true
    exit 1
  fi

  if pages_json="$(curl_cdp "$CDP_URL/json/list" 2>/dev/null)"; then
    if python3 - "$pages_json" <<'PY'
import json
import sys
pages = json.loads(sys.argv[1])
urls = [page.get("url", "") for page in pages]
raise SystemExit(0 if any(url.startswith("http://127.0.0.1:") for url in urls) else 1)
PY
    then
      break
    fi
  fi
  sleep 0.25
done

if [ -z "$pages_json" ]; then
  echo "ERROR: Chrome DevTools did not become ready at $CDP_URL" >&2
  cat "$APP_LOG" >&2 || true
  exit 1
fi

APP_URL="$(python3 - "$pages_json" <<'PY'
import json
import sys
pages = json.loads(sys.argv[1])
for page in pages:
    url = page.get("url", "")
    if url.startswith("http://127.0.0.1:"):
        print(url.rstrip("/"))
        raise SystemExit(0)
raise SystemExit(1)
PY
)"

curl_cdp "$APP_URL/" >/dev/null
curl_cdp "$APP_URL/_api/pkg/build_id" >/dev/null

echo "Chrome AppImage live smoke passed"
echo "  app: $APPIMAGE"
echo "  browser: $CHROME"
echo "  cdp: $CDP_URL"
echo "  url: $APP_URL"
