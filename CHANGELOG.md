# Changelog

All notable changes to NovyWave will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-XX-XX

### Added

- **Core Waveform Viewing**
  - Load and view VCD (Value Change Dump) waveform files
  - Load and view FST (Fast Signal Trace) waveform files
  - Load and view GHW (GHDL Wave) waveform files
  - Multi-file support with automatic time alignment
  - File drag-and-drop support

- **Navigation & Interaction**
  - Timeline cursor for precise time measurement (yellow line)
  - Zoom center indicator (blue line) that follows mouse hover
  - Keyboard shortcuts for zoom (W/S), pan (A/D), and cursor movement (Q/E)
  - Shift modifiers for faster navigation
  - Click-to-position cursor on timeline
  - Reset view (R key) and reset zoom center (Z key)

- **Variable Selection**
  - Hierarchical scope browser with file and scope tree
  - Variable search/filter with instant results
  - Multi-file variable selection
  - Smart labeling for duplicate filenames

- **Signal Display**
  - Multiple display formats: Binary, Hex, Octal, Decimal (signed/unsigned), ASCII
  - Per-variable format selection with dropdown
  - Copy signal value to clipboard
  - Real-time value updates at cursor position

- **User Interface**
  - Dark and Light theme support (Ctrl+T to toggle)
  - Dock mode: Bottom (default) or Right layout (Ctrl+D to toggle)
  - Resizable panels with draggable dividers
  - Session persistence (panels, theme, dock mode, selected files/variables)

- **Desktop Application**
  - Cross-platform Tauri 2.0 desktop app
  - Native file dialogs
  - Auto-updater support

### Technical Details

- Built with Rust and WebAssembly (MoonZoon framework)
- Uses wellen library for waveform parsing
- Fast2D canvas rendering for timeline visualization
- Actor+Relay reactive architecture

## Example HDL Projects

NovyWave includes example HDL projects demonstrating various hardware description languages:

- **VHDL** - 8-bit counter using GHDL (generates GHW)
- **Verilog** - 8-bit counter using Icarus Verilog (generates VCD)
- **SpinalHDL** - 8-bit counter using Scala/Verilator (generates VCD)
- **Amaranth** - 8-bit counter using Python (generates VCD)
- **Spade** - 8-bit counter using Spade HDL (generates VCD)

See `examples/` directory for source code and build instructions.
