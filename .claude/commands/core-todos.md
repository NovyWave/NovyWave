---
allowed-tools: mcp__memory__open_nodes
description: Display all todos, blockers, and task backlogs from Memory MCP
---

# Todos - Display all todos, blockers, and task backlogs

Show comprehensive task overview from Memory MCP focused entities.

**Usage examples:** `/core-todos`

## What it does:

**Displays all task-related information:**

1. **Next Steps:** Immediate actions to take (up to 5)
2. **Active Blockers:** Current issues blocking progress
3. **Incomplete Tasks:** Backlog of deferred tasks
4. **Recently Completed:** Last 5 completed tasks for context

## Quick Examples:

```bash
/core-todos                     # Display all task lists
```

**Perfect timing:**
- Start of development sessions
- When planning work priorities
- Before creating new tasks
- When reviewing progress
- When context switching

**Output format:**
```
📋 TASK OVERVIEW

Next Steps (5):
  1. Use real TreeView component in Files & Scope panel
  2. Fix Remove All button styles
  3. Check Dock button duplicates
  4. Make Files & Scope panel functional
  5. Create /core-todos command

🚧 Active Blockers (1):
  • Config mismatch: npm-global vs unknown

📦 Backlog (0):
  (No incomplete tasks)

✅ Recently Completed (5/20):
  • Test dock button functionality
  • Test notification system
  • Update working-with-claude.md
  • Fix /project-stop command
  • Enhanced ButtonBuilder API
```

**Note:** This is read-only - use `/core-note` to add new tasks/blockers.