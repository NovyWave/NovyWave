# Working with Claude Code Effectively

This guide explains how to work efficiently with Claude Code in the NovyWave project, including the automated memory system and best practices.

## 🧠 Understanding the Memory System

### Automated Project Knowledge Loading
- **Zero Setup Required:** Claude gets project knowledge automatically when you start any session
- **Always Current:** Session context file contains distilled Memory MCP patterns
- **No Manual Work:** You never need to ask Claude to "remember" or load project information
- **Hook-Based Updates:** Context refreshes automatically when Memory MCP changes

### How It Works Behind the Scenes
1. **Static Context File:** `session-context.md` exists with current project patterns 
2. **Memory Sync Hook:** Updates the session file whenever Memory MCP patterns are modified  
3. **CLAUDE.md Import:** Always includes `@ai-docs/session-context.md` so Claude reads it
4. **Manual Refresh:** Use `/project:store-pattern` or `/project:memory-cleanup` to update context

### Understanding the Different "Contexts"
- **Claude's Conversation Context:** Limited token window for our current chat
- **Memory MCP:** Persistent knowledge graph stored in `ai-memory.json`
- **CLAUDE.md Files:** Project instructions Claude reads at session start
- **Session Context File:** Human-readable distillation of Memory MCP for CLAUDE.md import

### Why Session Context File Exists
The Memory MCP stores patterns in a structured knowledge graph format that's optimized for searching and relationships, but not ideal for Claude to read quickly at session start. The session context file acts as a **human-readable cache** that:

- **Distills** key patterns from Memory MCP into readable markdown format
- **Combines** recent discoveries with core architecture patterns
- **Provides** immediate project knowledge without needing tool calls
- **Updates** automatically when Memory MCP changes via hooks
- **Bridges** the gap between structured data storage and session-start accessibility

## 📋 Available Slash Commands

### When to Use Each Command

#### `/memory-cleanup` 
**Use monthly or when feeling context bloat:**
- Optimizes CLAUDE.md from verbose to streamlined
- Consolidates Memory MCP entities (removes redundant observations)
- Moves detailed patterns to organized ai-docs files
- **Result:** Faster session starts, better context efficiency

#### `/session-start`
**Shows status of the automated system:**
- Explains how the session context system works
- **Not needed for loading context** - that's automatic via CLAUDE.md imports
- Use only to understand the system or for troubleshooting

#### `/refresh-context`
**Manually update session context file:**
- Regenerates `ai-docs/session-context.md` from current Memory MCP
- Useful before important sessions to ensure latest patterns included
- Alternative to waiting for PostToolUse hook to trigger

#### `/store-pattern "description"`
**Use immediately after discovering/solving something:**
- Bug fixes: `/store-pattern "Fixed E0382 with mut self pattern"`
- New patterns: `/store-pattern "Responsive layout needs Width::fill()"`
- Architectural decisions: `/store-pattern "Using Stripe for dynamic layouts"`
- **Best Practice:** Store while the solution is fresh in context

#### `/test-hello`
**Testing only** - Verifies slash command system works

## 🚀 Optimal Claude Code Workflow

### Starting a Session
1. **Just start coding** - project knowledge loads automatically via CLAUDE.md
2. **Claude will have immediate access to:**
   - Recent compilation fixes and solutions (from Memory MCP)
   - Component usage patterns (NovyUI, Zoon frameworks)
   - Architectural decisions and framework patterns
   - Development workflow rules and best practices

### During Development
1. **Ask for help normally** - Claude has full project knowledge loaded
2. **After solving bugs:** Use `/store-pattern "what you learned"`
3. **System automatically updates Memory MCP** and regenerates session file

### Session Hygiene
- **Store patterns immediately** - don't batch them using `/store-pattern`
- **Be specific in descriptions:** "Fixed IconName enum" not "fixed bug"
- **Let automation handle the rest** - hooks maintain everything else

## 📁 Documentation Structure

### For Humans (docs/)
- `working-with-claude.md` - This guide
- Future human documentation goes here

### For AI (ai-docs/)
- `session-context.md` - Auto-generated context (never edit manually)
- `development-workflow.md` - WASM compilation, testing patterns
- `novyui-patterns.md` - Component API, layout patterns  
- `zoon-framework-patterns.md` - Framework fundamentals
- `memory-best-practices.md` - Memory management rules
- `memory-mcp.md` / `browser-mcp.md` - MCP server configs

## 🎯 Best Practices for Effective Collaboration

### Do This:
- **Start sessions normally** - no setup ritual needed
- **Store discoveries immediately** using `/store-pattern`
- **Ask specific questions** - Claude has full project context
- **Work naturally** - the memory system is invisible when working correctly

### Don't Do This:
- ~~Ask Claude to "remember previous context"~~ (CLAUDE.md imports session-context.md)
- ~~Run `/refresh-context` every session~~ (PostToolUse hook handles updates)
- ~~Manually manage Memory MCP~~ (use `/store-pattern` instead)
- ~~Edit `ai-docs/session-context.md`~~ (auto-generated from Memory MCP)

### When Memory System Needs Attention:
- **Monthly:** Run `/memory-cleanup` to optimize Memory MCP and CLAUDE.md
- **After major discoveries:** Use `/store-pattern` immediately
- **If Claude seems uninformed:** Run `/refresh-context` to update session file
- **Before important sessions:** Run `/refresh-context` to ensure latest context

## 🔧 Troubleshooting

### If Claude Seems to Lack Project Knowledge:
1. Check if `ai-docs/session-context.md` exists and is recent
2. Run `/refresh-context` to manually regenerate the session file  
3. Ensure PostToolUse hook is working (check `.claude/hooks/` directory)
4. Verify CLAUDE.md is importing the session file correctly
5. Check that Memory MCP contains recent patterns (`ai-memory.json`)

### If Memory Gets Bloated:
1. Run `/project:memory-cleanup` to streamline
2. Review `ai-memory.json` for redundant entities
3. Ensure observations are atomic facts, not verbose explanations

### If Slash Commands Don't Work:
1. Verify files exist in `.claude/commands/`
2. Check you're using Claude Code CLI, not web interface
3. Ensure you're in the project directory

## 📊 System Status Check

**Current Optimization Level:**
- CLAUDE.md: 67 lines (optimized from 130)
- Memory entities: 15 focused entities with 3-5 observations each
- AI documentation: 7 organized files in ai-docs/
- Automation: Full hooks system active

The memory system is designed to be invisible when working correctly - you should rarely think about it. Just code, ask questions, and store discoveries. Everything else happens automatically.