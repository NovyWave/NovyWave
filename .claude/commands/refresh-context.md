# Refresh Context Command

Manually refresh the session context file from Memory MCP patterns.

## What it does:

**Regenerates session-context.md with latest patterns:**

1. **Extract Current Patterns:**
   - Read latest Memory MCP entities and observations
   - Combine with core architecture and workflow rules
   - Generate fresh summary of key development patterns

2. **Update Session File:**
   - Overwrites `ai-docs/session-context.md` with current state
   - Includes recent bug fixes, component patterns, framework rules
   - Ensures Claude has up-to-date project knowledge

## When to use:

- **After major discoveries** but before using Memory MCP tools
- **When Claude seems uninformed** about recent patterns
- **Before important sessions** to ensure fresh context
- **Alternative to** waiting for PostToolUse hook to trigger

## Usage:

```
/project:refresh-context
```

**Note:** This runs the same script as the PostToolUse hook, but manually.