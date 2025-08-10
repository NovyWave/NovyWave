# /project-start Command

## Purpose
Start NovyWave development server with proper background process management

## CRITICAL: Slash Command = Automation

**NEVER provide consultation when user types `/project-start`**
**ALWAYS execute the startup workflow immediately**

## Workflow

### 1. Process Cleanup
Check for and terminate any existing development processes:

```bash
# Check for existing processes
ps aux | grep -E "(mzoon|makers)" | grep -v grep

# Kill existing processes if found
pkill -f "makers start" 2>/dev/null || true
pkill -f "mzoon" 2>/dev/null || true

# Clean up lock files
rm -f dev_server.lock .novywave.pid 2>/dev/null || true
```

### 2. Start Development Server
```bash
# Start as background process with output redirection
cd /home/martinkavik/repos/NovyWave
makers start > dev_server.log 2>&1 &

# Get process ID
DEV_PID=$!
echo $DEV_PID > .novywave.pid
echo "Started development server with PID: $DEV_PID"
```

### 3. Initialize Monitoring
```bash
# Wait a moment for startup
sleep 2

# Show initial log output to confirm startup
tail -10 dev_server.log

echo "Development server starting..."
echo "Monitor with: tail -f dev_server.log"
echo "Server will be available at: http://localhost:8080"
```

## Expected Behavior
- **Backend/shared compilation**: Takes DOZENS OF SECONDS TO MINUTES (normal)
- **Auto-reload**: Only triggers after successful compilation
- **Browser access**: Wait for "compilation complete" messages before testing

## Process Management Rules
- **NEVER restart without permission** - only restart when MoonZoon.toml changes
- **PATIENCE REQUIRED**: Wait for compilation to complete, no matter how long
- **MONITOR ONLY**: Use `tail -f dev_server.log` to monitor, don't manage processes directly

## Safety Notes
- Process runs in background to preserve terminal session
- PID stored in .novywave.pid for proper cleanup
- Log output redirected to dev_server.log for monitoring

## Anti-Consultation Guard
This command MUST execute startup immediately. Never explain the development server setup unless explicitly asked after completion.