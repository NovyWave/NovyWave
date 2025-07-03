# Memory Cleanup Command

Optimize CLAUDE.md and Memory MCP for better context efficiency.

## What it does:

1. **Analyze Current State:**
   - Check CLAUDE.md line count and verbosity
   - Review Memory MCP entities and observations
   - Identify redundant or outdated information

2. **CLAUDE.md Optimization:**
   - Move verbose sections to `docs/` with `@` imports
   - Keep only: commands, core architecture, critical rules
   - Target: 50-65 lines maximum

3. **Memory MCP Consolidation:**
   - Delete redundant/verbose entities
   - Limit entities to 3-5 observations max
   - Remove temporary debugging info
   - Keep only persistent, actionable patterns

4. **Create Documentation Structure:**
   - `docs/development-workflow.md` - WASM workflow, testing patterns
   - `docs/novyui-patterns.md` - Component API, layout patterns
   - `docs/zoon-framework-patterns.md` - Framework fundamentals
   - `docs/memory-best-practices.md` - Session patterns, storage decisions

5. **Implement Session Start Pattern:**
   - Add mandatory `mcp__memory__search_nodes` at session beginning
   - Create storage decision matrix
   - Establish immediate storage triggers

## Usage:

```
/project:memory-cleanup
```

This command will systematically optimize your Claude Code memory system for maximum efficiency and minimal context usage.