# Note - Store discoveries in focused Memory MCP (storage only)

Store important discoveries in the focused Memory MCP entities for productivity tracking.
This command only stores notes - it does not execute any commands mentioned in the note.

**Usage examples:** `/note "Fixed compilation error"`

## What it does:

**Updates focused entities with new discoveries:**

1. **Automatic Entity Selection:**
   - **recent_solutions:** Bug fixes and compilation solutions
   - **daily_patterns:** Essential rules and patterns to remember
   - **current_session_state:** Update what you're working on
   - **active_blockers:** Add new blockers or mark as resolved
   - **next_steps:** Update immediate next actions

2. **Smart Storage:**
   - Adds to existing focused entities (no new entities created)
   - Keeps entities clean with 3-5 observations max
   - Archives old observations automatically
   - Maintains productivity focus

## Quick Examples:

```bash
/note "Fixed compilation by adding mut self"           # → recent_solutions
/note "Always use Width::fill() for responsive"       # → daily_patterns  
/note "Working on panel resize functionality"         # → current_session_state
/note "Blocked by missing Timeline component"         # → active_blockers
/note "Next: implement drag-and-drop for variables"   # → next_steps
/note "TODO: test /focus command later"                # → next_steps (stored as-is)
/note "Plan: run makers build tomorrow"                # → next_steps (stored as-is)
```

**Confirmation format:**
```
✓ Stored note in FOCUSED current_session_state: "i'll test /focus to see this note"
✓ Stored note in FOCUSED recent_solutions: "Fixed compilation by adding mut self"
✓ Stored note in library_examples: "Fast2D circle rendering example"  # non-focused entity
```

Note: "FOCUSED" appears only for the 5 productivity entities (current_session_state, recent_solutions, active_blockers, daily_patterns, next_steps)

**Perfect timing:**
- Right after fixing any bug or compilation error
- When you discover essential patterns to remember daily
- When switching focus to new features
- When encountering blockers
- When planning next immediate steps