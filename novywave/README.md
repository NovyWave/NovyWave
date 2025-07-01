# NovyWave

A cross-platform application for web browsers and desktop.

## Prerequisites

1. Install Rust: https://www.rust-lang.org/tools/install
2. Install cargo-make: `cargo install cargo-make`
3. For Tauri mode: Install Tauri prerequisites from https://v2.tauri.app/start/prerequisites/

## Installation

```bash
makers install
```

This will install all necessary dependencies for both browser and Tauri modes.

## Running the Application

### Browser Mode

For development with auto-reload:
```bash
makers start
```

To start and automatically open in browser:
```bash
makers open
```

The app will be available at http://localhost:8080

### Desktop Mode (Tauri)

For development:
```bash
makers tauri
```

To build a production desktop app:
```bash
makers tauri-build
```

## Building

### Browser Build
```bash
makers build
```

### Desktop Build
```bash
makers tauri-build
```

## Features

Three visual examples:

1. **Simple Rectangle** - Basic shape rendering with text overlay
2. **Face with Hat** - Complex composition using circles, rectangles, and lines
3. **Sine Wave** - Mathematical visualization with dynamic point generation

## Project Structure

- `frontend/` - Rust/WASM frontend code (shared between browser and Tauri)
- `backend/` - MoonZoon backend for browser mode
- `src-tauri/` - Tauri-specific configuration and entry point
- `public/` - Static assets (fonts)
- `shared/` - Code shared between frontend and backend

## Clean Build Artifacts

```bash
makers clean
```

This will clean all build artifacts from both MoonZoon and Tauri.