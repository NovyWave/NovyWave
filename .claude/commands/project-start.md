---
allowed-tools: Bash(*), mcp__browsermcp__browser_navigate
description: Start NovyWave development server (fast startup)
---

## Your Task
Start the NovyWave development server:

**Simple start logic:**
```bash
if [ -f .novywave.pid ] && kill -0 $(cat .novywave.pid) 2>/dev/null; then
  echo "Server already running (PID: $(cat .novywave.pid))"
  # Extract URLs from log and show them
else
  rm -f dev_server.log
  makers start > dev_server.log 2>&1 & echo $! > .novywave.pid
  sleep 5 && cat dev_server.log
  # Show server output with QR code
fi
```

**Open browser when ready:**
- Extract URL from log and navigate to it
- Show mobile URL and QR code in summary

## Notes
- Uses PID file (.novywave.pid) for reliable process tracking
- Shows complete QR code and URLs from server log
- Auto-reload enabled for development