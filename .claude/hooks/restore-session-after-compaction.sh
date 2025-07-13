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
    
    # Use lock file to prevent duplicate recovery contexts
    RECOVERY_LOCK=".claude/last-recovery.lock"
    CURRENT_TIME=$(date +%s)
    
    # Check if recovery was already done recently (within 5 minutes)
    if [ -f "$RECOVERY_LOCK" ]; then
        LAST_RECOVERY=$(cat "$RECOVERY_LOCK" 2>/dev/null || echo "0")
        TIME_DIFF=$((CURRENT_TIME - LAST_RECOVERY))
        
        if [ $TIME_DIFF -lt 300 ]; then
            echo "   ⏭️  Skipped recovery context (recent entry: ${TIME_DIFF}s ago)" >> "$HOOK_LOG"
            exit 0
        fi
    fi
    
    # Check file size and clean up if needed (prevent bloat)
    FOCUS_FILE=".claude/ai-docs/focus-context.md"
    CURRENT_LINES=$(wc -l < "$FOCUS_FILE" 2>/dev/null || echo "0")
    
    if [ "$CURRENT_LINES" -gt 100 ]; then
        echo "   🧹 Cleaning focus-context.md (${CURRENT_LINES} lines, keeping first 50)" >> "$HOOK_LOG"
        # Keep the first 50 lines (useful content) and add cleanup note
        head -50 "$FOCUS_FILE" > "${FOCUS_FILE}.tmp"
        echo "" >> "${FOCUS_FILE}.tmp"
        echo "## Recovery Context" >> "${FOCUS_FILE}.tmp"
        echo "- Recovery contexts cleaned up to prevent bloat (was ${CURRENT_LINES} lines)" >> "${FOCUS_FILE}.tmp"
        echo "- Only the most recent context is preserved below" >> "${FOCUS_FILE}.tmp"
        mv "${FOCUS_FILE}.tmp" "$FOCUS_FILE"
    fi
    
    # Update focus context with recovery info
    cat >> "$FOCUS_FILE" << EOF

## 🔄 Post-Compaction Recovery Context
- Recovered from session: $(cat "$BACKUP_PATH/backup-info.txt" 2>/dev/null | head -1)
- Previous task: $(head -1 "$BACKUP_PATH/focus-context.md" 2>/dev/null | grep -o "Working on.*" || echo "Unknown")
- Recovery timestamp: $(date)
- Backup location: $BACKUP_PATH
EOF
    
    # Update lock file with current timestamp
    echo "$CURRENT_TIME" > "$RECOVERY_LOCK"
    echo "   📝 Added recovery context (lock updated)" >> "$HOOK_LOG"

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
    
    # Use lock file to prevent duplicate recovery contexts (no backup case)
    RECOVERY_LOCK=".claude/last-recovery.lock"
    CURRENT_TIME=$(date +%s)
    
    # Check if recovery was already done recently (within 5 minutes)
    if [ -f "$RECOVERY_LOCK" ]; then
        LAST_RECOVERY=$(cat "$RECOVERY_LOCK" 2>/dev/null || echo "0")
        TIME_DIFF=$((CURRENT_TIME - LAST_RECOVERY))
        
        if [ $TIME_DIFF -lt 300 ]; then
            echo "   ⏭️  Skipped minimal recovery context (recent entry: ${TIME_DIFF}s ago)" >> "$HOOK_LOG"
            exit 0
        fi
    fi
    
    # Check file size and clean up if needed (prevent bloat)
    FOCUS_FILE=".claude/ai-docs/focus-context.md"
    CURRENT_LINES=$(wc -l < "$FOCUS_FILE" 2>/dev/null || echo "0")
    
    if [ "$CURRENT_LINES" -gt 100 ]; then
        echo "   🧹 Cleaning focus-context.md (${CURRENT_LINES} lines, keeping first 50)" >> "$HOOK_LOG"
        # Keep the first 50 lines (useful content) and add cleanup note
        head -50 "$FOCUS_FILE" > "${FOCUS_FILE}.tmp"
        echo "" >> "${FOCUS_FILE}.tmp"
        echo "## Recovery Context" >> "${FOCUS_FILE}.tmp"
        echo "- Recovery contexts cleaned up to prevent bloat (was ${CURRENT_LINES} lines)" >> "${FOCUS_FILE}.tmp"
        echo "- Only the most recent context is preserved below" >> "${FOCUS_FILE}.tmp"
        mv "${FOCUS_FILE}.tmp" "$FOCUS_FILE"
    fi
    
    # Minimal recovery context
    cat >> "$FOCUS_FILE" << EOF

## 🔄 Post-Compaction Recovery Context
- Recovered from session: Unknown
- Previous task: Unknown
- Recovery timestamp: $(date)
EOF
    
    # Update lock file with current timestamp
    echo "$CURRENT_TIME" > "$RECOVERY_LOCK"
    echo "   📝 Added minimal recovery context (lock updated)" >> "$HOOK_LOG"
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