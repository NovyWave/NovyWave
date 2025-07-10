# Test Compaction System - Debug utility (Claude will recommend when hook issues occur)

This command tests different session scenarios to validate the smart compaction survival system.

## Test Scenarios:

### 1. **Fresh Start (No Survival Data):**
```bash
# Clean slate
rm -f .claude/session-recovery-completed .claude/compaction-survival-snapshot.json*

# Trigger recovery - should be silent noop
bash ./.claude/hooks/restore-session-after-compaction.sh

# Check result (should be silent)
echo "Recovery result: $(tail -1 .claude/hooks.log)"
```

### 2. **Old Survival Data (Stale):**
```bash
# Create old survival data
echo '{"timestamp":"2025-01-01T00:00:00+00:00","session_id":"old"}' > .claude/compaction-survival-snapshot.json
touch -d "2 hours ago" .claude/compaction-survival-snapshot.json
rm -f .claude/session-recovery-completed

# Trigger recovery - should skip old data
bash ./.claude/hooks/restore-session-after-compaction.sh

# Check result
tail -3 .claude/hooks.log
```

### 3. **Fresh Compaction (Recent Survival Data):**
```bash
# Create fresh survival data
bash ./.claude/hooks/preserve-session-before-compaction.sh
rm -f .claude/session-recovery-completed

# Trigger recovery - should run full recovery
bash ./.claude/hooks/restore-session-after-compaction.sh

# Check results
tail -5 .claude/hooks.log
grep "POST_COMPACTION_RECOVERY" .claude/ai-memory.json | tail -1
```

### 4. **Double Recovery Protection:**
```bash
# Try recovery again - should be silent noop
bash ./.claude/hooks/restore-session-after-compaction.sh

# Check result (should be silent)
echo "Second recovery: $(tail -1 .claude/hooks.log)"
```

### 5. **Session End Cleanup:**
```bash
# Simulate session end
bash ./.claude/hooks/claude-finished-notification.sh >/dev/null 2>&1

# Check cleanup
ls -la .claude/session-recovery-completed 2>/dev/null || echo "✅ Session marker cleaned"
```

This tests all compaction survival scenarios including fresh start, /clear, actual compaction, and session cleanup.