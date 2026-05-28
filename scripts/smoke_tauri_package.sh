#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <tauri-app-executable-or-appimage>" >&2
  exit 2
fi

APP_PATH="$1"
if [ ! -x "$APP_PATH" ]; then
  echo "ERROR: app executable is not executable: $APP_PATH" >&2
  exit 1
fi

if [ -z "${DISPLAY:-}" ] && [ -z "${NOVYWAVE_SMOKE_UNDER_XVFB:-}" ]; then
  if command -v xvfb-run >/dev/null 2>&1; then
    export NOVYWAVE_SMOKE_UNDER_XVFB=1
    exec xvfb-run -a "$0" "$APP_PATH"
  fi
fi

BRIDGE_PORT="${NOVYWAVE_DESKTOP_TEST_PORT:-9226}"
TIMEOUT_SECONDS="${NOVYWAVE_SMOKE_TIMEOUT_SECONDS:-45}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/novywave-tauri-smoke.XXXXXX")"
APP_LOG="$WORK_DIR/app.log"

cleanup() {
  if [ -n "${APP_PID:-}" ] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
  if [ -z "${NOVYWAVE_SMOKE_KEEP_LOGS:-}" ]; then
    rm -rf "$WORK_DIR"
  else
    echo "Smoke logs kept at: $WORK_DIR"
  fi
}
trap cleanup EXIT

show_smoke_logs() {
  local startup_log="$XDG_CACHE_HOME/novywave/novywave-tauri-startup.log"
  if [ -f "$APP_LOG" ]; then
    echo "=== app log: $APP_LOG ===" >&2
    cat "$APP_LOG" >&2 || true
  fi
  if [ -f "$startup_log" ]; then
    echo "=== Tauri startup log: $startup_log ===" >&2
    cat "$startup_log" >&2 || true
  fi
}

export NOVYWAVE_DESKTOP_TEST_PORT="$BRIDGE_PORT"
export XDG_CONFIG_HOME="$WORK_DIR/config"
export XDG_CACHE_HOME="$WORK_DIR/cache"
export XDG_DATA_HOME="$WORK_DIR/data"

case "$APP_PATH" in
  *.AppImage)
    export APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}"
    ;;
esac

"$APP_PATH" >"$APP_LOG" 2>&1 &
APP_PID=$!

BRIDGE_URL="http://127.0.0.1:${BRIDGE_PORT}"

curl_bridge() {
  curl --fail --silent --show-error --max-time 5 "$@"
}

post_eval() {
  local expression="$1"
  python3 - "$expression" <<'PY' | curl_bridge -X POST --data-binary @- "$BRIDGE_URL/eval"
import json
import sys
print(json.dumps({"expression": sys.argv[1]}))
PY
}

deadline=$((SECONDS + TIMEOUT_SECONDS))
health_json=""
while [ "$SECONDS" -lt "$deadline" ]; do
  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "ERROR: app exited before desktop bridge became ready" >&2
    show_smoke_logs
    exit 1
  fi

  if health_json="$(curl_bridge "$BRIDGE_URL/health" 2>/dev/null)"; then
    if python3 - "$health_json" <<'PY'
import json
import sys
payload = json.loads(sys.argv[1])
raise SystemExit(0 if payload.get("ok") and payload.get("windowFound") else 1)
PY
    then
      break
    fi
  fi
  sleep 0.25
done

if [ -z "$health_json" ] || ! python3 - "$health_json" <<'PY'
import json
import sys
payload = json.loads(sys.argv[1])
raise SystemExit(0 if payload.get("ok") and payload.get("windowFound") else 1)
PY
then
  echo "ERROR: desktop test bridge did not report a main window at $BRIDGE_URL: ${health_json:-<empty>}" >&2
  show_smoke_logs
  exit 1
fi

deadline=$((SECONDS + TIMEOUT_SECONDS))
config_json=""
while [ "$SECONDS" -lt "$deadline" ]; do
  config_json="$(curl_bridge "$BRIDGE_URL/state/config-debug" 2>/dev/null || true)"
  if [ -n "$config_json" ] && python3 - "$config_json" <<'PY'
