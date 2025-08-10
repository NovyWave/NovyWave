#!/bin/bash

# Source shared functions for proper path handling
HOOK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HOOK_DIR/shared-functions.sh"
init_hook_env

# Quick update context from session notes
CONTEXT_FILE="$PROJECT_ROOT/.claude/ai-docs/focus-context.md"
SESSION_NOTES="$PROJECT_ROOT/.claude/session-notes.md"
TIMESTAMP=$(date)

# Only regenerate if file missing - avoid timestamp noise in git
if [ -f "$CONTEXT_FILE" ]; then
    # File exists - skip update to avoid git noise from timestamp changes
    echo "✅ Context file exists - skipping timestamp update to avoid git noise: $TIMESTAMP" >> "$HOOK_LOG"
else
    # File doesn't exist - regenerate focused productivity context
    echo "🔄 Context file missing - regenerating focused context: $TIMESTAMP" >> "$HOOK_LOG"
    
    # Generate focused context file
    cat > "$CONTEXT_FILE" << EOF
# Session Context

*Last updated: $TIMESTAMP*

## Recent Work & Focus

EOF

    # Extract recent session notes if available
    if [ -f "$SESSION_NOTES" ]; then
        echo "**Recent Session Notes:**" >> "$CONTEXT_FILE"
        # Get last 10 lines from session notes
        tail -n 10 "$SESSION_NOTES" >> "$CONTEXT_FILE" 2>/dev/null || echo "- No session notes available" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
    else
        echo "**No Session Notes:**" >> "$CONTEXT_FILE"
        echo "- Use /core-remember-important to store session discoveries" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
    fi

    # Add footer
    echo "" >> "$CONTEXT_FILE"
    echo "*Context generated from session notes at $TIMESTAMP*" >> "$CONTEXT_FILE"
    
    echo "✅ Session context regenerated from session notes: $TIMESTAMP" >> "$HOOK_LOG"
fi