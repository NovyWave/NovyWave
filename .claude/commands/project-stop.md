---
allowed-tools: Bash(*)
description: Stop NovyWave development server (brutal shutdown)
---

Stop NovyWave development server brutally and clean up files.

```bash
echo "Stopping NovyWave development server..."

# Read port from MoonZoon.toml and kill process using that port
PORT=$(grep "^port = " MoonZoon.toml | head -1 | cut -d' ' -f3)
if [ -n "$PORT" ]; then
    # Handle multiple PIDs returned by lsof (space-separated)
    for PID in $(lsof -ti:$PORT 2>/dev/null); do
        kill -9 $PID 2>/dev/null && echo "Killed process $PID using port $PORT"
    done
fi

# Kill all related processes directly
pkill -9 -f "makers" || true
pkill -9 -f "mzoon" || true

# Remove log file
rm -f dev_server.log
echo "Removed dev_server.log"

echo "Server stopped brutally"
exit 0
```

Simple and brutal: kill all processes, remove files.