#!/usr/bin/env bash
set -euo pipefail

BRIDGE_URL="${NOVYWAVE_DESKTOP_BRIDGE_URL:-http://127.0.0.1:9226}"
WORKSPACE_TEMPLATE_ROOT="${NOVYWAVE_WORKSPACE_TEMPLATE_ROOT:-/tmp/novywave_row_resize_bench/many_rows_valid}"
POLL_SECONDS="${NOVYWAVE_POLL_SECONDS:-30}"
SETTLE_SECONDS="${NOVYWAVE_SETTLE_SECONDS:-4}"
ROW_DELTAS=(${NOVYWAVE_ROW_DELTAS:-8 16 24 32 40 48 56 64 56 48 40 32 24 16 8 0})

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "missing required command: $1" >&2
        exit 1
    }
}

require_cmd curl
require_cmd jq

WORKSPACE_ROOT="$(mktemp -d /tmp/novywave_row_resize_desktop_XXXXXX)"
trap 'rm -rf "$WORKSPACE_ROOT"' EXIT
cp "$WORKSPACE_TEMPLATE_ROOT/.novywave" "$WORKSPACE_ROOT/.novywave"

curl_json() {
    curl -fsS "$@"
}

assert_backend_ready() {
    local ready
    ready=$(curl_json "$BRIDGE_URL/state/config-debug" | jq -r '.value.serverReady // false')
    if [[ "$ready" != "true" ]]; then
        echo "desktop backend is not ready; refusing to trust live verification" >&2
        return 1
    fi
}

assert_console_clean() {
    local error_count
    error_count=$(curl_json "$BRIDGE_URL/state/console-log" | jq '[.value[] | .message // "" | select(test("Connection refused|Load failed|send_up_msg error|reload_sse|up_msg_handler"; "i"))] | length')
    if [[ "$error_count" -gt 0 ]]; then
        echo "desktop console contains transport errors; refusing to trust live verification" >&2
        curl_json "$BRIDGE_URL/state/console-log" | jq '[.value[] | select((.message // "") | test("Connection refused|Load failed|send_up_msg error|reload_sse|up_msg_handler"; "i"))]'
        return 1
    fi
}

wait_for_workspace() {
    local selected_count loaded_count cursor_count loading_count render_count server_ready
    for _ in $(seq 1 "$POLL_SECONDS"); do
        server_ready=$(curl_json "$BRIDGE_URL/state/config-debug" | jq -r '.value.serverReady // false')
        selected_count=$(curl_json "$BRIDGE_URL/state/selected-variables" | jq '.value | length')
        loaded_count=$(curl_json "$BRIDGE_URL/state/loaded-files" | jq '.value | length')
        cursor_count=$(curl_json "$BRIDGE_URL/state/cursor-values" | jq '.value | length')
        loading_count=$(curl_json "$BRIDGE_URL/state/cursor-values" | jq '[.value[] | tostring | select(contains("Loading"))] | length')
        render_count=$(curl_json "$BRIDGE_URL/state/timeline" | jq '.value.renderCount // 0')
        if [[ "$server_ready" == "true" && "$selected_count" -gt 0 && "$loaded_count" -gt 0 && "$cursor_count" -ge "$selected_count" && "$loading_count" -eq 0 && "$render_count" -gt 0 ]]; then
            return 0
        fi
        sleep 1
    done
    echo "workspace did not become ready within ${POLL_SECONDS}s" >&2
    return 1
}

main() {
    curl_json "$BRIDGE_URL/health" >/dev/null
    curl_json -X POST --data "$WORKSPACE_ROOT" "$BRIDGE_URL/workspace/select" >/dev/null
    wait_for_workspace
    assert_backend_ready
    curl_json -X POST "$BRIDGE_URL/action/clear-console-log" >/dev/null
    curl_json -X POST "$BRIDGE_URL/window/focus" >/dev/null || true
    sleep "$SETTLE_SECONDS"
    assert_console_clean

    local unique_id unique_id_json
    unique_id=$(curl_json "$BRIDGE_URL/state/selected-variables" | jq -r '.value[0].uniqueId')
    unique_id_json=$(printf '%s' "$unique_id" | jq -Rs .)

    curl_json -X POST "$BRIDGE_URL/action/reset-perf-counters" >/dev/null
    curl_json -X POST "$BRIDGE_URL/action/start-frame-sampler" >/dev/null
    curl_json \
        -X POST \
        -H "Content-Type: application/json" \
        -d "{\"uniqueId\":$unique_id_json}" \
        "$BRIDGE_URL/action/start-row-resize" >/dev/null

    local delta
    for delta in "${ROW_DELTAS[@]}"; do
        curl_json \
            -X POST \
            -H "Content-Type: application/json" \
            -d "{\"deltaY\":$delta}" \
            "$BRIDGE_URL/action/move-active-drag" >/dev/null
    done

    curl_json -X POST "$BRIDGE_URL/action/end-active-drag" >/dev/null

    local perf_json frame_json
    frame_json=$(curl_json -X POST "$BRIDGE_URL/action/stop-frame-sampler")
    sleep 1
    assert_backend_ready
    assert_console_clean
    perf_json=$(curl_json "$BRIDGE_URL/state/perf-counters")

    jq -n \
        --arg workspaceRoot "$WORKSPACE_ROOT" \
        --arg uniqueId "$unique_id" \
        --argjson perf "$(printf '%s' "$perf_json" | jq '.value')" \
        --argjson frames "$(printf '%s' "$frame_json" | jq '.value')" \
        '{
            workspaceRoot: $workspaceRoot,
            uniqueId: $uniqueId,
            perf: $perf,
            frames: $frames
        }'
}

main
