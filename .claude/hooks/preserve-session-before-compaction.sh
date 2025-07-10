#!/bin/bash
# PreCompact Hook: Preserve Session Before Compaction

source "$(dirname "$0")/shared-functions.sh"
init_hook_env

echo "🔄 PreCompact hook triggered: $(date)" >> "$HOOK_LOG"

# Phase 1: Critical Session State Capture

echo "📋 Capturing critical session state..." >> "$HOOK_LOG"

# 1.1 Current Task Context - Get from current_session_state
FOCUS_CONTEXT=$(grep -A 20 "current_session_state" .claude/ai-docs/focus-context.md 2>/dev/null | head -10)
update_memory_mcp "compaction_survival_context" "Session focus: $FOCUS_CONTEXT"

# 1.2 Working Files Context - Comprehensive file tracking
WORKING_FILES=$(git diff --name-only HEAD~10..HEAD 2>/dev/null | tr '\n' ',' | sed 's/,$//')
STAGED_FILES=$(git diff --cached --name-only 2>/dev/null | tr '\n' ',' | sed 's/,$//')
MODIFIED_FILES=$(git diff --name-only 2>/dev/null | tr '\n' ',' | sed 's/,$//')
update_memory_mcp "compaction_survival_files" "Recent: $WORKING_FILES | Staged: $STAGED_FILES | Modified: $MODIFIED_FILES"

# 1.3 Active Development State
GIT_BRANCH=$(git branch --show-current 2>/dev/null)
LAST_COMMITS=$(git log --oneline -5 2>/dev/null | tr '\n' '|')
DEV_SERVER_STATUS=$(pgrep -f "makers start" >/dev/null && echo "running" || echo "stopped")
update_memory_mcp "compaction_survival_dev_state" "Branch: $GIT_BRANCH | Server: $DEV_SERVER_STATUS | Recent commits: $LAST_COMMITS"

# 1.4 Critical Patterns from Memory MCP
RECENT_SOLUTIONS=$(grep -A 5 "recent_solutions" .claude/ai-docs/focus-context.md 2>/dev/null | head -5)
ACTIVE_BLOCKERS=$(grep -A 5 "active_blockers" .claude/ai-docs/focus-context.md 2>/dev/null | head -5)
DAILY_PATTERNS=$(grep -A 5 "daily_patterns" .claude/ai-docs/focus-context.md 2>/dev/null | head -5)
update_memory_mcp "compaction_survival_patterns" "Solutions: $RECENT_SOLUTIONS | Blockers: $ACTIVE_BLOCKERS | Patterns: $DAILY_PATTERNS"

# 1.5 Project Configuration State
CONFIG_STATE=$(cat .novywave 2>/dev/null | head -10 | tr '\n' ' ')
CLAUDE_MD_UPDATES=$(tail -10 CLAUDE.md 2>/dev/null | tr '\n' ' ')
update_memory_mcp "compaction_survival_config" "Config: $CONFIG_STATE | CLAUDE.md: $CLAUDE_MD_UPDATES"

# 1.6 Technical Environment State
NODEJS_VERSION=$(node --version 2>/dev/null || echo "unknown")
RUST_VERSION=$(rustc --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
MZOON_VERSION=$(mzoon --version 2>/dev/null || echo "unknown")
update_memory_mcp "compaction_survival_environment" "Node: $NODEJS_VERSION | Rust: $RUST_VERSION | MZoon: $MZOON_VERSION"

# Phase 2: Knowledge Pattern Archive
echo "📚 Archiving knowledge patterns..." >> "$HOOK_LOG"

# Extract comprehensive patterns from CLAUDE.md
CLAUDE_MD_PATTERNS=$(cat CLAUDE.md 2>/dev/null | head -50 | tr '\n' ' ')
update_memory_mcp "compaction_survival_claude_md" "Core patterns: $CLAUDE_MD_PATTERNS"

# Framework-specific patterns
FRAMEWORK_PATTERNS="MoonZoon+Zoon UI, NovyWave waveform viewer, shared crate for types, never restart server without permission, use signal chains not Timer::sleep, IconName enum not strings"
update_memory_mcp "compaction_survival_framework" "$FRAMEWORK_PATTERNS"

# Development patterns from recent work
DEV_PATTERNS="Two-stage git workflow (checkpoint+commit), Memory MCP focused entities, subagent usage for research, CONFIG_LOADED gates prevent startup saves"
update_memory_mcp "compaction_survival_dev_patterns" "$DEV_PATTERNS"

# Phase 3: User Workflow Preservation
echo "👤 Preserving user workflow..." >> "$HOOK_LOG"

# Communication and workflow preferences
USER_PREFS="Prefers: concise responses, git checkpoint workflow, Memory MCP persistence, subagent delegation, multi-line commit messages, no unnecessary server restarts"
update_memory_mcp "compaction_survival_user_prefs" "$USER_PREFS"

# Project-specific preferences
PROJECT_PREFS="NovyWave: waveform viewer, Rust+WASM, auto-save config gates, virtual list for performance, TreeView components, shared types, signal-based architecture"
update_memory_mcp "compaction_survival_project_prefs" "$PROJECT_PREFS"

# Phase 4: Documentation Reinforcement
echo "📄 Reinforcing critical documentation..." >> "$HOOK_LOG"

# Backup critical files to survival snapshot
mkdir -p .claude/compaction-backup
cp CLAUDE.md .claude/compaction-backup/ 2>/dev/null
cp .claude/ai-docs/focus-context.md .claude/compaction-backup/ 2>/dev/null
cp .novywave .claude/compaction-backup/ 2>/dev/null

# Store critical documentation snippets
DOC_CRITICAL="SYSTEM.md: mandatory subagent usage, FRAMEWORK.md: MoonZoon patterns, debugging.md: never restart server, development.md: checkpoint workflow"
update_memory_mcp "compaction_survival_docs" "$DOC_CRITICAL"

# Create compaction survival snapshot
SNAPSHOT_FILE=".claude/compaction-survival-snapshot.json"
cat > "$SNAPSHOT_FILE" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "session_id": "$(date +%s)",
  "critical_context": {
    "current_task": "$CURRENT_TASK",
    "git_branch": "$GIT_BRANCH",
    "modified_files_count": $GIT_STATUS,
    "hook_version": "1.0.0-phase1"
  },
  "status": "PreCompact hook execution completed"
}
EOF

echo "✅ PreCompact preservation completed at $(date)" >> "$HOOK_LOG"
echo "📄 Survival snapshot saved to $SNAPSHOT_FILE" >> "$HOOK_LOG"

# Success exit
exit 0