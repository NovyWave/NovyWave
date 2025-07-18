---
allowed-tools: Bash(*), mcp__browsermcp__browser_navigate
description: Start NovyWave development server
---

Start NovyWave development server. Check for conflicts and recommend /project-stop if needed.

```bash
# Check if port is already in use (reliable check)
PORT=$(grep "^port = " MoonZoon.toml | head -1 | cut -d' ' -f3)
if [ -n "$PORT" ]; then
  PORT_PID=$(lsof -ti:$PORT 2>/dev/null)
  if [ -n "$PORT_PID" ]; then
    echo "Port $PORT is already in use by process $PORT_PID. Use /project-stop to kill existing server."
    exit 1
  fi
fi

# Create log file and acquire exclusive lock
if ! rm -f dev_server.log || ! touch dev_server.log; then
  echo "Failed to create dev_server.log"
  exit 1
fi

# Acquire exclusive lock on dev_server.log - prevents multiple servers logging to same file
exec 200>dev_server.log
if ! flock -n 200; then
  echo "Another server is using dev_server.log. Use /project-stop to kill existing server."
  exit 1
fi

# Start server with lock held (output already redirected by Makefile.toml)
echo "Starting development server..."
makers start &
MAKERS_PID=$!

# Wait for compilation and backend startup
echo "Waiting for compilation (this can take several minutes)..."
LAST_SIZE=0
STABLE_COUNT=0

while true; do
  sleep 5
  
  # Check if makers process is still running
  if ! kill -0 $MAKERS_PID 2>/dev/null; then
    echo "Makers process died, check log:"
    tail -10 dev_server.log
    exit 1
  fi
  
  # Check if server is ready from log
  if grep -q "Server is running" dev_server.log; then
    echo "Server started successfully"
    break
  fi
  
  # Check if log file is growing (compilation ongoing)
  CURRENT_SIZE=$(wc -c < dev_server.log 2>/dev/null || echo 0)
  if [ "$CURRENT_SIZE" -eq "$LAST_SIZE" ]; then
    STABLE_COUNT=$((STABLE_COUNT + 1))
  else
    STABLE_COUNT=0
    echo "Compilation in progress..."
  fi
  LAST_SIZE=$CURRENT_SIZE
  
  # Timeout after 3 minutes
  if [ $STABLE_COUNT -ge 36 ]; then
    echo "Server startup timed out after 3 minutes"
    kill -9 $MAKERS_PID 2>/dev/null || true
    exit 1
  fi
done

# Show URLs and QR code from log (always display in summary)
echo ""
echo "=== Server Status ==="
if grep -q "Server is running" dev_server.log; then
  echo "✓ Server started successfully"
  echo ""
  echo "URLs:"
  grep -E "Server is running|Server URL" dev_server.log
  echo ""
  echo "QR Code:"
  grep -A 15 "█" dev_server.log | head -15
else
  echo "⚠ Server may not be fully ready, check log manually"
fi
```

ALWAYS show summary with URLs and QR code extracted from dev_server.log, then navigate to the server using browser MCP.

**Required Summary Format:**
```
**Server URLs:**
- Local: [extract from log]
- Network: [extract from log]

**QR Code:**
[Full QR code block from log]
```

Then navigate to the local URL extracted from the log with browser MCP to verify the server is running.