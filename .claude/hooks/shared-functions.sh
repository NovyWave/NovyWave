#!/bin/bash
# Shared functions for Claude Code hooks

# Auto-detect project root by looking for marker files
detect_project_root() {
    local current_dir="$(pwd)"
    
    while [ "$current_dir" != "/" ]; do
        if [ -f "$current_dir/CLAUDE.md" ] || [ -f "$current_dir/Cargo.toml" ] || [ -d "$current_dir/.git" ]; then
            echo "$current_dir"
            return 0
        fi
        current_dir="$(dirname "$current_dir")"
    done
    
    # Fallback to current directory
    echo "$(pwd)"
}

# Initialize common variables
init_hook_env() {
    PROJECT_ROOT=$(detect_project_root)
    cd "$PROJECT_ROOT" || exit 1
    HOOK_LOG=".claude/hooks.log"
    export PROJECT_ROOT HOOK_LOG
}

# Update Memory MCP with survival data
update_memory_mcp() {
    local entity_name="$1"
    local content="$2"
    
    echo "  📝 $entity_name: $content" >> "$HOOK_LOG"
    
    local timestamp=$(date -Iseconds)
    local observation_json=$(cat << EOF
{"type":"observation","entityName":"$entity_name","contents":["PRECOMPACT_$timestamp: $content"]}
EOF
)
    
    echo "$observation_json" >> ".claude/ai-memory.json"
}