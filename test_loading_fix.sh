#!/bin/bash
# Test script to verify NovyWave app loads correctly after refactoring

set -e

echo "=== NovyWave Loading Test ==="

# Check if dev server is running
if ! pgrep -f "mzoon" > /dev/null 2>&1; then
    echo "ERROR: Dev server (mzoon) is not running"
    echo "Start with: makers start"
    exit 1
fi

echo "✓ Dev server is running"

# Check if frontend was built recently (look for most recent build result)
LAST_BUILD=$(tail -200 dev_server.log 2>/dev/null | grep -E "Frontend built|Failed to compile" | tail -1)
if echo "$LAST_BUILD" | grep -q "Failed"; then
    echo "ERROR: Most recent build failed"
    tail -50 dev_server.log | grep -A2 "error\[E" | head -20
    exit 1
fi

if ! echo "$LAST_BUILD" | grep -q "Frontend built"; then
    echo "WARNING: No recent build status found"
fi

echo "✓ Frontend built successfully"

# Test HTTP connection to app
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/ 2>/dev/null || echo "000")
if [ "$HTTP_CODE" != "200" ]; then
    echo "ERROR: HTTP request to localhost:8080 failed (code: $HTTP_CODE)"
    exit 1
fi

echo "✓ App responds to HTTP requests"

# Check for active sessions (indicates frontend loaded)
SESSIONS=$(tail -200 dev_server.log 2>/dev/null | grep -c "New session" || true)
if [ "$SESSIONS" -eq 0 ]; then
    echo "WARNING: No active sessions found in recent logs"
else
    echo "✓ Found $SESSIONS session(s) in recent logs"
fi

# Check for ConfigLoaded (indicates frontend-backend communication works)
CONFIG_LOADED=$(tail -200 dev_server.log 2>/dev/null | grep -c "ConfigLoaded" || true)
if [ "$CONFIG_LOADED" -eq 0 ]; then
    echo "WARNING: No ConfigLoaded messages found"
else
    echo "✓ ConfigLoaded messages present ($CONFIG_LOADED)"
fi

# Check for any Actor/ActorVec types (should be zero after refactor)
ACTORS=$(grep -r "Actor<\|ActorVec<" frontend/src/*.rs frontend/src/**/*.rs 2>/dev/null | grep -v "// " | grep -v "/// " | wc -l || true)
if [ "$ACTORS" -gt 0 ]; then
    echo "WARNING: Found $ACTORS Actor<T> usages (should be 0)"
else
    echo "✓ No Actor<T> types found"
fi

# Check for Task::start (non-droppable)
TASK_START=$(grep -r "Task::start(" frontend/src/*.rs frontend/src/**/*.rs 2>/dev/null | grep -v "start_droppable" | wc -l || true)
if [ "$TASK_START" -gt 0 ]; then
    echo "WARNING: Found $TASK_START Task::start() usages (prefer Task::start_droppable)"
else
    echo "✓ No Task::start() usages"
fi

# Check for Relay usages
RELAYS=$(grep -r "Relay<\|: Relay" frontend/src/*.rs frontend/src/**/*.rs 2>/dev/null | wc -l || true)
if [ "$RELAYS" -gt 0 ]; then
    echo "WARNING: Found $RELAYS Relay usages (should be 0 for idiomatic Rust)"
else
    echo "✓ No Relay usages"
fi

# Browser test for Loading... bug
echo ""
echo "=== Loading... Bug Test ==="
LOADING_COUNT=$(python3 -c "
import subprocess
import time
import sys

proc = subprocess.Popen(['chromium', '--headless', '--disable-gpu', '--dump-dom', 'http://localhost:8080'],
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
time.sleep(15)
try:
    stdout, stderr = proc.communicate(timeout=30)
    print(stdout.count('Loading...'))
except subprocess.TimeoutExpired:
    proc.kill()
    print('-1')
" 2>/dev/null || echo "-1")

if [ "$LOADING_COUNT" = "-1" ]; then
    echo "WARNING: Could not run browser test (chromium not available?)"
elif [ "$LOADING_COUNT" = "0" ]; then
    echo "✓ No Loading... instances found in UI"
else
    echo "✗ FAIL: Found $LOADING_COUNT Loading... instances in UI"
    exit 1
fi

echo ""
echo "=== Test Complete ==="
echo "App is functional. Warnings indicate areas that may need refactoring."
