# NovyWave Project Configuration

## Project Overview

NovyWave - Professional waveform viewer for digital design verification and analysis.

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
shared/       - Shared types and utilities between frontend/backend
novyui/       - Custom UI component library
public/       - Static assets
```

## Key Dependencies

- MoonZoon pinned to git revision `7c5178d891cf4afbc2bbbe864ca63588b6c10f2a`
- Fast2D graphics from NovyWave/Fast2D
- NovyUI component library with IconName tokens

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

## NovyWave-Specific Rules

**Component Usage:**
- ALL icons use `IconName` enum tokens, never strings
- Use `Width::fill()` for responsive layouts, never fixed widths
- Apply `Font::new().no_wrap()` to prevent text wrapping

**Domain Focus:**
- Professional waveform visualization
- Digital design verification workflows
- High-performance graphics rendering
- Desktop and web dual deployment

## Shared Crate Usage

The `shared/` crate contains types and utilities that need to be used by both frontend and backend:

**Core Types:**
- `LoadingFile`, `LoadingStatus` - File loading state management
- `WaveformFile`, `ScopeData`, `Signal` - Waveform data structures
- `UpMsg`, `DownMsg` - Communication messages between frontend/backend
- `AppConfig` and related config types - Application configuration

**When to Use:**
- Any type that needs to be serialized/deserialized between frontend and backend
- Data structures representing waveform files and their contents
- Configuration types that are saved/loaded from disk
- Message types for frontend-backend communication

**Import Pattern:**
```rust
use shared::{LoadingFile, LoadingStatus, WaveformFile, Signal};
```

**Do NOT duplicate types:** Always import from `shared` rather than defining duplicate types in frontend or backend.