# Working with Claude Code Effectively

This guide explains how to work efficiently with Claude Code in the NovyWave project, including the automated focused productivity system.

## 🎯 Understanding the Focused Productivity System

### Automatic Productivity Context Loading
- **Zero Setup Required:** Claude gets focused work context automatically when you start any session
- **Always Current:** Focus context file contains current work state, recent solutions, and next steps
- **No Manual Work:** You never need to ask Claude to "remember" or load context
- **Hook-Based Updates:** Context refreshes automatically when Memory MCP changes

### How It Works Behind the Scenes
1. **Static Focus File:** `focus-context.md` exists with current productivity context
2. **Memory Sync Hook:** Updates the focus file whenever Memory MCP patterns are modified  
3. **CLAUDE.md Import:** Always includes `@.claude/ai-docs/focus-context.md` so Claude reads it
4. **Auto Updates:** CLAUDE.md rules guide automatic memory updates

### Understanding the Focused Memory Structure
The system maintains 5 focused entities for maximum productivity:

- **`current_session_state`** - What you're working on right now
- **`recent_solutions`** - Last bug fixes (don't repeat these mistakes)
- **`active_blockers`** - Current issues blocking progress  
- **`daily_patterns`** - Essential rules to remember (5 max)
- **`next_steps`** - Immediate actions to take

### Why Focus Context File Exists
Instead of overwhelming Claude with comprehensive project data, the focus context provides exactly what's needed for productivity:

- **Current State:** "Working on command system optimization"
- **Recent Solutions:** "Fixed hook system, NDJSON parsing, etc."
- **Current Blockers:** "None" or specific issues
- **Daily Patterns:** "IconName tokens, zoon::println!(), etc."
- **Next Steps:** "Test focused system, continue UI work"

## 📋 Available Slash Commands

### When to Use Each Command

#### `/focus` 
**Display current productivity context:**
- Shows current work state, recent solutions, blockers, patterns, next steps
- Perfect for session starts, task switching, or returning from breaks
- Read-only overview of what you need to be productive

#### `/note "description"`
**Add discoveries to focused entities:**
- Bug fixes: `/note "Fixed compilation by adding mut self"` → recent_solutions
- Essential patterns: `/note "Always use Width::fill()"` → daily_patterns  
- Work updates: `/note "Working on panel resize"` → current_session_state
- Blockers: `/note "Blocked by missing Timeline component"` → active_blockers
- Planning: `/note "Next: implement drag-and-drop"` → next_steps

#### `/memory-search [term]`
**Search Memory MCP for specific patterns:**
- `/memory-search "IconName"` - Find component-specific context
- `/memory-search "compilation"` - Find debugging context  
- `/memory-search` - General project context
- Use when you need specific historical patterns

#### `/memory-cleanup`
**Monthly maintenance:**
- Optimizes Memory MCP entities (removes outdated observations)
- Keeps focused entities clean with 3-5 observations max
- Archives old patterns that are no longer relevant

## 🚀 Optimal Claude Code Workflow

### Starting a Session
1. **Optional:** Run `/focus` to see current productivity context
2. **Just start coding** - Claude has immediate access to:
   - What you were working on last
   - Recent bug fixes to avoid repeating
   - Current blockers to be aware of
   - Essential daily patterns to follow
   - Next immediate steps to take

### During Development
1. **Work normally** - Claude has focused context loaded
2. **After solving bugs:** Use `/note "Fixed X by doing Y"`
3. **When switching tasks:** Use `/note "Working on new feature Z"`
4. **When encountering blockers:** Use `/note "Blocked by missing component"`
5. **System automatically updates** focused entities per CLAUDE.md rules

### Session Hygiene
- **Store discoveries immediately** using `/note`
- **Be specific:** "Fixed IconName compilation with mut self" not "fixed bug"
- **Update work state** when switching major tasks
- **Note blockers** when encountering issues

## 📁 Documentation Structure

### For Humans (docs/)
- Future human documentation goes here

### For Claude Code (.claude/)
- `working-with-claude.md` - This guide for humans
- `ai-docs/focus-context.md` - Auto-generated productivity context (never edit manually)
- `ai-docs/development-workflow.md` - WASM compilation, testing patterns
- `ai-docs/novyui-patterns.md` - Component API, layout patterns  
- `ai-docs/zoon-framework-patterns.md` - Framework fundamentals
- `ai-docs/memory-best-practices.md` - Memory management rules
- `ai-docs/memory-mcp.md` / `ai-docs/browser-mcp.md` - MCP server configurations

### File Organization (.claude/)
- `hooks/` - Automatic scripts (update-context-from-memory.sh)
- `commands/` - Slash commands (focus.md, note.md, etc.)
- `ai-docs/` - AI documentation files
- `tmp/` - Temporary workspace for planning documents and working files
- `settings.json` - Hook configuration
- `ai-memory.json` - Memory MCP storage

## 🎯 Best Practices for Effective Collaboration

### Do This:
- **Start sessions with `/focus`** to see current productivity context
- **Store discoveries immediately** using `/note`
- **Update work state** when switching major focus areas
- **Note blockers** when encountering issues
- **Ask specific questions** - Claude has focused project context

### Don't Do This:
- ~~Ask Claude to "remember previous context"~~ (CLAUDE.md imports focus-context.md)
- ~~Manually edit Memory MCP~~ (use `/note` instead)
- ~~Edit `.claude/ai-docs/focus-context.md`~~ (auto-generated from Memory MCP)
- ~~Batch pattern storage~~ (store immediately when discovered)

### Automatic Memory Updates (Per CLAUDE.md Rules):
- **current_session_state** updates when switching major tasks
- **recent_solutions** adds immediately after fixing bugs  
- **active_blockers** updates when encountering/resolving issues
- **daily_patterns** adds when discovering essential rules
- **next_steps** updates when completing tasks or planning actions

## 🔧 Troubleshooting

### If Claude Seems to Lack Focus Context:
1. Run `/focus` to see current productivity context
2. Check if `.claude/ai-docs/focus-context.md` exists and is recent
3. Ensure PostToolUse hook is working (check `.claude/hooks.log`)
4. Verify CLAUDE.md is importing the focus file correctly

### If Memory Gets Unfocused:
1. Run `/memory-cleanup` to optimize focused entities
2. Review focused entities have 3-5 observations max
3. Use `/note` to update current work state

### If Slash Commands Don't Work:
1. Verify files exist in `.claude/commands/`
2. Check you're using Claude Code CLI, not web interface
3. Ensure you're in the project directory

## 📊 Current System Status

**Focused Productivity System:**
- CLAUDE.md: 74 lines with automatic memory update rules
- Memory entities: 5 focused entities (current_session_state, recent_solutions, active_blockers, daily_patterns, next_steps)
- AI documentation: 7 organized files in .claude/ai-docs/
- Automation: PostToolUse hook triggers after Memory MCP usage
- Context: Focused 30-line productivity overview, not comprehensive data dump

The system is designed to keep you productive by providing exactly what you need to know right now, not everything that could possibly be relevant. Just code, ask questions, and store discoveries - the focus context maintains itself automatically.