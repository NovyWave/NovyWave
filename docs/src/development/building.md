# Building from Source

This guide explains how to build NovyWave from source code.

## Prerequisites

### Required Tools

- **Rust** (stable) - [Install Rust](https://rustup.rs/)
- **cargo-make** - Task runner
- **Node.js** (optional) - For some build scripts

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install cargo-make
cargo install cargo-make
```

### Platform-Specific Dependencies

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libssl-dev
```

**macOS:**
```bash
# Xcode Command Line Tools
xcode-select --install
```

**Windows:**
- Visual Studio Build Tools with C++ workload
- WebView2 runtime (usually pre-installed on Windows 10/11)

## Clone the Repository

```bash
git clone https://github.com/NovyWave/NovyWave.git
cd NovyWave
```

## Install Dependencies

```bash
makers install
```

This installs:
- MoonZoon CLI
- Required Rust crates
- WASM build target

## Development Mode

### Browser Mode

Start the development server with hot-reload:

```bash
makers start
```

Open http://localhost:8080 in your browser.

The development server:
- Watches for file changes
- Recompiles automatically
- Reloads the browser

### Desktop Mode (Tauri)

Start Tauri development mode:

```bash
makers tauri
```

This:
- Builds the frontend
- Launches the desktop window
- Enables hot-reload

## Production Build

### Browser Build

```bash
makers build
```

Output: `frontend_dist/` directory ready for web deployment.

### Desktop Build

```bash
makers tauri-build
```

Output: Platform-specific installer in `src-tauri/target/release/bundle/`

## Project Structure

```
NovyWave/
├── frontend/          # Frontend source (Rust/WASM)
├── backend/           # Backend source (Moon)
├── shared/            # Shared types
├── src-tauri/         # Tauri configuration and commands
├── novyui/            # UI component library
├── public/            # Static assets
├── test_files/        # Test waveform files
└── Makefile.toml      # Build tasks
```

## Common Tasks

### Clean Build

```bash
makers clean
makers build
```

### Check Compilation

The MoonZoon dev server compiles automatically. Check the terminal for errors.

**Important:** Don't use `cargo check` or `cargo build` directly - they don't handle WASM compilation correctly. Always use `makers` commands.

### Run Tests

```bash
cargo test --workspace
```

## Troubleshooting

### "mzoon not found"

```bash
makers install
```

### WASM compilation errors

Ensure WASM target is installed:
```bash
rustup target add wasm32-unknown-unknown
```

### Linux: Missing libraries

```bash
sudo apt-get install libwebkit2gtk-4.1-dev
```

### Build is slow

First build takes several minutes to compile all dependencies. Subsequent builds are much faster due to caching.

## Version Management

To update version numbers across all packages:

```bash
./scripts/bump-version.sh 0.2.0
```

This updates:
- All `Cargo.toml` files
- `tauri.conf.json`
