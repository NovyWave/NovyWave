#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <chrome-package-dir> <launcher-name>" >&2
  exit 2
fi

PACKAGE_DIR="$1"
LAUNCHER_NAME="$2"
LAUNCHER_PATH="$PACKAGE_DIR/$LAUNCHER_NAME"

if [ ! -d "$PACKAGE_DIR" ]; then
  echo "ERROR: Chrome package directory does not exist: $PACKAGE_DIR" >&2
  exit 1
fi

require_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "ERROR: required file missing: $path" >&2
    exit 1
  fi
}

require_dir() {
  local path="$1"
  if [ ! -d "$path" ]; then
    echo "ERROR: required directory missing: $path" >&2
    exit 1
  fi
}

require_file "$LAUNCHER_PATH"
require_dir "$PACKAGE_DIR/frontend_dist"
require_dir "$PACKAGE_DIR/_api/pkg"
require_file "$PACKAGE_DIR/_api/pkg/build_id"
require_dir "$PACKAGE_DIR/frontend/pkg"
require_dir "$PACKAGE_DIR/public"
require_dir "$PACKAGE_DIR/plugins/dist"

case "$LAUNCHER_NAME" in
  *.exe) require_file "$PACKAGE_DIR/backend.exe" ;;
  *) require_file "$PACKAGE_DIR/backend" ;;
esac

scripts/validate_chrome_frontend.sh \
  "$PACKAGE_DIR/frontend_dist/_api/pkg" \
  "$PACKAGE_DIR/_api/pkg" \
  "$PACKAGE_DIR/frontend/pkg"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/novywave-chrome-package-smoke.XXXXXX")"
APP_LOG="$WORK_DIR/chrome-package.log"
FAKE_CHROME_LOG="$WORK_DIR/fake-chrome.log"
FAKE_ROUTE_LOG="$WORK_DIR/fake-chrome-routes.log"

cleanup() {
  if [ -z "${NOVYWAVE_SMOKE_KEEP_LOGS:-}" ]; then
    rm -rf "$WORK_DIR"
  else
    echo "Chrome package smoke logs kept at: $WORK_DIR"
  fi
}
trap cleanup EXIT

create_fake_chrome_unix() {
  local path="$WORK_DIR/fake-chromium"
  cat > "$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = "--version" ]; then
  echo "Chromium 120.0.6099.224"
  exit 0
fi
printf '%s\n' "$*" > "${NOVYWAVE_FAKE_CHROME_LOG:?}"
app_url=""
for arg in "$@"; do
  case "$arg" in
    --app=*) app_url="${arg#--app=}" ;;
  esac
done
if [ -z "$app_url" ]; then
  echo "missing --app URL" > "${NOVYWAVE_FAKE_CHROME_ROUTE_LOG:?}"
  exit 1
fi
for route in "/" "/_api/pkg/build_id"; do
  if ! curl --fail --silent --show-error --max-time 10 "${app_url}${route}" >/dev/null; then
    echo "route failed: ${app_url}${route}" > "${NOVYWAVE_FAKE_CHROME_ROUTE_LOG:?}"
    exit 1
  fi
done
printf 'routes ok: %s\n' "$app_url" > "${NOVYWAVE_FAKE_CHROME_ROUTE_LOG:?}"
exit 0
EOF
  chmod +x "$path"
  printf '%s\n' "$path"
}

