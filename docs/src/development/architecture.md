# Architecture Overview

NovyWave is built with Rust, targeting both web browsers (via WebAssembly) and desktop platforms (via Tauri).

## High-Level Architecture

```
┌─────────────────────────────────────────────────────┐
│                    NovyWave                          │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌────────────┐  │
│  │  Frontend   │  │   Backend   │  │  src-tauri │  │
│  │  (WASM)     │  │   (Moon)    │  │  (Desktop) │  │
│  └──────┬──────┘  └──────┬──────┘  └─────┬──────┘  │
│         │                │               │         │
│         └────────┬───────┘               │         │
│                  │                       │         │
│         ┌───────┴──────┐                │         │
│         │    shared    │                │         │
│         │   (Types)    │                │         │
│         └──────────────┘                │         │
└─────────────────────────────────────────────────────┘
```

## Project Structure

```
NovyWave/
├── frontend/          # Rust/WASM frontend using Zoon
│   └── src/
│       ├── dataflow/  # Actor+Relay implementation
│       ├── visualizer/# Waveform rendering
│       └── ...
├── backend/           # MoonZoon backend (browser mode)
│   └── src/
├── shared/            # Shared types and utilities
│   └── src/
├── src-tauri/         # Tauri desktop wrapper
│   └── src/
├── novyui/            # Custom UI component library
└── public/            # Static assets
```

## Framework Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Frontend | Rust + WASM | UI and user interaction |
| Backend | Moon | File I/O, waveform parsing |
| Desktop | Tauri v2 | Native desktop application |
| Graphics | Fast2D | Canvas rendering |
| State | Actor+Relay | Reactive state management |

## State Management: Actor+Relay

NovyWave uses a custom Actor+Relay architecture for state management. See [Actor+Relay Pattern](./actor-relay.md) for details.

### Key Concepts

- **Actors** manage state through message processing
- **Relays** emit events for UI and cross-domain communication
- **Signals** provide reactive data binding to UI
- **Atoms** handle local UI state (hover, focus, etc.)

## Communication Patterns

### Browser Mode

```
Frontend (WASM) ←→ Backend (Moon)
       ↓                 ↓
   UI Rendering    File Operations
```

Frontend and backend communicate via MoonZoon's Connection:
- `UpMsg` - Frontend → Backend messages
- `DownMsg` - Backend → Frontend messages

### Desktop Mode (Tauri)

```
Frontend (WASM) ←→ Tauri Commands
       ↓                 ↓
   UI Rendering    File Operations
```

Tauri commands replace backend communication:
- `#[tauri::command]` functions handle file I/O
- Same frontend code works in both modes

## Key Domains

### TrackedFiles
Manages loaded waveform files:
- File loading state
- Parsing progress
- Scope hierarchy
- File disambiguation

### SelectedVariables
Manages signal selection:
- Selected variables list
- Format settings
- Selection order

### WaveformTimeline
Manages timeline visualization:
- Zoom and pan state
- Cursor position
- Visible range
- Signal data caching

## Rendering Pipeline

```
Waveform Data → Signal Processing → Canvas Rendering
     │                │                    │
     └─ Transitions  └─ Decimation       └─ Fast2D
```

1. **Data Loading** - Wellen library parses waveform files
2. **Signal Processing** - Decimation for visible time range
3. **Canvas Rendering** - Fast2D draws waveforms

## File Format Support

| Format | Library | Description |
|--------|---------|-------------|
| VCD | Wellen | Value Change Dump |
| FST | Wellen | Fast Signal Trace |
| GHW | Wellen | GHDL Waveform |

Wellen provides efficient parsing with:
- Header-only parsing for fast initial load
- On-demand signal data loading
- Built-in decimation support
