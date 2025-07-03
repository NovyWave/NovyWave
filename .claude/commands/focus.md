# Focus - Display current work focus and productivity context

Show the current state of all focused Memory MCP entities for quick overview.

**Usage examples:** `/focus`

## What it does:

**Displays focused productivity context:**

1. **Current State:** What you're working on right now
2. **Recent Solutions:** Last bug fixes (don't repeat these)
3. **Current Blockers:** Active issues blocking progress
4. **Daily Patterns:** Essential rules to remember
5. **Next Steps:** Immediate actions to take

## Quick Examples:

```bash
/focus                          # Display all focused context
```

**Perfect timing:**
- Start of development sessions
- When you need a quick refresher
- Before switching tasks
- When returning from breaks
- When planning next work

**Output format:**
```
Current State: Working on command system optimization
Recent Solutions: Fixed hook system, NDJSON parsing, etc.
Current Blockers: None
Daily Patterns: IconName tokens, zoon::println!(), etc.
Next Steps: Test focused system, continue UI work
```

**Note:** This is read-only - use `/note` to update the focused entities.