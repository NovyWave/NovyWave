# CLAUDE.md

Core guidance for Claude Code when working with NovyWave.

## Development Commands

**Browser Mode (default):**
- `makers start` - Start development server with auto-reload at http://localhost:8080
- `makers build` - Production build for browser deployment

**Desktop Mode (Tauri):**
- `makers tauri` - Start Tauri desktop development mode
- `makers tauri-build` - Build production desktop application

**Utilities:**
- `makers install` - Install all dependencies (MoonZoon CLI, Rust WASM target, etc.)
- `makers clean` - Clean all build artifacts

## Architecture

**Dual-Platform:** Web application + Tauri desktop using shared Rust/WASM frontend

**Framework Stack:**
- **Frontend:** Rust + WASM using Zoon framework 
- **Backend:** Moon framework (browser mode only)
- **Desktop:** Tauri v2 wrapper
- **Graphics:** Fast2D rendering library

**Project Structure:**
```
frontend/     - Rust/WASM frontend (shared)
backend/      - MoonZoon backend (browser only)
src-tauri/    - Tauri desktop wrapper
novyui/       - Custom UI component library
public/       - Static assets
```

**Key Dependencies:**
- MoonZoon pinned to git revision `7c5178d891cf4afbc2bbbe864ca63588b6c10f2a`
- Fast2D graphics from NovyWave/Fast2D
- NovyUI component library with IconName tokens

## Critical Development Rules

**WASM Compilation:**
- Use `zoon::println!()` for logging, NOT `std::println!()`
- NEVER use `cargo build` or `cargo check` - only mzoon handles WASM properly
- Auto-reload only triggers after successful compilation

**Focused Productivity System:**
- AUTOMATIC: Hook generates focused productivity context on Memory MCP usage
- Context always available via @.claude/ai-docs/focus-context.md import
- Shows: Current State, Recent Solutions, Active Blockers, Daily Patterns, Next Steps

**Automatic Memory Updates:**
- Update `current_session_state` when switching major tasks or focus areas
- Add to `recent_solutions` immediately after fixing any bugs or compilation errors
- Update `active_blockers` when encountering issues or when resolving existing ones
- Add to `daily_patterns` when discovering essential rules or patterns to remember
- Update `next_steps` when completing current tasks or planning immediate actions

**Memory MCP Entity Focus:**
- Keep focused entities with 3-5 observations maximum
- Store discoveries immediately using focused entity types
- Archive old observations to maintain productivity focus

**Planning Documents & Temporary Files:**
- Create planning documents in `.claude/tmp/` folder (not project root)
- Extract key insights to Memory MCP entities for searchable storage
- Use `/memory-cleanup` to review and clean temporary files periodically
- Keep project root clean with only essential files

**Component Usage:**
- ALL icons use `IconName` enum tokens, never strings
- Use `Width::fill()` for responsive layouts, never fixed widths
- Apply `Font::new().no_wrap()` to prevent text wrapping

## Documentation

@.claude/ai-docs/focus-context.md
@.claude/ai-docs/development-workflow.md
@.claude/ai-docs/novyui-patterns.md  
@.claude/ai-docs/zoon-framework-patterns.md
@.claude/ai-docs/memory-best-practices.md
@.claude/ai-docs/memory-mcp.md
@.claude/ai-docs/browser-mcp.md