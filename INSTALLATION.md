# Installation and Setup Guide

**NovyWave** is a professional waveform viewer for digital design verification and analysis, designed as a modern alternative to GTKWave.

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

## Project Structure

- `frontend/` - Rust/WASM frontend code (shared between browser and Tauri)
- `backend/` - MoonZoon backend for browser mode
- `shared/` - Types and utilities shared between frontend and backend
- `src-tauri/` - Tauri desktop configuration and entry point
- `novyui/` - Custom UI component library with design tokens
- `public/` - Static assets (fonts)
- `design/` - UI/UX design assets and documentation
- `docs/` - Project documentation and development guides
- `test_files/` - Sample waveform files for testing (.vcd, .fst)

## Clean Build Artifacts

```bash
makers clean
```

This will clean all build artifacts from both MoonZoon and Tauri.