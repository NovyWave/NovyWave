# CLAUDE.md

Core guidance for Claude Code when working with NovyWave.

<!-- Core System Layer - Universal Claude Configuration -->
@.claude/core/SYSTEM.md
@.claude/core/memory-management.md
@.claude/core/mcp-tools.md
@.claude/core/development.md

<!-- Framework Layer - MoonZoon-Specific Patterns -->
@.claude/frameworks/moonzoon/FRAMEWORK.md
@.claude/frameworks/moonzoon/patterns.md
@.claude/frameworks/moonzoon/debugging.md

<!-- Project Layer - NovyWave-Specific Configuration -->
@.claude/project/project-overview.md
@.claude/project/novyui-patterns.md

<!-- Auto-Generated Productivity Context -->
@.claude/ai-docs/focus-context.md

<!-- Remaining AI Documentation -->
@.claude/ai-docs/memory-best-practices.md
@.claude/ai-docs/memory-mcp.md
@.claude/ai-docs/browser-mcp.md

## Granular UI Updates Implementation (Session Learning)

**CRITICAL DEBUGGING PATTERN**: When UI changes aren't visible after implementation, **always check compilation first**:
```bash
tail -100 dev_server.log | grep -i "error"
```

**Root Cause**: MoonZoon only auto-reloads after **successful compilation**. Failed builds mean browser keeps running old code, making new optimizations invisible.

**Implementation Success**: 
- ButtonBuilder.label_signal() - Reactive text without component recreation
- MutableVec migration - Granular list updates (badges appear individually)
- TreeView external_selected_vec() bridge for compatibility
- Result: "Load X Files" button text updates smoothly, badges don't flash/recreate

**Key Lesson**: UI optimization verification requires compilation success verification first.