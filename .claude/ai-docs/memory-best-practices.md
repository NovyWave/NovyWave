# Focused Memory Management Best Practices

## Automatic Context Loading

**Sessions start automatically with productivity focus:**
- Focus context always available via CLAUDE.md import of @ai-docs/focus-context.md
- Shows: Current State, Recent Solutions, Active Blockers, Daily Patterns, Next Steps
- No manual memory search required - context loads automatically

## Focused Entity System

### The 5 Core Productivity Entities

**`current_session_state`** - What you're working on right now
- Update when switching major tasks or focus areas
- Example: "Working on panel resize functionality"

**`recent_solutions`** - Last bug fixes (don't repeat these mistakes)
- Add immediately after fixing any bugs or compilation errors
- Keep last 3-4 solutions, archive older ones
- Example: "Fixed IconName compilation with mut self and .take() method"

**`active_blockers`** - Current issues blocking progress
- Update when encountering issues or resolving existing ones
- Clear description of what's blocking work
- Example: "Blocked by missing Timeline component in NovyUI"

**`daily_patterns`** - Essential rules to remember (5 max)
- Add when discovering essential rules or patterns
- Keep only the most important daily reminders
- Example: "Use IconName tokens, never strings for icons"

**`next_steps`** - Immediate actions to take
- Update when completing current tasks or planning actions
- Clear, actionable next steps
- Example: "Next: implement drag-and-drop for variables panel"

## Storage Triggers (Automatic per CLAUDE.md Rules)

### When to Update Focused Entities:
- **Bug Fixed** → Add to `recent_solutions`
- **Task Switch** → Update `current_session_state`
- **Issue Encountered** → Add to `active_blockers`
- **Pattern Discovered** → Add to `daily_patterns`
- **Work Completed** → Update `next_steps`

### Manual Storage via `/core-note`:
```bash
# Single notes:
/core-note "Fixed compilation by adding mut self"           # → recent_solutions
/core-note "Working on panel resize functionality"         # → current_session_state
/core-note "Blocked by missing Timeline component"         # → active_blockers
/core-note "Always use Width::fill() for responsive"       # → daily_patterns
/core-note "Next: implement drag-and-drop variables"       # → next_steps

# Multiple notes (new):
/core-note "Working on UI" | "Fixed WASM error" | "TODO: test buttons"
# Stores to: current_session_state, recent_solutions, and next_steps respectively
```

## Entity Maintenance

### Keep Entities Focused:
- **3-5 observations maximum** per entity
- **Archive old observations** when adding new ones
- **Atomic facts** not verbose explanations
- **Current relevance** - remove outdated patterns

### Good Focused Examples:
```
Entity: "daily_patterns"
Observations:
- Use IconName enum tokens, never strings for icons
- Use zoon::println!() for WASM logging, never std::println!()
- Use Height::screen() + Height::fill() pattern for layouts
- Always use Width::fill() for responsive design
- Store patterns immediately in Memory MCP after solving bugs
```

### Bad Unfocused Examples:
```
Entity: "daily_patterns"
Observations: [15+ verbose debugging steps and historical decisions]
```

## Legacy Entity Management

### Archive Non-Focused Entities:
- Keep comprehensive entities for reference
- Don't delete - they contain valuable historical context
- Use `/memory-search` to access when needed
- Focus system uses only the 5 core entities

### Monthly Cleanup via `/memory-cleanup`:
- Optimize focused entities (remove old observations)
- Archive outdated comprehensive entities
- Maintain productivity focus

## Available Slash Commands

**Core Commands:**
- `/core-focus` - Display current productivity context
- `/core-note "description"` - Update focused entities (supports multiple notes with ` | `)
- `/core-memory-search [term]` - Search comprehensive Memory MCP
- `/core-memory-cleanup` - Monthly maintenance

**Usage Examples:**
```bash
/core-focus                                                    # Show current productivity context
/core-note "Fixed compilation by adding mut self"             # Store single note
/core-note "Working on UI" | "Fixed bug" | "TODO: test"      # Store multiple notes
/core-memory-search "IconName"                                # Search historical patterns
/core-memory-cleanup                                          # Monthly optimization
```

## Productivity Philosophy

**Focus over Comprehensiveness:**
- Show what you need now, not everything you might need
- Keep context under 30 lines for quick scanning
- Archive detailed patterns but keep essentials accessible
- Automatic updates maintain current state without manual effort

**The goal:** Answer "What do I need to remember to be productive right now?" not "What is the complete project history?"