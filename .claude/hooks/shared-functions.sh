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
    # Prefer git rev-parse for consistency with Claude Code, fallback to detect_project_root
    if command -v git >/dev/null 2>&1 && git rev-parse --show-toplevel >/dev/null 2>&1; then
        PROJECT_ROOT=$(git rev-parse --show-toplevel)
    else
        PROJECT_ROOT=$(detect_project_root)
    fi
    
    cd "$PROJECT_ROOT" || exit 1
    
    # Ensure .claude directory exists
    mkdir -p "$PROJECT_ROOT/.claude"
    
    # Use absolute path for hook log to avoid path resolution issues
    HOOK_LOG="$PROJECT_ROOT/.claude/hooks.log"
    
    # Ensure log file exists
    touch "$HOOK_LOG"
    
    export PROJECT_ROOT HOOK_LOG
}

