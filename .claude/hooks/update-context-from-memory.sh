#!/bin/bash

# Quick update context after Memory MCP usage
CONTEXT_FILE="./.claude/ai-docs/focus-context.md"
MEMORY_FILE="./.claude/ai-memory.json"
TIMESTAMP=$(date)

# Only update timestamp if file exists, otherwise regenerate focused context
if [ -f "$CONTEXT_FILE" ]; then
    # Update timestamp in existing file
    sed -i "s/\*Last updated:.*\*/\*Last updated: $TIMESTAMP\*/" "$CONTEXT_FILE"
    
    # Log the quick update
    echo "🔄 Context timestamp updated after Memory MCP usage: $TIMESTAMP" >> ./.claude/hooks.log
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
        # Current project state
        echo "**Current State:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .entityType == "current_session_state") | "- " + (.observations | join("\n- "))' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- No current session state recorded" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Recent solutions (limit to 3 most recent)
        echo "**Recent Solutions (Don't Repeat):**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .entityType == "recent_solutions") | .observations[-3:] | .[] | "- " + .' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- No recent solutions recorded" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Active blockers
        echo "**Current Blockers:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .entityType == "active_blockers") | "- " + (.observations | join("\n- "))' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- None" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Daily patterns (essential rules)
        echo "**Essential Daily Patterns:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .entityType == "daily_patterns") | .observations[0:5] | .[] | "- " + .' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- No daily patterns recorded" >> "$CONTEXT_FILE"
        echo "" >> "$CONTEXT_FILE"
        
        # Next steps
        echo "**Next Steps:**" >> "$CONTEXT_FILE"
        jq -r 'select(.type == "entity" and .entityType == "next_steps") | "- " + (.observations | join("\n- "))' "$MEMORY_FILE" >> "$CONTEXT_FILE" 2>/dev/null || echo "- Continue with current implementation" >> "$CONTEXT_FILE"
        
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