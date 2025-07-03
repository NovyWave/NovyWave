# Memory Management System

## Focused Productivity System

- AUTOMATIC: Hook generates focused productivity context on Memory MCP usage
- Context always available via @.claude/ai-docs/focus-context.md import
- Shows: Current State, Recent Solutions, Active Blockers, Daily Patterns, Next Steps

## The 5 Focused Entities

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

## Automatic Memory Updates

Update entities immediately when:
- Solving bugs → recent_solutions
- Switching tasks → current_session_state
- Finding patterns → daily_patterns
- Encountering blockers → active_blockers
- Planning actions → next_steps

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

## Memory MCP Best Practices

- Keep focused entities with 5 observations maximum
- Store discoveries immediately using focused entity types
- Archive old observations to maintain productivity focus
- Never create new entities (except comprehensive archives)

## Planning Documents & Temporary Files

- Create planning documents in `.claude/tmp/` folder (not project root)
- Extract key insights to Memory MCP entities for searchable storage
- Use `/memory-cleanup` to review and clean temporary files periodically
- Keep project root clean with only essential files