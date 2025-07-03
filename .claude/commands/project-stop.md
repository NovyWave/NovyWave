---
allowed-tools: Bash(*)
description: Stop NovyWave development server and clean up resources
---

## Current Status
- Makers processes: !`pgrep -f "makers start" | wc -l`
- Mzoon processes: !`pgrep -f "mzoon" | wc -l`
- Port 8080 usage: !`netstat -tlnp 2>/dev/null | grep :8080 | awk '{print $7}' | cut -d'/' -f2 || echo "free"`

## Your Task
Stop the NovyWave development server:

1. **Kill all related processes**:
   - `pkill -f "makers start"`
   - `pkill -f "mzoon"`
   - Kill any process using port 8080

2. **Clean up resources**:
   - Remove `dev_server.log` if it exists
   - Wait 2 seconds, then force kill any remaining processes

3. **Verify shutdown**:
   - Check no makers/mzoon processes remain
   - Confirm port 8080 is free
   - Show final status

## Important Notes
- Use graceful shutdown first, then force kill if needed
- Always verify complete shutdown
- Clean up log files to prevent confusion