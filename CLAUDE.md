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

## Command Execution Protocol

**CRITICAL BEHAVIORAL RULE**: Slash commands = automation execution, NEVER consultation

**Examples of CORRECT behavior:**
- User types `/core-commit` → Immediately run git analysis commands and present results
- User types `/core-checkpoint` → Immediately execute checkpoint workflow
- User types `/memory-search term` → Immediately search and return results

**Examples of WRONG behavior (never do this):**
- ❌ "Here's how /core-commit works..."
- ❌ "The /core-commit protocol requires..."
- ❌ "You should use /core-commit by..."

**Anti-Consultation Guards**: Command files have explicit enforcement sections to prevent consultation mode

## Virtual List Optimization (Session Learning)

**OPTIMAL CONFIGURATION ACHIEVED**: MutableVec hybrid stable pool with velocity-based dynamic buffering

**Critical Implementation Details**:
- **Stable Element Pool**: DOM elements never recreated, only content/position updates
- **Dynamic Height Support**: Parent-child viewport monitoring with signal-based height propagation
- **Velocity-Based Buffering**: 5 elements (static) → 10 elements (medium scroll) → 15 elements (fast scroll)
- **Performance Testing Results**: Velocity-based buffers optimal, large fixed buffers (50+) cause slower rerendering

**Key Technical Pattern**:
```rust
// MutableVec pool with efficient resizing
let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(...)

// Velocity calculation for smart buffering  
let velocity_buffer = if current_velocity > 1000.0 { 15 } 
                     else if current_velocity > 500.0 { 10 } 
                     else { 5 };
```

**Critical Lesson**: Virtual list buffer size sweet spot is 5-15 elements with velocity adaptation - avoid over-buffering which hurts performance.

## Granular UI Updates Implementation

**CRITICAL DEBUGGING PATTERN**: When UI changes aren't visible after implementation, **always check compilation first**:
```bash
tail -100 dev_server.log | grep -i "error"
```

**Root Cause**: MoonZoon only auto-reloads after **successful compilation**. Failed builds mean browser keeps running old code, making new optimizations invisible.

**Implementation Success**: 
- ButtonBuilder.label_signal() - Reactive text without component recreation
- MutableVec migration - Granular list updates (badges appear individually)
- TreeView external_selected_vec() bridge for compatibility
- Virtual list optimization - Stable pool eliminates DOM recreation
- Result: "Load X Files" button text updates smoothly, badges don't flash/recreate

**Key Lesson**: UI optimization verification requires compilation success verification first.