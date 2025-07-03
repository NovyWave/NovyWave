# Refresh Context - When Claude seems uninformed about recent patterns

Prompt Claude to manually regenerate session context from latest Memory MCP.

**Usage examples:** `/refresh-context`

## What this prompts Claude to do:

**Regenerate session-context.md with latest patterns:**

1. **Extract Current Patterns:**
   - Read latest Memory MCP entities and observations
   - Combine with core architecture and workflow rules
   - Generate fresh summary of key development patterns

2. **Update Session File:**
   - Update `ai-docs/session-context.md` with current state
   - Include recent bug fixes, component patterns, framework rules
   - Ensure fresh project knowledge is available

## Quick Examples:

```bash
/refresh-context                   # Prompt context regeneration
```

**Perfect timing:**
- Before important coding sessions
- After storing several patterns with /store-pattern
- When Claude doesn't know about recent discoveries
- If session context feels outdated
- When automatic hooks aren't sufficient