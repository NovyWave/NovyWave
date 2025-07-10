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

## Strategic Subagent Usage for Context Conservation

**MANDATORY: Use Task tool subagents extensively to preserve main session context:**

**Delegate to Subagents:**
- File analysis & research (instead of main session reading multiple files)
- Implementation tasks (code changes, testing, debugging)
- Investigation work (finding patterns, analyzing codebases)
- Complex searches across many files

**CRITICAL SELF-REMINDER:**
BEFORE using Read/Glob/Grep tools, ask: "Could a subagent research this instead?"
- If reading 2+ files → delegate to Task tool
- If searching for patterns → delegate to Task tool  
- If analyzing codebase structure → delegate to Task tool
- Exception: Single specific files (configs, CLAUDE.md)

**Main Session Focus:**
- High-level coordination & planning
- User interaction & decision making
- Architecture decisions & task delegation
- Synthesis of subagent results

**Context Benefits:**
- Subagents use their own context space, not main session's
- Main session gets condensed summaries instead of raw file contents
- Can parallelize multiple research/implementation tasks
- Dramatically extends effective session length (2-3x longer)

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
- Auto-triggered when daily_patterns reaches 5 observations

**recent_solutions** → Smart archiving based on importance
- Keywords: "compilation", "IconName", "zoon", "WASM", "error", "frontend", "backend"
- Important → `comprehensive_solutions` entity
- Trivial → deleted (simple typos, minor tweaks)
- Auto-triggered when recent_solutions reaches 5 observations

**active_blockers** → Resolution flow
- Resolved → move to `recent_solutions`
- Unresolved → remove oldest when adding new
- Auto-cleanup when adding 6th blocker

**next_steps** → Task management
- Completed → `completed_tasks` entity
- Outdated → deleted
- Auto-cleanup when adding 6th step

**session_planning** → Planning archival
- Archived → `archived_planning` entity
- Outdated planning → deleted during cleanup
- Auto-triggered when session_planning reaches 5 observations

## Archive Entity Access

**Comprehensive Archives:**
- `comprehensive_solutions` - Critical bug fixes and architectural solutions
- `comprehensive_development_patterns` - Essential coding patterns and rules
- `completed_tasks` - Historical task completion records
- `archived_planning` - Historical planning and design decisions

**Search Archives:**
```bash
/core-memory-search "IconName"           # Search all entities including archives
/core-memory-search "scrollbar"          # Find layout solutions
/core-memory-search "compilation error"  # Find debugging patterns
```

**Manual Memory Storage:**
```bash
/core-remember-important  # Intelligently store all important session discoveries
```

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