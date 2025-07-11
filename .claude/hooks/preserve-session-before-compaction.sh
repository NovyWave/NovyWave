#!/bin/bash
# PreCompact Hook: Smart Session Preservation with Proper Separation

source "$(dirname "$0")/shared-functions.sh"
init_hook_env

echo "🔄 PreCompact hook triggered: $(date)" >> "$HOOK_LOG"

# =============================================================================
# PHASE 1: CRITICAL MEMORY MCP STORAGE (< 500 chars per observation)
# Only store essential patterns that must survive compaction
# =============================================================================

echo "📋 Phase 1: Critical Memory MCP preservation..." >> "$HOOK_LOG"

# 1.1 Current Task Focus (essential for session continuation)
CURRENT_TASK=$(grep -A 2 "Current State:" .claude/ai-docs/focus-context.md 2>/dev/null | tail -1 | head -c 400)
if [ ! -z "$CURRENT_TASK" ]; then
    update_memory_mcp "compaction_survival_focus" "Task: $CURRENT_TASK"
fi

# 1.2 Active Blockers (critical for immediate attention)
ACTIVE_BLOCKERS=$(grep -A 3 "Current Blockers:" .claude/ai-docs/focus-context.md 2>/dev/null | tail -2 | tr '\n' ' ' | head -c 400)
if [ ! -z "$ACTIVE_BLOCKERS" ] && [ "$ACTIVE_BLOCKERS" != "- None" ]; then
    update_memory_mcp "compaction_survival_blockers" "Blockers: $ACTIVE_BLOCKERS"
fi

# 1.3 Development State (essential context)
GIT_BRANCH=$(git branch --show-current 2>/dev/null)
DEV_SERVER_STATUS=$(pgrep -f "makers start" >/dev/null && echo "running" || echo "stopped")
THEME_STATUS=$(grep 'theme = ' .novywave 2>/dev/null | cut -d'"' -f2)
update_memory_mcp "compaction_survival_state" "Branch: $GIT_BRANCH | Server: $DEV_SERVER_STATUS | Theme: $THEME_STATUS"

# 1.4 Recent Critical Solutions (last 2 only - most important)
RECENT_SOLUTIONS=$(grep -A 4 "Recent Solutions" .claude/ai-docs/focus-context.md 2>/dev/null | tail -2 | head -c 400)
if [ ! -z "$RECENT_SOLUTIONS" ]; then
    update_memory_mcp "compaction_survival_solutions" "Recent: $RECENT_SOLUTIONS"
fi

# =============================================================================
# PHASE 2: DETAILED BACKUP STORAGE (separate files, unlimited size)
# Store comprehensive data for manual recovery if needed
# =============================================================================

echo "📁 Phase 2: Comprehensive backup to separate files..." >> "$HOOK_LOG"

# Create timestamped backup directory
BACKUP_DIR=".claude/compaction-backups/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# 2.1 Session Context Backup
cp .claude/ai-docs/focus-context.md "$BACKUP_DIR/focus-context.md" 2>/dev/null
echo "Session backup created: $BACKUP_DIR" >> "$HOOK_LOG"

# 2.2 Git State Backup
git status --porcelain > "$BACKUP_DIR/git-status.txt" 2>/dev/null
git log --oneline -10 > "$BACKUP_DIR/git-history.txt" 2>/dev/null
git diff --name-only > "$BACKUP_DIR/modified-files.txt" 2>/dev/null

# 2.3 Config State Backup
cp .novywave "$BACKUP_DIR/config.toml" 2>/dev/null
echo "$(date): PreCompact backup" > "$BACKUP_DIR/backup-info.txt"

# 2.4 Development Environment Snapshot
echo "Node: $(node --version 2>/dev/null)" > "$BACKUP_DIR/environment.txt"
echo "Rust: $(rustc --version 2>/dev/null)" >> "$BACKUP_DIR/environment.txt"
echo "MZoon: $(mzoon --version 2>/dev/null)" >> "$BACKUP_DIR/environment.txt"

# =============================================================================
# PHASE 3: CLEANUP OLD BACKUPS (keep last 5 only)
# =============================================================================

echo "🧹 Phase 3: Cleanup old backups..." >> "$HOOK_LOG"

# Keep only last 5 backup directories
cd .claude/compaction-backups 2>/dev/null
if [ $? -eq 0 ]; then
    ls -1t | tail -n +6 | xargs -r rm -rf
    BACKUP_COUNT=$(ls -1 | wc -l)
    echo "Backup cleanup: kept $BACKUP_COUNT recent backups" >> "$HOOK_LOG"
    cd - >/dev/null
fi

# =============================================================================
# COMPLETION
# =============================================================================

echo "✅ PreCompact completed: $(date)" >> "$HOOK_LOG"
echo "   📝 Memory MCP: Essential patterns stored (< 2KB total)" >> "$HOOK_LOG"
echo "   📁 File backup: $BACKUP_DIR" >> "$HOOK_LOG"
echo "   🧹 Cleanup: Old backups removed" >> "$HOOK_LOG"

exit 0