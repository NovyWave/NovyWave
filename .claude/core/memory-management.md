# Memory Management System

## Focused Productivity System

- AUTOMATIC: Hook generates focused productivity context on Memory MCP usage
- Context always available via @.claude/ai-docs/focus-context.md import
- Shows: Current State, Recent Solutions, Active Blockers, Daily Patterns, Next Steps

## The 6 Focused Entities

**`current_session_state`** - What you're working on right now
- Only 1 observation (overwrites previous)
- Update when switching major tasks or focus areas

**`recent_solutions`** - Last bug fixes (don't repeat these mistakes)
- Keep 5 observations maximum
- Add immediately after fixing any bugs or compilation errors
- Smart archiving: Important solutions → `comprehensive_solutions`

**`active_blockers`** - Current issues blocking progress
- Keep 5 observations maximum
- Update when encountering issues or when resolving existing ones
- Resolved blockers → move to `recent_solutions`

**`daily_patterns`** - Essential rules to remember
- Keep 5 observations maximum
- Add when discovering essential rules or patterns to remember
- All archived to `comprehensive_development_patterns`

**`next_steps`** - Immediate actions to take
- Keep 5 observations maximum
- Update when completing current tasks or planning immediate actions
- Enhanced TODO/Plan prefixes for clarity
- Completed tasks → `completed_tasks`

**`session_planning`** - Long-form planning and analysis
- Keep 5 observations maximum
- Store detailed planning, design decisions, strategy analysis
- Use for complex planning that needs persistence across sessions
- Archived to `archived_planning` when limit reached

## Automatic Memory Updates - MANDATORY BEHAVIOR

**CRITICAL: Always update Memory MCP immediately and proactively as you work. Never wait for user commands.**

Update entities immediately when:
- Solving bugs → recent_solutions
- Switching tasks → current_session_state  
- Finding patterns → daily_patterns
- Encountering blockers → active_blockers
- Planning actions → next_steps
- Complex planning → session_planning

**Examples of required proactive updates:**
- Fix compilation error → immediately store in recent_solutions
- Change focus from UI to backend → update current_session_state
- Discover "always use Width::fill()" → store in daily_patterns
- Can't find component → add to active_blockers
- Plan next implementation step → update next_steps
- Design architecture → store in session_planning

**Never require user to use /core-note or manual commands - Claude must be completely proactive with memory management.**

## Smart Archiving Rules

When entities reach 5 observations:

**daily_patterns** → Archived to `comprehensive_development_patterns`
- Preserves all valuable patterns discovered
- Never loses hard-learned lessons

**recent_solutions** → Smart archiving based on importance
- Keywords: "compilation", "IconName", "zoon", "WASM", "error", "frontend", "backend"
- Important → `comprehensive_solutions`
- Trivial → deleted

**active_blockers** → Resolution flow
- Resolved → move to `recent_solutions`
- Unresolved → remove oldest when adding new

**next_steps** → Task management
- Completed → `completed_tasks`
- Outdated → deleted

**session_planning** → Planning archival
- Archived → `archived_planning`
- Outdated planning → deleted during cleanup

## Memory MCP Best Practices

- Keep focused entities with 5 observations maximum
- Store discoveries immediately using focused entity types
- Archive old observations to maintain productivity focus
- Never create new entities (except comprehensive archives)

## Planning and Analysis

- Store all planning in Memory MCP using `session_planning` entity
- Use `/core-note` for both short insights and long-form planning
- Complex analysis and design decisions go in `session_planning`
- All planning is searchable and persistent across sessions
- No temporary files needed - Memory MCP handles everything