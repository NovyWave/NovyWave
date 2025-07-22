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
The system maintains 6 focused entities for maximum productivity:

- **`current_session_state`** - What you're working on right now (1 item, overwrites)
- **`recent_solutions`** - Last bug fixes (don't repeat these mistakes) (5 max, smart archiving)
- **`active_blockers`** - Current issues blocking progress (5 max, resolved → recent_solutions)
- **`daily_patterns`** - Essential rules to remember (5 max, all archived to comprehensive storage)
- **`next_steps`** - Immediate actions to take (5 max, enhanced TODO/Plan prefixes)
- **`session_planning`** - Long-form planning and analysis (5 max, archived to archived_planning)

### Why Focus Context File Exists
Instead of overwhelming Claude with comprehensive project data, the focus context provides exactly what's needed for productivity:

- **Current State:** "Working on command system optimization"
- **Recent Solutions:** "Fixed hook system, NDJSON parsing, etc."
- **Current Blockers:** "None" or specific issues
- **Daily Patterns:** "IconName tokens, zoon::println!(), etc."
- **Next Steps:** "Test focused system, continue UI work"

## 🤖 Strategic Subagent Usage

**CRITICAL for extending session length 2-3x:**

Claude uses Task tool subagents extensively for context conservation. Instead of reading files and doing implementation work directly (which burns context fast), Claude delegates to subagents who use their own context space.

**What gets delegated to subagents:**
- File analysis & research (instead of Claude reading multiple files)
- Implementation tasks (code changes, testing, debugging) 
- Investigation work (finding patterns, analyzing codebases)
- Complex searches across many files

**Benefits:**
- Subagents use separate context space, preserving main session
- Claude gets condensed summaries instead of raw file contents
- Can parallelize multiple research/implementation tasks
- Sessions last 2-3x longer with higher quality work

**Example:** Instead of Claude reading 5 files to understand TreeView patterns, a subagent researches and returns key insights summary.

**See:** `/core-subagent-guide` for detailed patterns and workflows.

## 📋 Available Slash Commands

### Development Commands

#### `/project-start`
**Start development server:**
- Uses atomic file locking (`flock`) to prevent multiple instances
- Kills any existing server processes to ensure clean startup
- Compiles Rust/WASM frontend and starts MoonZoon server
- Monitors compilation progress with background logging
- Waits for successful compilation before reporting ready
- Shows current server status if already running
- Server runs at http://localhost:8080 (or port from MoonZoon.toml)

**Technical Details:**
- Uses `flock` on `dev_server.log` to prevent corruption from multiple instances
- Detects orphaned processes and cleans them up automatically
- Reads port configuration from `MoonZoon.toml` dynamically
- Runs `makers start` in background with output logging
- Safe to run multiple times - handles existing processes gracefully

#### `/project-stop`
**Stop development server:**
- Uses atomic file locking to prevent conflicts during shutdown
- Reliably kills all makers, mzoon, and backend processes
- Port-based detection using `lsof` for foolproof process identification
- Frees configured port and cleans up dev_server.log
- Shows confirmation of complete shutdown with process count

**Technical Details:**
- Reads port from `MoonZoon.toml` instead of hardcoding 8080
- Uses `lsof -ti:PORT` for precise process identification
- Handles multiple PIDs returned by lsof correctly
- Removes lock file and log file for clean state
- Graceful handling of already-stopped servers

### Memory Management Commands

#### `/core-focus` 
**Display current productivity context:**
- Shows current work state, recent solutions, blockers, patterns, next steps
- Perfect for session starts, task switching, or returning from breaks
- Read-only overview of what you need to be productive

#### `/core-note "description"`
**Add discoveries to focused entities with smart archiving:**
- Bug fixes: `/core-note "Fixed compilation by adding mut self"` → recent_solutions
- Essential patterns: `/core-note "Always use Width::fill()"` → daily_patterns  
- Work updates: `/core-note "Working on panel resize"` → current_session_state
- Blockers: `/core-note "Blocked by missing Timeline component"` → active_blockers
- Planning: `/core-note "TODO: test focus command"` → next_steps (enhanced to "Test focus command functionality")

**Multiple notes support (new):**
- Separate with ` | ` (pipe with spaces): `/core-note "Working on UI" | "Fixed WASM error" | "TODO: test buttons"`
- Each note stored in appropriate entity based on content
- Perfect for storing multiple discoveries at once

**Smart Archiving:** When entities reach 5 observations:
- `daily_patterns` → archived to `comprehensive_development_patterns`
- `recent_solutions` → important ones to `comprehensive_solutions`
- `active_blockers` → resolved ones to `resolved_blockers`
- `next_steps` → completed ones to `completed_tasks`

#### `/core-todos`
**Display all task-related information from Memory MCP:**
- Shows next steps (immediate actions to take)
- Shows active blockers (current issues)
- Shows incomplete tasks (backlog items)
- Shows recently completed tasks for context
- Perfect for session starts, planning work, or reviewing progress

#### `/core-memory-search [term]`
**Search Memory MCP for specific patterns:**
- `/core-memory-search "IconName"` - Find component-specific context
- `/core-memory-search "compilation"` - Find debugging context  
- `/core-memory-search` - General project context
- Use when you need specific historical patterns

#### `/core-memory-cleanup`
**Monthly maintenance:**
- Optimizes Memory MCP entities (removes outdated observations)
- Keeps focused entities clean with 5 observations max
- Archives old patterns that are no longer relevant

## 🚀 Optimal Claude Code Workflow

### Starting a Session
1. **Optional:** Run `/core-focus` to see current productivity context
2. **Just start coding** - Claude has immediate access to:
   - What you were working on last
   - Recent bug fixes to avoid repeating
   - Current blockers to be aware of
   - Essential daily patterns to follow
   - Next immediate steps to take

### During Development
1. **Work normally** - Claude has focused context loaded
2. **After solving bugs:** Use `/core-note "Fixed X by doing Y"`
3. **When switching tasks:** Use `/core-note "Working on new feature Z"`
4. **When encountering blockers:** Use `/core-note "Blocked by missing component"`
5. **System automatically updates** focused entities per CLAUDE.md rules

### Session Hygiene
- **Store discoveries immediately** using `/core-note`
- **Be specific:** "Fixed IconName compilation with mut self" not "fixed bug"
- **Update work state** when switching major tasks
- **Note blockers** when encountering issues

## 📁 Documentation Structure

### For Humans (docs/)
- Future human documentation goes here

### For Claude Code (.claude/)
**Modular Structure:**
- `core/` - Universal Claude configuration (copy to any project)
- `frameworks/moonzoon/` - MoonZoon-specific patterns (copy to MoonZoon projects)  
- `project/` - NovyWave-specific documentation (rewrite for new project)
- `commands/` - Slash commands with prefixes:
  - `core-*.md` - Universal commands (copy to any project)
  - `project-*.md` - Project-specific commands (customize for each project)

**Auto-Generated:**
- `ai-docs/focus-context.md` - Productivity context (never edit manually)

**Configuration:**
- `ai-docs/` - Remaining AI documentation  
- `hooks/` - Automatic scripts
- `tmp/` - Temporary workspace
- `settings.json` - Hook configuration
- `ai-memory.json` - Memory MCP storage

## 🎯 Best Practices for Effective Collaboration

### Do This:
- **Start sessions with `/core-focus`** to see current productivity context
- **Store discoveries immediately** using `/core-note`
- **Update work state** when switching major focus areas
- **Note blockers** when encountering issues
- **Ask specific questions** - Claude has focused project context

### Don't Do This:
- ~~Ask Claude to "remember previous context"~~ (CLAUDE.md imports focus-context.md)
- ~~Manually edit Memory MCP~~ (use `/core-note` instead)
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
1. Run `/core-focus` to see current productivity context
2. Check if `.claude/ai-docs/focus-context.md` exists and is recent
3. Check if CLAUDE.md is importing the focus file correctly

### If Memory Gets Unfocused:
1. Run `/core-memory-cleanup` to optimize focused entities
2. Review focused entities have 5 observations max
3. Use `/core-note` to update current work state

### If Slash Commands Don't Work:
1. Verify files exist in `.claude/commands/` with proper prefixes (core-*, project-*)
2. Check you're using Claude Code CLI, not web interface
3. Ensure you're in the project directory

## 📊 Current System Status

**Focused Productivity System:**
- CLAUDE.md: Modular import structure with core/framework/project separation
- Memory entities: 6 focused entities (5 observations max each) + comprehensive archives
- AI documentation: Organized in modular .claude/ structure
- Automation: Automatic focus context generation and memory storage
- Context: Focused 30-line productivity overview, not comprehensive data dump
- Commands: /project-start, /project-stop for development; /core-focus, /core-note, /core-memory-search, /core-memory-cleanup for memory

## 🚀 Migrating to New Projects

The modular structure makes Claude Code configuration portable and reusable.

### Quick Migration for MoonZoon Projects (2 minutes)

```bash
# 1. Copy reusable layers
cp -r existing-project/.claude/core new-project/.claude/core
cp -r existing-project/.claude/frameworks new-project/.claude/frameworks
cp existing-project/.claude/commands/core-* new-project/.claude/commands/

# 2. Copy and customize project commands
cp existing-project/.claude/commands/project-*.md new-project/.claude/commands/
# Edit: Change "NovyWave" → "NewProject", update ports if needed

# 3. Create project-specific files
echo "# NewProject Configuration" > new-project/PROJECT.md
mkdir -p new-project/.claude/project/
# Add project-specific documentation

# 4. Create CLAUDE.md assembly
cat > new-project/CLAUDE.md << 'EOF'
# CLAUDE.md

<!-- Core System Layer -->
@.claude/core/SYSTEM.md
@.claude/core/memory-management.md
@.claude/core/mcp-tools.md
@.claude/core/development.md

<!-- Framework Layer -->
@.claude/frameworks/moonzoon/FRAMEWORK.md
@.claude/frameworks/moonzoon/patterns.md
@.claude/frameworks/moonzoon/debugging.md

<!-- Project Layer -->
@PROJECT.md
@.claude/project/custom-patterns.md

<!-- Auto-Generated Context -->
@.claude/ai-docs/focus-context.md
EOF

# 5. Initialize fresh memory
rm -f new-project/.claude/ai-memory.json
```

### Migration for Non-MoonZoon Projects (1 minute)

```bash
# 1. Copy only core (universal)
cp -r existing-project/.claude/core new-project/.claude/core
cp existing-project/.claude/commands/core-* new-project/.claude/commands/

# 2. Create custom project commands
# Write project-start.md and project-stop.md from scratch for your framework

# 3. Create PROJECT.md and CLAUDE.md
# Skip framework layer imports in CLAUDE.md
```

### What Gets Migrated vs Customized

**Always Copy (Universal):**
- `.claude/core/` - All files (Claude behavior, memory management, MCP tools)
- `core-*.md` commands - Focus, note, memory search/cleanup

**Copy for Same Framework:**
- `.claude/frameworks/moonzoon/` - Framework-specific patterns

**Always Customize:**
- `PROJECT.md` - Completely rewrite for new project
- `project-*.md` commands - Update project names, ports, build commands
- `.claude/project/` - Project-specific documentation
- `CLAUDE.md` - Update imports for your project structure

### Benefits

✅ **2-minute setup** for new MoonZoon projects  
✅ **1-minute setup** for any-framework projects  
✅ **Consistent Claude behavior** across all projects  
✅ **Easy updates** - pull latest core improvements  
✅ **Clear boundaries** - know exactly what to customize

---

The system is designed to keep you productive by providing exactly what you need to know right now, not everything that could possibly be relevant. Just code, ask questions, and store discoveries - the focus context maintains itself automatically.