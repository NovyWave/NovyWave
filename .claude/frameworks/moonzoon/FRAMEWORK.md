# MoonZoon Framework Configuration

## Framework Overview

MoonZoon is a Rust-based full-stack web framework using:
- **Frontend:** Rust + WASM using Zoon UI framework
- **Backend:** Moon server framework (optional)
- **Build Tool:** mzoon CLI

## Development Commands

**Standard Commands:**
- `makers start` - Start development server with auto-reload
- `makers build` - Production build
- `makers install` - Install dependencies
- `makers clean` - Clean build artifacts

**Desktop Support (if using Tauri):**
- `makers tauri` - Start Tauri desktop development
- `makers tauri-build` - Build desktop application

## Project Structure

```
frontend/     - Rust/WASM frontend code
backend/      - Moon backend (optional)
public/       - Static assets
```

## Critical WASM Rules

**Compilation:**
- Use `zoon::println!()` for logging, NOT `std::println!()`
- NEVER use `cargo build` or `cargo check` - only mzoon handles WASM properly
- Auto-reload only triggers after successful compilation

**Development Workflow:**
- Run `makers start > dev_server.log 2>&1 &` as BACKGROUND PROCESS
- Monitor compilation with `tail -f dev_server.log`
- Read compilation errors from mzoon output for accurate WASM build status
- Browser auto-reloads ONLY after successful compilation

## Common Patterns

**UI Components:**
- Use Zoon's signal-based reactive system
- Prefer `Width::fill()` for responsive layouts
- Use `Height::screen()` on root + `Height::fill()` on containers

**State Management:**
- Signals for reactive state
- Mutable for local component state
- Global state through static signals

**Error Handling:**
- WASM panics show in browser console
- Use `zoon::println!()` for debug output
- Check browser DevTools for runtime errors