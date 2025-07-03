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

**Session Start Pattern:**
- AUTOMATIC: Hook generates fresh context on first tool use
- Context always available via @ai-docs/session-context.md import
- Store solved bugs, new patterns, architectural decisions immediately in Memory MCP

**Component Usage:**
- ALL icons use `IconName` enum tokens, never strings
- Use `Width::fill()` for responsive layouts, never fixed widths
- Apply `Font::new().no_wrap()` to prevent text wrapping

## Documentation

@ai-docs/session-context.md
@ai-docs/development-workflow.md
@ai-docs/novyui-patterns.md  
@ai-docs/zoon-framework-patterns.md
@ai-docs/memory-best-practices.md
@ai-docs/memory-mcp.md
@ai-docs/browser-mcp.md