import json
import sys
payload = json.loads(sys.argv[1])
value = payload.get("value") or {}
raise SystemExit(0 if payload.get("ok") and value.get("configLoaded") and value.get("serverReady") else 1)
PY
  then
    break
  fi
  sleep 0.25
done

if [ -z "$config_json" ] || ! python3 - "$config_json" <<'PY'
import json
import sys
payload = json.loads(sys.argv[1])
value = payload.get("value") or {}
raise SystemExit(0 if payload.get("ok") and value.get("configLoaded") and value.get("serverReady") else 1)
PY
then
  echo "ERROR: config debug state was not ready: ${config_json:-<empty>}" >&2
  show_smoke_logs
  exit 1
fi

deadline=$((SECONDS + TIMEOUT_SECONDS))
roots_json=""
while [ "$SECONDS" -lt "$deadline" ]; do
  roots_json="$(curl_bridge "$BRIDGE_URL/state/file-picker-roots" 2>/dev/null || true)"
  if [ -n "$roots_json" ] && python3 - "$roots_json" <<'PY'
import json
import sys
payload = json.loads(sys.argv[1])
value = payload.get("value") or {}
roots = value.get("roots") or []
raise SystemExit(0 if payload.get("ok") and len(roots) > 0 else 1)
PY
  then
    break
  fi
  sleep 0.25
done

if [ -z "$roots_json" ] || ! python3 - "$roots_json" <<'PY'
import json
import sys
payload = json.loads(sys.argv[1])
value = payload.get("value") or {}
roots = value.get("roots") or []
raise SystemExit(0 if payload.get("ok") and len(roots) > 0 else 1)
PY
then
  echo "ERROR: platform roots did not load: ${roots_json:-<empty>}" >&2
  show_smoke_logs
  exit 1
fi

inputs_json="$(post_eval "Array.from(document.querySelectorAll('input')).map(input => input.value)")"
if ! python3 - "$inputs_json" <<'PY'
import json
import sys
payload = json.loads(sys.argv[1])
values = payload.get("value") or []
if not payload.get("ok"):
    print(f"ERROR: failed to read workspace inputs: {payload}", file=sys.stderr)
    raise SystemExit(1)
if any("Loading workspace" in str(value) for value in values):
    print(f"ERROR: workspace input is still loading: {values}", file=sys.stderr)
    raise SystemExit(1)
if "No workspace" not in values:
    print(f"ERROR: expected packaged fresh run to show 'No workspace', got: {values}", file=sys.stderr)
    raise SystemExit(1)
PY
then
  show_smoke_logs
  exit 1
fi

console_json="$(curl_bridge "$BRIDGE_URL/state/console-log")"
if ! python3 - "$console_json" <<'PY'
import json
import re
import sys
payload = json.loads(sys.argv[1])
entries = payload.get("value") or []
pattern = re.compile(r"(Config error|Workspace load timeout|panic|failed to start)", re.IGNORECASE)
matches = [
    entry for entry in entries
    if pattern.search(str(entry.get("message", entry)))
]
if matches:
    print(f"ERROR: startup console contained blocking errors: {matches}", file=sys.stderr)
    raise SystemExit(1)
PY
then
  show_smoke_logs
  exit 1
fi

DEFAULT_WORKSPACE="$XDG_CONFIG_HOME/novywave/default-workspace"
if [ -e "$DEFAULT_WORKSPACE/.novywave" ]; then
  echo "ERROR: fresh packaged run created workspace config in default workspace: $DEFAULT_WORKSPACE/.novywave" >&2
  show_smoke_logs
  exit 1
fi

BACKEND_CWD="$(sed -n 's/^.*BACKEND cwd = //p' "$APP_LOG" | tail -1)"
if [ -n "$BACKEND_CWD" ] && [ -e "$BACKEND_CWD/.novywave" ]; then
  echo "ERROR: fresh packaged run created workspace config in backend cwd: $BACKEND_CWD/.novywave" >&2
  show_smoke_logs
  exit 1
fi

echo "Tauri package smoke passed"
echo "  app: $APP_PATH"
echo "  bridge: $BRIDGE_URL"
echo "  config: $config_json"
echo "  roots: $roots_json"
