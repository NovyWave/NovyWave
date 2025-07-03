---
allowed-tools: Bash(*)
description: Stop NovyWave development server (fast shutdown)
---

## Your Task
Stop the NovyWave development server:

**Kill only our server processes:**
```bash
if [ -f .novywave.pid ]; then
  PID=$(cat .novywave.pid)
  # Find all child processes of our PID
  CHILDREN=$(pgrep -P $PID)
  if [ -n "$CHILDREN" ]; then
    # Kill children first (mzoon, backend)
    for CHILD in $CHILDREN; do
      GRANDCHILDREN=$(pgrep -P $CHILD)
      [ -n "$GRANDCHILDREN" ] && kill -9 $GRANDCHILDREN 2>/dev/null || true
      kill -9 $CHILD 2>/dev/null || true
    done
  fi
  # Kill the parent process
  kill -9 $PID 2>/dev/null || true
  echo "Server process tree stopped"
  rm -f .novywave.pid
else
  echo "No .novywave.pid file - server not managed by project commands"
  PORT=$(grep "^port = " MoonZoon.toml | head -1 | cut -d' ' -f3)
  if lsof -i:$PORT >/dev/null 2>&1; then
    echo "Warning: Something is still using port $PORT"
    echo "Run 'lsof -i:$PORT' to see what process it is"
  fi
fi
```

## Notes
- Uses PID file (.novywave.pid) for reliable process tracking
- Only kills processes started by project commands
- Handles manual/orphaned processes safely by asking user
- No log file removal - preserves development history