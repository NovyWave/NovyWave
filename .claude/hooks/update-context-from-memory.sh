#!/bin/bash

# Quick update context after Memory MCP usage
CONTEXT_FILE="./.claude/ai-docs/focus-context.md"
MEMORY_FILE="./.claude/ai-memory.json"
TIMESTAMP=$(date)

# Only regenerate if file missing - avoid timestamp noise in git
if [ -f "$CONTEXT_FILE" ]; then
    # File exists - skip update to avoid git noise from timestamp changes
    echo "✅ Context file exists - skipping timestamp update to avoid git noise: $TIMESTAMP" >> ./.claude/hooks.log
else
    # File doesn't exist - regenerate focused productivity context
    echo "🔄 Context file missing - regenerating focused context: $TIMESTAMP" >> ./.claude/hooks.log
    
    # Check if Memory MCP file exists
    if [ ! -f "$MEMORY_FILE" ]; then
        echo "❌ Memory MCP file not found: $MEMORY_FILE" >> ./.claude/hooks.log
        exit 1
    fi
    
    # Generate focused context file
    cat > "$CONTEXT_FILE" << EOF
# Auto-Generated Session Context

*Last updated: $TIMESTAMP*

## Recent Work & Focus

EOF

    # Extract productivity-focused data using jq if available
    if command -v jq >/dev/null 2>&1; then
        # Current project state (limit to last 5)
        echo "**Current State:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .name == "current_session_state") | .observations[-5:] | .[] | "- " + .' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- No current session state recorded" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Recent solutions (limit to 5 most recent - updated from 3)
        echo "**Recent Solutions (Don't Repeat):**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .name == "recent_solutions") | .observations[-5:] | .[] | "- " + .' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- No recent solutions recorded" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Active blockers (limit to last 5 - updated from 3)
        echo "**Current Blockers:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .name == "active_blockers") | .observations[-5:] | .[] | "- " + .' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- None" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Daily patterns (limit to 5)
        echo "**Essential Daily Patterns:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .name == "daily_patterns") | .observations[-5:] | .[] | "- " + .' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- No daily patterns recorded" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Next steps (limit to last 5)
        echo "**Next Steps:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .name == "next_steps") | .observations[-5:] | .[] | "- " + .' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- Continue with current implementation" >> "$CONTEXT_FILE"
        
    else
        echo "**Memory MCP Data:**" >> "$CONTEXT_FILE"
        echo "- Install \`jq\` for focused data extraction" >> "$CONTEXT_FILE"
        echo "- Use \`/memory-search\` for specific queries" >> "$CONTEXT_FILE"
    fi

    # Add footer
    echo "" >> "$CONTEXT_FILE"
    echo "*Focused productivity context generated at $TIMESTAMP*" >> "$CONTEXT_FILE"
    
    echo "✅ Session context regenerated with focused data: $TIMESTAMP" >> ./.claude/hooks.log
fi