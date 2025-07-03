# Memory Cleanup - Monthly maintenance or when feeling context bloat

Optimize CLAUDE.md and Memory MCP for better context efficiency.

**Usage examples:** `/memory-cleanup`

## Quick Examples:

```bash
/memory-cleanup                    # Full system optimization
```

**Perfect timing:**
- After 3-4 weeks of active development
- When CLAUDE.md exceeds 80 lines
- When Memory MCP has 20+ entities
- Before starting major new features
- When Claude responses feel slow/unfocused

## What it does:

1. **Analyze Current State:**
   - Check CLAUDE.md line count and verbosity
   - Review Memory MCP entities and observations  
   - Identify redundant or outdated information

2. **CLAUDE.md Optimization:**
   - Move verbose sections to `ai-docs/` with `@` imports
   - Keep only: commands, core architecture, critical rules
   - Target: 50-65 lines maximum

3. **Memory MCP Consolidation:**
   - Delete redundant/verbose entities
   - Limit entities to 3-5 observations max
   - Remove temporary debugging info
   - Keep only persistent, actionable patterns

4. **Temporary File Cleanup:**
   - Review files in `.claude/tmp/` directory
   - Archive completed planning documents
   - Extract valuable insights to Memory MCP
   - Remove outdated temporary files

**Result:** Faster session starts, better context efficiency, cleaner knowledge base, organized workspace