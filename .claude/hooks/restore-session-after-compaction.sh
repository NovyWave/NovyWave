#!/bin/bash
# PostCompact Recovery Hook: Restore Session After Compaction

source "$(dirname "$0")/shared-functions.sh"
init_hook_env
SESSION_MARKER=".claude/session-recovery-completed"
SURVIVAL_SNAPSHOT=".claude/compaction-survival-snapshot.json"

# Check if recovery already ran in this session
if [ -f "$SESSION_MARKER" ]; then
    # Silent noop - already ran this session
    exit 0
fi

# Check if there's actual compaction survival data to recover
if [ ! -f "$SURVIVAL_SNAPSHOT" ]; then
    # No survival data = fresh start or /clear, not post-compaction
    # Create marker to prevent re-runs but don't log noise
    touch "$SESSION_MARKER"
    exit 0
fi

# Check if survival data is recent (within last 10 minutes = 600 seconds)
SURVIVAL_AGE=$(stat -c %Y "$SURVIVAL_SNAPSHOT" 2>/dev/null || echo 0)
CURRENT_TIME=$(date +%s)
AGE_DIFF=$((CURRENT_TIME - SURVIVAL_AGE))

if [ $AGE_DIFF -gt 600 ]; then
    # Survival data is old - probably not from recent compaction
    touch "$SESSION_MARKER"
    echo "ℹ️ Old survival data found (${AGE_DIFF}s old) - skipping recovery" >> "$HOOK_LOG"
    exit 0
fi

# Mark that recovery is running for this session
touch "$SESSION_MARKER"
echo "🔄 COMPACTION RECOVERY triggered (fresh survival data): $(date)" >> "$HOOK_LOG"

# We already know survival snapshot exists and is recent - proceed with recovery
echo "📋 Compaction survival snapshot found - initiating recovery..." >> "$HOOK_LOG"

# Extract critical context from survival snapshot
PREVIOUS_TASK=$(jq -r '.critical_context.current_task' "$SURVIVAL_SNAPSHOT" 2>/dev/null || echo "Unknown")
SESSION_ID=$(jq -r '.session_id' "$SURVIVAL_SNAPSHOT" 2>/dev/null || echo "Unknown")

# Update Memory MCP with recovery information
RECOVERY_JSON=$(cat << EOF
{"type":"observation","entityName":"current_session_state","contents":["POST_COMPACTION_RECOVERY: Previous task was: $PREVIOUS_TASK (Session: $SESSION_ID)"]}
EOF
)
echo "$RECOVERY_JSON" >> ".claude/ai-memory.json"

# Update focus context with recovery state
echo "" >> ".claude/ai-docs/focus-context.md"
echo "## 🔄 Post-Compaction Recovery Context" >> ".claude/ai-docs/focus-context.md"
echo "- Recovered from session: $SESSION_ID" >> ".claude/ai-docs/focus-context.md"
echo "- Previous task: $PREVIOUS_TASK" >> ".claude/ai-docs/focus-context.md"
echo "- Recovery timestamp: $(date)" >> ".claude/ai-docs/focus-context.md"

# Load survival patterns from ai-memory.json
SURVIVAL_PATTERNS=$(grep "PRECOMPACT_" .claude/ai-memory.json 2>/dev/null | tail -5)
if [ -n "$SURVIVAL_PATTERNS" ]; then
    echo "📚 Restored patterns from pre-compaction:" >> "$HOOK_LOG"
    echo "$SURVIVAL_PATTERNS" >> "$HOOK_LOG"
    
    # Add survival patterns to focus context
    echo "- Survival patterns restored: $(echo "$SURVIVAL_PATTERNS" | wc -l) entries" >> ".claude/ai-docs/focus-context.md"
fi

echo "✅ COMPACTION RECOVERY completed successfully" >> "$HOOK_LOG"

# Archive the survival snapshot after successful recovery
mv "$SURVIVAL_SNAPSHOT" "${SURVIVAL_SNAPSHOT}.recovered-$(date +%s)" 2>/dev/null || true

exit 0