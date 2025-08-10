# /project-stop Command

## Purpose
Cleanly stop all NovyWave development processes

## CRITICAL: Slash Command = Automation

**NEVER provide consultation when user types `/project-stop`**
**ALWAYS execute the shutdown workflow immediately**

## Workflow

### 1. Identify Running Processes
```bash
# Find development processes
ps aux | grep -E "(mzoon|makers)" | grep -v grep

# Check for PID file
if [[ -f .novywave.pid ]]; then
    PID=$(cat .novywave.pid)
    echo "Found stored PID: $PID"
fi
```

### 2. Graceful Termination
```bash
# Terminate processes gracefully
pkill -TERM -f "makers start" 2>/dev/null || echo "No makers process found"
pkill -TERM -f "mzoon" 2>/dev/null || echo "No mzoon process found"

# Wait for graceful shutdown
sleep 2

# Force kill if still running
pkill -KILL -f "makers start" 2>/dev/null || true
pkill -KILL -f "mzoon" 2>/dev/null || true
```

### 3. Cleanup
```bash
# Remove PID and lock files
rm -f .novywave.pid dev_server.lock 2>/dev/null || true

# Truncate log file to prevent excessive growth
> dev_server.log

echo "Development server stopped and cleaned up"
```

### 4. Verification
```bash
# Verify no processes remain
REMAINING=$(ps aux | grep -E "(mzoon|makers)" | grep -v grep | wc -l)
if [[ $REMAINING -eq 0 ]]; then
    echo "✓ All development processes terminated"
else
    echo "⚠ Some processes may still be running"
    ps aux | grep -E "(mzoon|makers)" | grep -v grep
fi
```

## Safety Notes
- Uses graceful TERM signal first, then KILL if needed
- Cleans up PID and lock files to prevent conflicts
- Truncates log file to manage disk space
- Verifies complete shutdown

## Anti-Consultation Guard
This command MUST execute shutdown immediately. Never explain the process termination steps unless explicitly asked after completion.