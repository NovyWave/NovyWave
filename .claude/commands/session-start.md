# Session Start Status

Shows the automated session context system status.

## ✅ Fully Automated Context Loading

**What happens automatically:**

1. **Session Start Hook:**
   - Runs on first tool use in any session
   - Generates fresh `ai-docs/session-context.md` with key patterns
   - No manual intervention required

2. **Memory Sync Hook:**
   - Triggers after any Memory MCP tool usage
   - Updates context with latest patterns and solutions
   - Keeps context fresh throughout session

3. **CLAUDE.md Import:**
   - Automatically imports `@ai-docs/session-context.md`
   - Context available immediately at session start
   - Always up-to-date with recent discoveries

**Manual override only needed for:**
- Specific topic deep-dives
- Debugging particular issues

## Usage:

```
/project:session-start [search-term]
```

Examples:
- `/project:session-start` - General NovyWave context
- `/project:session-start "button"` - Component-specific context
- `/project:session-start "error"` - Debugging context

**This should be the first command run in every Claude Code session.**