create_fake_chrome_windows() {
  local path="$WORK_DIR/fake-chromium.cmd"
  cat > "$path" <<'EOF'
@echo off
setlocal EnableExtensions EnableDelayedExpansion
if "%~1"=="--version" (
  echo Chromium 120.0.6099.224
  exit /b 0
)
echo %*>"%NOVYWAVE_FAKE_CHROME_LOG%"
set "APP_URL="
:parse
if "%~1"=="" goto parsed
set "ARG=%~1"
if "!ARG:~0,6!"=="--app=" set "APP_URL=!ARG:~6!"
shift
goto parse
:parsed
if "%APP_URL%"=="" (
  echo missing --app URL>"%NOVYWAVE_FAKE_CHROME_ROUTE_LOG%"
  exit /b 1
)
curl --fail --silent --show-error --max-time 10 "%APP_URL%/" >NUL
if errorlevel 1 (
  echo route failed: %APP_URL%/>"%NOVYWAVE_FAKE_CHROME_ROUTE_LOG%"
  exit /b 1
)
curl --fail --silent --show-error --max-time 10 "%APP_URL%/_api/pkg/build_id" >NUL
if errorlevel 1 (
  echo route failed: %APP_URL%/_api/pkg/build_id>"%NOVYWAVE_FAKE_CHROME_ROUTE_LOG%"
  exit /b 1
)
echo routes ok: %APP_URL%>"%NOVYWAVE_FAKE_CHROME_ROUTE_LOG%"
exit /b 0
EOF
  printf '%s\n' "$path"
}

run_with_timeout() {
  local seconds="$1"
  shift

  if command -v timeout >/dev/null 2>&1; then
    timeout "${seconds}s" "$@"
    return $?
  fi

  if command -v gtimeout >/dev/null 2>&1; then
    gtimeout "${seconds}s" "$@"
    return $?
  fi

  local timed_out_marker="$WORK_DIR/timeout-fired"
  rm -f "$timed_out_marker"

  "$@" &
  local app_pid=$!
  (
    sleep "$seconds"
    if kill -0 "$app_pid" 2>/dev/null; then
      touch "$timed_out_marker"
      kill "$app_pid" 2>/dev/null || true
    fi
  ) &
  local timer_pid=$!

  local status=0
  wait "$app_pid" || status=$?

  if [ -f "$timed_out_marker" ]; then
    wait "$timer_pid" 2>/dev/null || true
    return 124
  fi

  kill "$timer_pid" 2>/dev/null || true
  wait "$timer_pid" 2>/dev/null || true
  return "$status"
}

case "$(uname -s 2>/dev/null || echo unknown)" in
  MINGW*|MSYS*|CYGWIN*) FAKE_CHROME="$(create_fake_chrome_windows)" ;;
  *) FAKE_CHROME="$(create_fake_chrome_unix)" ;;
esac

export NOVYWAVE_CHROME="$FAKE_CHROME"
export NOVYWAVE_FAKE_CHROME_LOG="$FAKE_CHROME_LOG"
export NOVYWAVE_FAKE_CHROME_ROUTE_LOG="$FAKE_ROUTE_LOG"
export XDG_CONFIG_HOME="$WORK_DIR/config"
export XDG_CACHE_HOME="$WORK_DIR/cache"
export XDG_DATA_HOME="$WORK_DIR/data"
export APPDATA="$WORK_DIR/appdata"
export LOCALAPPDATA="$WORK_DIR/localappdata"
export HOME="$WORK_DIR/home"
mkdir -p "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_DATA_HOME" "$APPDATA" "$LOCALAPPDATA" "$HOME"

run_with_timeout "${NOVYWAVE_SMOKE_TIMEOUT_SECONDS:-45}" "$LAUNCHER_PATH" >"$APP_LOG" 2>&1

grep -q -- "--app=http://127.0.0.1:" "$FAKE_CHROME_LOG"
grep -q -- "routes ok: http://127.0.0.1:" "$FAKE_ROUTE_LOG"

if find "$PACKAGE_DIR" -maxdepth 2 -name .novywave -print -quit | grep -q .; then
  echo "ERROR: Chrome package smoke created .novywave under package resources" >&2
  find "$PACKAGE_DIR" -maxdepth 2 -name .novywave -print >&2
  exit 1
fi

echo "Chrome package directory smoke passed"
echo "  package: $PACKAGE_DIR"
echo "  launcher: $LAUNCHER_NAME"
