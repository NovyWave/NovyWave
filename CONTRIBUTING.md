# Contributing to NovyWave

Thank you for your interest in contributing to NovyWave! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

1. **Rust toolchain**: Install from https://www.rust-lang.org/tools/install
2. **cargo-make**: `cargo install cargo-make`
3. **For Tauri development**: Install prerequisites from https://v2.tauri.app/start/prerequisites/

### Development Setup

```bash
# Clone the repository
git clone https://github.com/NovyWave/NovyWave.git
cd NovyWave

# Install all dependencies
makers install

# Start development server (browser mode)
makers start
```

The app will be available at http://localhost:8080 with auto-reload.

For desktop development:
```bash
makers tauri
```

## Project Architecture

NovyWave is a dual-platform application:

- **Browser Mode**: Uses MoonZoon framework with Rust/WASM frontend and Moon backend
- **Desktop Mode**: Uses Tauri v2 wrapper around the same frontend

### Directory Structure

| Directory | Purpose |
|-----------|---------|
| `frontend/` | Rust/WASM frontend code (shared between browser and Tauri) |
| `backend/` | MoonZoon backend for browser mode |
| `shared/` | Types and utilities shared between frontend and backend |
| `src-tauri/` | Tauri desktop configuration and entry point |
| `novyui/` | Custom UI component library with design tokens |
| `docs/` | Documentation and development guides |
| `test_files/` | Sample waveform files for testing |

### Supported Waveform Formats

NovyWave supports the following waveform file formats:
- **VCD** (Value Change Dump) - Standard IEEE format
- **FST** (Fast Signal Trace) - GTKWave's compressed format
- **GHW** (GHDL Waveform) - GHDL native format

## Development Workflow

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p frontend
cargo test -p shared
```

### Building

```bash
# Browser build (release)
makers build

# Desktop build (creates installers)
makers tauri-build
```

### Common Development Tasks

```bash
# Start browser dev server
makers start

# Start Tauri dev mode
makers tauri

# Clean all build artifacts
makers clean

# List all available tasks
makers --list-all-steps
```

## Code Style Guidelines

### Rust Code

- Follow standard Rust formatting (`cargo fmt`)
- Use `clippy` for linting (`cargo clippy`)
- Prefer explicit error handling over `unwrap()`
- Write documentation comments for public APIs
- Use `zoon::println!()` for WASM logging, not `std::println!()`

### Architecture Patterns

NovyWave uses **Actor+Relay architecture** for state management:

- **Actors**: Manage domain state (TrackedFiles, SelectedVariables, WaveformTimeline)
- **Relays**: Handle event communication between components
- **Atoms**: Used for local UI state only

Key rules:
- No raw global Mutables - use Actor+Relay patterns
- Event-source relay naming (describe what happened, not what to do)
- Domain-driven design (model what it IS, not what manages it)

See `.claude/extra/architecture/actor-relay-patterns.md` for detailed patterns.

### Commit Messages

- Use clear, descriptive commit messages
- Reference issue numbers when applicable
- Keep commits focused on a single change

Example:
```
Add GHW file format support in file picker

- Enable GHW extension validation in is_waveform_file()
- Update error messages to include GHW format
- Fixes #123
```

## Submitting Changes

### Pull Request Process

1. **Fork** the repository
2. **Create a branch** for your feature/fix: `git checkout -b feature/my-feature`
3. **Make your changes** with clear, focused commits
4. **Test your changes** in both browser and Tauri modes if applicable
5. **Update documentation** if needed
6. **Submit a pull request** with a clear description

### PR Guidelines

- Describe what the PR does and why
- Include screenshots for UI changes
- Reference related issues
- Ensure all tests pass
- Keep PRs focused and reasonably sized

## Reporting Issues

When reporting bugs, please include:

1. **Operating system** and version
2. **NovyWave version** (or commit hash)
3. **Steps to reproduce** the issue
4. **Expected behavior** vs **actual behavior**
5. **Sample waveform files** if relevant (or describe the file characteristics)
6. **Console logs** or error messages

## Feature Requests

We welcome feature suggestions! When proposing features:

1. Check existing issues to avoid duplicates
2. Describe the use case and motivation
3. Provide examples if possible
4. Be open to discussion about implementation approaches

## Community

- **Questions**: Contact martin@kavik.cz
- **Issues**: https://github.com/NovyWave/NovyWave/issues

## License

By contributing to NovyWave, you agree that your contributions will be licensed under the same license as the project.

## Acknowledgments

NovyWave is funded through [NGI Zero Core](https://nlnet.nl/core), a fund established by [NLnet](https://nlnet.nl) with financial support from the European Commission's [Next Generation Internet](https://ngi.eu) program.
