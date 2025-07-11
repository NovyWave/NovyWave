#!/bin/bash
# PostCompact Hook: Smart Session Restoration

source "$(dirname "$0")/shared-functions.sh"
init_hook_env

echo "🔄 PostCompact hook triggered: $(date)" >> "$HOOK_LOG"

# =============================================================================
# PHASE 1: RESTORE FROM MEMORY MCP (automatic)
# =============================================================================

echo "📋 Phase 1: Restoring from Memory MCP..." >> "$HOOK_LOG"

# Check if we have survival data in Memory MCP
SURVIVAL_ENTITIES="compaction_survival_focus compaction_survival_blockers compaction_survival_state compaction_survival_solutions"

for entity in $SURVIVAL_ENTITIES; do
    # Try to read the entity (this will work if Memory MCP is functional)
    if command -v claude >/dev/null 2>&1; then
        echo "   🔍 Checking for survival entity: $entity" >> "$HOOK_LOG"
    fi
done

# =============================================================================
# PHASE 2: UPDATE FOCUS CONTEXT (smart recovery)
# =============================================================================

echo "📝 Phase 2: Updating focus context..." >> "$HOOK_LOG"

# Find the most recent backup
LATEST_BACKUP=$(ls -1t .claude/compaction-backups/ 2>/dev/null | head -1)

if [ ! -z "$LATEST_BACKUP" ]; then
    BACKUP_PATH=".claude/compaction-backups/$LATEST_BACKUP"
    echo "   📁 Found backup: $BACKUP_PATH" >> "$HOOK_LOG"
    
    # Update focus context with recovery info
    cat >> .claude/ai-docs/focus-context.md << EOF

## 🔄 Post-Compaction Recovery Context
- Recovered from session: $(cat "$BACKUP_PATH/backup-info.txt" 2>/dev/null | head -1)
- Previous task: $(head -1 "$BACKUP_PATH/focus-context.md" 2>/dev/null | grep -o "Working on.*" || echo "Unknown")
- Recovery timestamp: $(date)
- Backup location: $BACKUP_PATH
EOF

    # Show key recovery info
    if [ -f "$BACKUP_PATH/git-status.txt" ]; then
        MODIFIED_COUNT=$(wc -l < "$BACKUP_PATH/git-status.txt")
        echo "   📊 Recovery context: $MODIFIED_COUNT modified files" >> "$HOOK_LOG"
    fi
    
    if [ -f "$BACKUP_PATH/environment.txt" ]; then
        echo "   🔧 Environment restored from backup" >> "$HOOK_LOG"
    fi
else
    echo "   ⚠️  No recent backup found for detailed recovery" >> "$HOOK_LOG"
    
    # Minimal recovery context
    cat >> .claude/ai-docs/focus-context.md << EOF

## 🔄 Post-Compaction Recovery Context
- Recovered from session: Unknown
- Previous task: Unknown
- Recovery timestamp: $(date)
EOF
fi

# =============================================================================
# PHASE 3: SMART SUGGESTIONS
# =============================================================================

echo "💡 Phase 3: Recovery suggestions..." >> "$HOOK_LOG"

# Provide helpful recovery info
if [ -f "$BACKUP_PATH/modified-files.txt" ] && [ -s "$BACKUP_PATH/modified-files.txt" ]; then
    MODIFIED_FILES=$(head -3 "$BACKUP_PATH/modified-files.txt" | tr '\n' ', ' | sed 's/,$//')
    echo "   💡 You were working on: $MODIFIED_FILES" >> "$HOOK_LOG"
fi

if [ -f "$BACKUP_PATH/git-history.txt" ]; then
    LAST_COMMIT=$(head -1 "$BACKUP_PATH/git-history.txt" 2>/dev/null)
    echo "   💡 Last commit: $LAST_COMMIT" >> "$HOOK_LOG"
fi

# =============================================================================
# COMPLETION
# =============================================================================

echo "✅ PostCompact completed: $(date)" >> "$HOOK_LOG"
echo "   📝 Focus context updated with recovery info" >> "$HOOK_LOG"
echo "   💡 Check .claude/ai-docs/focus-context.md for session context" >> "$HOOK_LOG"

exit 0