# Changelog

All notable changes to NovyWave are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- GHW file format support (GHDL waveforms)
- Automatic update system
- Cross-platform release builds

### Changed
- Improved documentation

## [0.1.0] - 2025-01-15

### Added

#### Core Features
- VCD and FST waveform file support
- Hierarchical scope browsing
- Signal selection and visualization
- Timeline navigation with zoom and pan
- Keyboard-driven workflow (WASDQE + shortcuts)
- Timeline cursor with value inspection
- Signal value formatting (Hex, Binary, Decimal, etc.)

#### User Interface
- Three-panel layout (Files & Scopes, Variables, Selected Variables)
- Dark and light themes
- Right and Bottom dock modes
- Panel resizing with drag handles
- Persistent configuration

#### Platform Support
- Browser deployment (WebAssembly)
- Desktop application (Tauri)
- Linux, macOS, and Windows support

#### Navigation
- Smooth zoom centered on zoom point
- Panning with keyboard controls
- Cursor movement with transition jumping
- Full reset capability

### Technical Details
- Built with Rust and WebAssembly
- MoonZoon framework for web UI
- Tauri v2 for desktop wrapper
- Actor+Relay state management
- Fast2D canvas rendering
- Wellen library for waveform parsing

[Unreleased]: https://github.com/NovyWave/NovyWave/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/NovyWave/NovyWave/releases/tag/v0.1.0
