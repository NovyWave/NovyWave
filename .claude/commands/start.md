---
allowed-tools: Bash(*), mcp__browsermcp__browser_navigate
description: Start NovyWave development server with auto-reload
---

## Current Status
- Check server status: !`pgrep -f "makers start" > /dev/null 2>&1 && echo "Running" || echo "Stopped"`
- Port 8080 status: !`netstat -tlnp 2>/dev/null | grep :8080 | awk '{print $7}' | cut -d'/' -f2 || echo "free"`

## Your Task
Start the NovyWave development server:

1. **If already running**: Just open browser at http://localhost:8080
2. **If not running**: 
   - Clean up old log: `rm -f dev_server.log`
   - Start server: `makers start > dev_server.log 2>&1 &`
   - Wait for compilation success (check for "Server is running on" in log)
   - Open browser at http://localhost:8080 when ready

## Important Notes
- Server compiles Rust/WASM frontend with MoonZoon
- Auto-reload enabled for development
- Use `tail -f dev_server.log` to monitor compilation
- Stop with `/stop` command