#!/bin/bash
# Memory monitoring hook for Claude Code sessions

source "$(dirname "$0")/shared-functions.sh"
init_hook_env

# Memory thresholds (MB)
MEMORY_WARNING=1200
MEMORY_CRITICAL=1800
SESSION_TIME_WARNING=86400  # 24 hours in seconds

# Get Claude process info
get_claude_info() {
    local claude_pid
    claude_pid=$(pgrep -f "claude" | head -1)
    
    if [ -z "$claude_pid" ]; then
        echo "No Claude process found"
        return 1
    fi
    
    # Get memory usage in MB and session duration
    local mem_kb=$(ps -p "$claude_pid" -o rss= 2>/dev/null | tr -d ' ')
    local session_time=$(ps -p "$claude_pid" -o etime= 2>/dev/null | tr -d ' ')
    
    if [ -z "$mem_kb" ]; then
        echo "Cannot get Claude process info"
        return 1
    fi
    
    local mem_mb=$((mem_kb / 1024))
    
    echo "$claude_pid $mem_mb $session_time"
}

# Convert etime format to seconds
etime_to_seconds() {
    local etime="$1"
    local total_seconds=0
    
    # Handle different formats: SS, MM:SS, HH:MM:SS, DD-HH:MM:SS
    if [[ "$etime" =~ ^([0-9]+)-(.+)$ ]]; then
        # Days format: DD-HH:MM:SS
        local days=${BASH_REMATCH[1]}
        local remaining=${BASH_REMATCH[2]}
        total_seconds=$((days * 86400))
        etime="$remaining"
    fi
    
    # Split by colons
    IFS=':' read -ra parts <<< "$etime"
    local len=${#parts[@]}
    
    if [ "$len" -eq 1 ]; then
        # Just seconds - remove leading zeros
        local seconds=$(echo "${parts[0]}" | sed 's/^0*//')
        [ -z "$seconds" ] && seconds=0
        total_seconds=$((total_seconds + seconds))
    elif [ "$len" -eq 2 ]; then
        # MM:SS - remove leading zeros
        local minutes=$(echo "${parts[0]}" | sed 's/^0*//')
        local seconds=$(echo "${parts[1]}" | sed 's/^0*//')
        [ -z "$minutes" ] && minutes=0
        [ -z "$seconds" ] && seconds=0
        total_seconds=$((total_seconds + minutes * 60 + seconds))
    elif [ "$len" -eq 3 ]; then
        # HH:MM:SS - remove leading zeros
        local hours=$(echo "${parts[0]}" | sed 's/^0*//')
        local minutes=$(echo "${parts[1]}" | sed 's/^0*//')
        local seconds=$(echo "${parts[2]}" | sed 's/^0*//')
        [ -z "$hours" ] && hours=0
        [ -z "$minutes" ] && minutes=0
        [ -z "$seconds" ] && seconds=0
        total_seconds=$((total_seconds + hours * 3600 + minutes * 60 + seconds))
    fi
    
    echo "$total_seconds"
}

# Check memory and time thresholds
check_thresholds() {
    local info
    info=$(get_claude_info)
    
    if [ $? -ne 0 ]; then
        echo "$(date): $info" >> "$HOOK_LOG"
        return 1
    fi
    
    read -r pid mem_mb session_time <<< "$info"
    local session_seconds
    session_seconds=$(etime_to_seconds "$session_time")
    
    echo "$(date): Claude PID=$pid, Memory=${mem_mb}MB, Runtime=${session_time}" >> "$HOOK_LOG"
    
    # Check thresholds
    local warnings=()
    
    if [ "$mem_mb" -gt "$MEMORY_CRITICAL" ]; then
        warnings+=("🚨 CRITICAL: Memory usage ${mem_mb}MB > ${MEMORY_CRITICAL}MB - Consider restarting Claude session")
    elif [ "$mem_mb" -gt "$MEMORY_WARNING" ]; then
        warnings+=("⚠️  WARNING: Memory usage ${mem_mb}MB > ${MEMORY_WARNING}MB - Consider using /clear")
    fi
    
    if [ "$session_seconds" -gt "$SESSION_TIME_WARNING" ]; then
        local hours=$((session_seconds / 3600))
        warnings+=("⏰ Long session: ${hours}h runtime - Context may be degrading")
    fi
    
    # Output warnings
    if [ ${#warnings[@]} -gt 0 ]; then
        for warning in "${warnings[@]}"; do
            echo "$(date): $warning" >> "$HOOK_LOG"
            echo "$warning"
        done
        
        # Store in memory
        update_memory_mcp "active_blockers" "High memory usage (${mem_mb}MB) or long session (${session_time}) causing performance issues"
        
        return 0
    fi
    
    return 1
}

# Manual monitoring mode
if [ "$1" = "check" ]; then
    check_thresholds
elif [ "$1" = "watch" ]; then
    echo "Starting memory monitoring (Ctrl+C to stop)..."
    while true; do
        check_thresholds
        sleep 300  # Check every 5 minutes
    done
else
    # Hook mode - just check once (suppress output, always return 0)
    check_thresholds >/dev/null
    exit 0
fi