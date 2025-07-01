# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

The project uses `cargo make` (`makers`) for task automation. Key commands:

**Browser Mode (default):**
- `makers start` - Start development server with auto-reload at http://localhost:8080
- `makers open` - Start server and automatically open browser
- `makers build` - Production build for browser deployment

**Desktop Mode (Tauri):**
- `makers tauri` - Start Tauri desktop development mode
- `makers tauri-build` - Build production desktop application

**Utilities:**
- `makers install` - Install all dependencies (MoonZoon CLI, Rust WASM target, etc.)
- `makers clean` - Clean all build artifacts from both browser and desktop builds

## Architecture Overview

**Dual-Platform Application:** NovyWave runs as both a web application and desktop application using the same frontend codebase.

**Framework Stack:**
- **Frontend:** Rust + WASM using Zoon framework (MoonZoon's frontend library)
- **Backend (Browser mode):** Moon framework (MoonZoon's backend)
- **Desktop:** Tauri v2 wrapper around the web frontend
- **Graphics:** Fast2D rendering library (custom 2D graphics engine)

**Project Structure:**
```
frontend/     - Rust/WASM frontend code (shared between browser and desktop)
backend/      - MoonZoon backend for browser mode only
src-tauri/    - Tauri desktop wrapper and configuration
shared/       - Code shared between frontend and backend (currently empty)
public/       - Static assets (fonts: FiraCode, Inter family)
```

**Key Dependencies:**
- MoonZoon pinned to specific git revision `7c5178d891cf4afbc2bbbe864ca63588b6c10f2a`
- Fast2D graphics library from NovyWave/Fast2D repository
- Tauri CLI 2.x for desktop builds

**Frontend Architecture:**
- Entry point: `frontend/src/main.rs:10` - loads fonts then starts app
- Root UI: Column layout with scrollable panels containing Fast2D canvas examples
- Three visual examples: Simple Rectangle, Face with Hat, Sine Wave
- Each example renders to a Fast2D canvas wrapped in Zoon UI elements

**Rendering Pipeline:**
1. Fonts loaded asynchronously from `/public/fonts/`
2. Fast2D objects created in pure Rust
3. Canvas wrapper integrates Fast2D with Zoon's DOM canvas element
4. Objects swapped into canvas and rendered via Fast2D engine

**Development Modes:**
- Browser mode uses MoonZoon's dev server with hot reload
- Desktop mode uses Tauri dev command which launches the desktop app
- Both modes share the same frontend codebase but different build pipelines

**Testing and Quality:**
- No automated test suite is currently configured
- Manual testing is done through visual examples in the UI
- For development verification, use the three built-in examples: Simple Rectangle, Face with Hat, and Sine Wave

**Configuration Files:**
- `MoonZoon.toml` - MoonZoon dev server configuration (port 8080, CORS, file watching)
- `tauri.conf.json` - Tauri app configuration (window size 800x600, build commands)
- `Makefile.toml` - Task runner configuration with all development commands
- `.mcp.json` - MCP server configuration with memory storage at `novywave/ai-memory.json`

## MCP Server Configuration

This repository uses two MCP servers configured in `.mcp.json` (see `.mcp.example.json` for team setup):

@ai-docs/memory-mcp.md

@ai-docs/browser-mcp.md