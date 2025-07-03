---
allowed-tools: mcp__memory__search_nodes, mcp__memory__create_entities, mcp__memory__add_observations, mcp__memory__delete_observations, mcp__memory__open_nodes
description: 'Store discoveries: /core-note "Fixed blank page by restarting mzoon" or "Working on panel resizing"'
---

## Your Task

Analyze the note: "$ARGUMENTS" and store it in the appropriate focused Memory MCP entity.

### 1. Determine Target Entity

Based on the note content, select the appropriate entity:
- **recent_solutions**: Contains "fixed", "solved", "resolved", "compilation", "error", "bug"
- **daily_patterns**: Contains "always", "never", "use", "avoid", "pattern", "rule"
- **current_session_state**: Contains "working on", "focusing on", "implementing", "current"
- **active_blockers**: Contains "blocked", "stuck", "issue", "problem", "can't"
- **next_steps**: Contains "next", "todo", "plan", "test", "implement", "will"
- **session_planning**: Contains "planning", "strategy", "analysis", "design", "approach", or is longer than 200 characters

### 2. Enhance Content (for next_steps only)

If targeting next_steps and content starts with prefixes, enhance it:
- Remove: "TODO:", "Plan:", "FIXME:", "NOTE:", "Next:"
- Make actionable: Start with a verb, clarify the goal
- Example: "TODO: test /focus" → "Test /focus command functionality"

### 3. Check Entity Size & Archive if Needed

Before adding, check if the entity has 5 observations. If yes:

**For daily_patterns:**
1. Create `comprehensive_development_patterns` entity if it doesn't exist
2. Move the oldest observation to comprehensive entity
3. Add confirmation: "✓ Archived to comprehensive_development_patterns: [old pattern]"

**For recent_solutions:**
1. Check if oldest contains keywords: "compilation", "IconName", "zoon", "WASM", "error", "frontend", "backend"
2. If important: Archive to `comprehensive_solutions` entity
3. If trivial: Just delete it
4. Add appropriate confirmation message

**For active_blockers:**
1. If note contains "resolved", "fixed", "unblocked": Move to recent_solutions
2. Otherwise: Just remove oldest blocker

**For next_steps:**
1. If oldest task seems completed: Archive to `completed_tasks`
2. Otherwise: Just remove it

**For session_planning:**
1. Archive oldest to `archived_planning`
2. Planning always preserved, never deleted

### 4. Add New Observation

Add the new (possibly enhanced) observation to the focused entity.

### 5. Confirm Storage

Show confirmation in format:
- "✓ Stored note in FOCUSED [entity]: '[content]'" 
- Include "(enhanced from '[original]')" if content was enhanced
- Include archiving confirmations if any archiving occurred

## Important
- Never create new entities (except comprehensive archives)
- Maintain exactly 5 observations per focused entity  
- Preserve valuable patterns through smart archiving
- current_session_state is special: only 1 observation (overwrite)
- session_planning handles long-form content and complex analysis

## Quick Examples:

```bash
/note "Fixed compilation by adding mut self"           # → recent_solutions
/note "Always use Width::fill() for responsive"       # → daily_patterns  
/note "Working on panel resize functionality"         # → current_session_state
/note "Blocked by missing Timeline component"         # → active_blockers
/note "Next: implement drag-and-drop for variables"   # → next_steps
/note "TODO: test /focus command later"                # → next_steps ("Test /focus command functionality")
/note "Plan: run makers build tomorrow"                # → next_steps ("Run makers build to verify production")
```

**Confirmation format:**
```
✓ Stored note in FOCUSED current_session_state: "i'll test /focus to see this note"
✓ Stored note in FOCUSED recent_solutions: "Fixed compilation by adding mut self"
✓ Stored note in FOCUSED next_steps: "Test /focus command functionality" (enhanced from "TODO: test /focus")
✓ Archived to comprehensive_development_patterns: "Use IconName enum tokens, never strings"
✓ Stored note in library_examples: "Fast2D circle rendering example"  # non-focused entity
```

Note: "FOCUSED" appears only for the 5 productivity entities (current_session_state, recent_solutions, active_blockers, daily_patterns, next_steps)

**Perfect timing:**
- Right after fixing any bug or compilation error
- When you discover essential patterns to remember daily
- When switching focus to new features
- When encountering blockers
- When planning next immediate steps