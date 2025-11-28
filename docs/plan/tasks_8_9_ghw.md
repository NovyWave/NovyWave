# Tasks 8, 9, and GHW Support Implementation Plan

## Overview

This plan covers three interconnected work streams:
- **Task 8**: Distribution & Cross-Platform Testing
- **Task 9**: Examples, Documentation & Beta Release
- **GHW File Support**: Enable GHW waveform format (minimal changes required)

---

## Part 0: GHW File Support (Quick Win - 1 hour)

### Current State
The Wellen library already has **complete GHW support** (2,627 lines of parsing code). GHW is explicitly disabled in `backend/src/main.rs` with a comment "pending testing."

### Implementation

#### 0.1 Enable GHW in Shared Types
**File: `shared/src/lib.rs`**
```rust
// Add GHW variant to FileFormat enum
pub enum FileFormat {
    VCD,
    FST,
    GHW,  // Add this
}

// Update is_valid_waveform_extension()
pub fn is_valid_waveform_extension(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "vcd" | "fst" | "ghw")
}
```

#### 0.2 Enable GHW Processing in Backend
**File: `backend/src/main.rs`** (~line 1590)
```rust
// REMOVE the explicit GHW rejection:
// wellen::FileFormat::Ghw | wellen::FileFormat::Unknown => { ... }

// Instead, include GHW in the main processing path alongside VCD/FST
```

#### 0.3 Update Frontend Strings
**Files:**
- `frontend/src/file_picker.rs` - Update dialog title: "(*.vcd, *.fst, *.ghw)"
- `frontend/src/error_display.rs` - Update error messages to include "ghw"

#### 0.4 Testing
- Use existing test files or download GHDL simulation outputs
- Verify hierarchy parsing, signal extraction, and timeline rendering

---

## Part 1: Task 8 - Distribution & Cross-Platform Testing

### 8a. Manual Bundling and Platform-Specific Testing

#### Linux Bundling
```bash
# Build AppImage, deb, rpm
makers tauri-build

# Test on different distributions:
# - Ubuntu 22.04/24.04 (primary target)
# - Fedora 39/40
# - Arch Linux
```

**Linux-Specific Considerations:**
- AppImage is most portable (self-contained)
- .deb for Ubuntu/Debian users
- .rpm for Fedora/RHEL users
- Test with both X11 and Wayland

#### macOS Bundling
```bash
# Build .app bundle and .dmg
makers tauri-build
```

**macOS-Specific Considerations:**
- Universal binary (arm64 + x86_64) for M1/M2/Intel support
- .dmg for drag-and-drop installation
- Test on macOS 12+ (Monterey and later)

#### Windows Bundling
```bash
# Build .msi and .exe installer
makers tauri-build
```

**Windows-Specific Considerations:**
- .msi for enterprise deployment
- NSIS .exe for consumer installation
- Test on Windows 10/11

#### Platform Testing Checklist
- [ ] Application launches correctly
- [ ] File picker opens and navigates filesystem
- [ ] VCD/FST/GHW files load and parse
- [ ] Timeline renders with correct zoom/pan
- [ ] Theme switching works
- [ ] Dock mode switching works
- [ ] Configuration persists across restarts
- [ ] Keyboard shortcuts function

---

### 8b. Finalize Bundling, Versioning, Signing, Releasing

#### Version Management Strategy

**Semantic Versioning:** `MAJOR.MINOR.PATCH`
- Major: Breaking changes to file format or config
- Minor: New features (GHW support, new panels)
- Patch: Bug fixes, performance improvements

**Version Sync Script** (`scripts/bump-version.sh`):
```bash
#!/bin/bash
VERSION=$1
# Update all version locations:
# - Cargo.toml (workspace version)
# - src-tauri/Cargo.toml
# - src-tauri/tauri.conf.json
# - package.json (if exists)
```

#### Code Signing Setup

**macOS (Apple Developer ID):**
```json
// tauri.conf.json additions
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)",
      "providerShortName": "TEAM_ID"
    }
  }
}
```

**Windows (Code Signing Certificate):**
```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "YOUR_CERT_THUMBPRINT",
      "digestAlgorithm": "sha256"
    }
  }
}
```

#### Notarization (macOS)
```bash
# After signing, notarize with Apple
xcrun notarytool submit NovyWave.dmg --apple-id $APPLE_ID --team-id $TEAM_ID --password $APP_SPECIFIC_PASSWORD
```

#### Security Hardening
**File: `src-tauri/tauri.conf.json`**
```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'"
    }
  }
}
```

---

### 8c. Automated Release Pipeline (Woodpecker CI)

#### Woodpecker Configuration
**File: `.woodpecker.yml`**

```yaml
when:
  event: [push, tag]
  branch: main

variables:
  - &rust_version "1.82"

steps:
  # Step 1: Build Linux
  linux-build:
    image: rust:${rust_version}
    commands:
      - apt-get update && apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - cargo install tauri-cli --version "^2.0"
      - cargo tauri build
    when:
      event: tag

  # Step 2: Build macOS (requires macOS runner)
  macos-build:
    image: macos-14
    commands:
      - cargo install tauri-cli --version "^2.0"
      - cargo tauri build --target universal-apple-darwin
    when:
      event: tag

  # Step 3: Build Windows (requires Windows runner)
  windows-build:
    image: windows-2022
    commands:
      - cargo install tauri-cli --version "^2.0"
      - cargo tauri build
    when:
      event: tag

  # Step 4: Create GitHub Release
  release:
    image: plugins/github-release
    settings:
      api_key:
        from_secret: github_token
      files:
        - target/release/bundle/appimage/*.AppImage
        - target/release/bundle/deb/*.deb
        - target/release/bundle/macos/*.dmg
        - target/release/bundle/msi/*.msi
      title: "NovyWave v${CI_COMMIT_TAG}"
      note: CHANGELOG.md
    when:
      event: tag
```

#### Alternative: GitHub Actions
**File: `.github/workflows/release.yml`**

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
          - os: macos-14
            target: universal-apple-darwin
          - os: windows-2022
            target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          # macOS signing
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        with:
          tagName: v__VERSION__
          releaseName: 'NovyWave v__VERSION__'
          releaseBody: 'See CHANGELOG.md for details.'
```

---

### 8d. Auto-Updater System

#### Install Tauri Updater Plugin
```bash
cargo add tauri-plugin-updater
```

#### Configure Updater
**File: `src-tauri/tauri.conf.json`**
```json
{
  "plugins": {
    "updater": {
      "pubkey": "YOUR_PUBLIC_KEY",
      "endpoints": [
        "https://github.com/NovyWave/NovyWave/releases/latest/download/latest.json"
      ]
    }
  }
}
```

#### Update Manifest Format
**File: `latest.json` (generated by CI)**
```json
{
  "version": "0.2.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2025-01-15T00:00:00Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "...",
      "url": "https://github.com/.../releases/download/v0.2.0/novywave_0.2.0_amd64.AppImage.tar.gz"
    },
    "darwin-universal": {
      "signature": "...",
      "url": "https://github.com/.../releases/download/v0.2.0/NovyWave.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "...",
      "url": "https://github.com/.../releases/download/v0.2.0/NovyWave_0.2.0_x64_en-US.msi.zip"
    }
  }
}
```

#### Frontend Update Check
**File: `src-tauri/src/lib.rs`**
```rust
use tauri_plugin_updater::UpdaterExt;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(update) = handle.updater().check().await.ok().flatten() {
                    // Prompt user to update
                    update.download_and_install(|_, _| {}, || {}).await.ok();
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running app");
}
```

---

## Part 2: Task 9 - Examples, Documentation & Beta Release

### 9a. Polish & Launch Beta Release

#### Pre-Beta Checklist
- [ ] All critical bugs fixed
- [ ] GHW support enabled and tested
- [ ] Cross-platform builds verified
- [ ] Auto-updater functional
- [ ] Basic documentation complete

#### Beta Landing Page Content
- Download links for all platforms
- Quick start guide
- Known limitations
- Feedback submission (GitHub Issues link)
- Roadmap preview

#### Beta Announcement Channels
- GitHub Releases (primary)
- Twitter/X announcement
- Reddit (r/FPGA, r/VHDL, r/rust)
- Hardware development forums

---

### 9b. Example Projects for Multiple HDLs

#### Directory Structure
```
examples/
├── verilog/
│   ├── counter/
│   │   ├── counter.v
│   │   ├── counter_tb.v
│   │   ├── Makefile
│   │   ├── README.md
│   │   └── output/
│   │       └── counter.vcd
│   └── uart/
│       ├── uart.v
│       ├── uart_tb.v
│       └── ...
├── vhdl/
│   ├── alu/
│   │   ├── alu.vhd
│   │   ├── alu_tb.vhd
│   │   ├── Makefile (GHDL)
│   │   └── output/
│   │       └── alu.ghw
│   └── ...
├── spinalhdl/
│   ├── blinky/
│   │   ├── build.sbt
│   │   ├── src/main/scala/Blinky.scala
│   │   ├── src/test/scala/BlinkyTest.scala
│   │   └── output/
│   │       └── blinky.vcd
│   └── ...
├── amaranth/
│   ├── led_blinker/
│   │   ├── led_blinker.py
│   │   ├── test_led_blinker.py
│   │   └── output/
│   │       └── led_blinker.vcd
│   └── ...
└── spade/
    ├── counter/
    │   ├── counter.spade
    │   ├── test_counter.spade
    │   └── output/
    │       └── counter.vcd
    └── ...
```

#### Example Project Template
Each example should include:
1. **Source code** - Well-commented HDL
2. **Testbench** - Generates waveform output
3. **Makefile/Build script** - One-command simulation
4. **README.md** - What it demonstrates, how to run
5. **Pre-generated output** - Ready-to-view waveform file
6. **NovyWave workspace** - `.novywave` config file

#### Priority Examples

**Tier 1 (Must Have):**
1. **Verilog Counter** - Basic sequential logic with VCD output
2. **VHDL ALU** - Combinational logic with GHW output (GHDL)
3. **SpinalHDL Blinky** - Modern Scala-based HDL

**Tier 2 (Should Have):**
4. **Amaranth LED Blinker** - Python-based HDL
5. **Spade Counter** - Emerging HDL support
6. **Verilog UART** - More complex example

**Tier 3 (Nice to Have):**
7. **SystemVerilog FSM** - Industry-standard example
8. **Multi-file project** - Demonstrates hierarchy navigation
9. **Mixed-language** - Verilog + VHDL co-simulation

---

### 9c. Comprehensive Documentation

#### Documentation Structure
```
docs/
├── user-guide/
│   ├── index.md              # Overview
│   ├── installation.md       # Platform-specific install
│   ├── quick-start.md        # 5-minute getting started
│   ├── loading-files.md      # VCD/FST/GHW loading
│   ├── navigation.md         # Timeline, zoom, pan
│   ├── keyboard-shortcuts.md # Complete shortcut reference
│   ├── configuration.md      # .novywave file format
│   └── troubleshooting.md    # Common issues
├── api/
│   ├── plugin-api.md         # Plugin development
│   ├── message-protocol.md   # UpMsg/DownMsg reference
│   └── data-types.md         # Shared type definitions
├── development/
│   ├── architecture.md       # System architecture
│   ├── contributing.md       # How to contribute
│   ├── building.md           # Build from source
│   ├── testing.md            # Test suite documentation
│   └── actor-relay.md        # Actor+Relay patterns
└── tutorials/
    ├── first-waveform.md     # Load and explore first file
    ├── multi-file.md         # Working with multiple files
    ├── custom-formats.md     # Signal formatters
    └── plugin-tutorial.md    # Build your first plugin
```

#### Documentation Tooling
**Recommended: mdBook**
```bash
cargo install mdbook
mdbook init docs
mdbook serve docs  # Local preview
mdbook build docs  # Generate static site
```

#### Key Documentation Pieces

**1. Quick Start Guide**
- Download and install (links to releases)
- Open first waveform file
- Basic navigation (zoom, pan, cursor)
- Select signals for display
- Done in 5 minutes

**2. Keyboard Shortcuts Reference**
Extract from specs and format as quick-reference table:
| Key | Action |
|-----|--------|
| W/S | Zoom in/out |
| A/D | Pan left/right |
| Q/E | Move cursor |
| Z | Reset zoom center |
| R | Reset all |
| Ctrl+T | Toggle theme |
| Ctrl+D | Toggle dock mode |

**3. Configuration Reference**
Document `.novywave` TOML format with all sections and options.

**4. Plugin API Reference**
Auto-generate from Rust docs + manual examples.

**5. CONTRIBUTING.md**
```markdown
# Contributing to NovyWave

## Development Setup
1. Install Rust (rustup)
2. Clone repository
3. Run `makers install`
4. Run `makers start`

## Code Style
- Follow Rust conventions
- Use Actor+Relay patterns (see docs/development/actor-relay.md)
- No raw Mutables

## Pull Request Process
1. Fork and branch from `main`
2. Make changes with tests
3. Run `makers build` to verify
4. Submit PR with description

## Reporting Issues
Use GitHub Issues with reproduction steps.
```

---

## Implementation Order

### Phase 1: GHW Support (Day 1)
1. Enable GHW in shared types
2. Remove backend rejection
3. Update frontend strings
4. Test with GHDL output files

### Phase 2: Manual Bundling (Days 2-3)
1. Test Linux builds (AppImage, deb)
2. Test macOS builds (dmg, universal binary)
3. Test Windows builds (msi, exe)
4. Create testing checklist

### Phase 3: CI/CD Pipeline (Days 4-6)
1. Set up Woodpecker/GitHub Actions
2. Configure multi-platform builds
3. Test release workflow with test tags

### Phase 4: Code Signing (Days 7-8)
1. Obtain/configure certificates
2. Set up macOS notarization
3. Configure Windows signing
4. Test signed builds

### Phase 5: Auto-Updater (Days 9-10)
1. Install tauri-plugin-updater
2. Configure update endpoints
3. Generate signing keys
4. Test update flow

### Phase 6: Documentation (Days 11-15)
1. Set up mdBook
2. Write Quick Start Guide
3. Write User Guide sections
4. Create API documentation
5. Write CONTRIBUTING.md

### Phase 7: Examples (Days 16-18)
1. Create Verilog counter example
2. Create VHDL ALU example (GHW)
3. Create SpinalHDL example
4. Create Amaranth example
5. Create Spade example

### Phase 8: Beta Release (Days 19-20)
1. Final testing pass
2. Create release notes
3. Publish v0.2.0-beta.1
4. Announce on channels

---

## Critical Files to Modify

### GHW Support
- `shared/src/lib.rs` - FileFormat enum
- `backend/src/main.rs` - Remove GHW rejection (~line 1590)
- `frontend/src/file_picker.rs` - Dialog title
- `frontend/src/error_display.rs` - Error messages

### Tauri Bundling
- `src-tauri/tauri.conf.json` - Security, signing, updater config
- `src-tauri/Cargo.toml` - Add tauri-plugin-updater
- `src-tauri/src/lib.rs` - Update check logic

### CI/CD
- `.woodpecker.yml` OR `.github/workflows/release.yml`
- `scripts/bump-version.sh` - Version management
- `CHANGELOG.md` - Release notes

### Documentation
- `docs/` - All new documentation structure
- `README.md` - Update with user-facing info
- `CONTRIBUTING.md` - New file

---

## Decisions Made

1. **CI/CD Platform**: GitHub Actions (primary), Woodpecker (backup/alternative)
2. **Code Signing**: Skip for beta, implement for v1.0 release
3. **Documentation Host**: GitHub Pages (free, `novywave.github.io/NovyWave`)
4. **Auto-Update Server**: GitHub Releases (simplest, free)
5. **Example HDLs**: All six + additional modern HDLs

---

## Code Signing Strategy

### Beta Release (Skip Signing)
- Document "How to install unsigned app" for Windows/macOS
- Linux doesn't require signing
- Early adopters are tech-savvy and can bypass warnings

### v1.0 Release (Full Signing)
- **macOS**: Apple Developer Program ($99/year, 1-2 days approval)
- **Windows**: Azure Trusted Signing (free for OSS) or commercial cert ($200-500/year)
- Enables clean installation experience and enterprise deployment

---

## Documentation Strategy

### GitHub Pages Setup
- **Domain**: `novywave.github.io/NovyWave` (free)
- **Custom domain option**: `docs.novywave.app` (if desired later)
- **Free HTTPS**: Included automatically
- **Deployment**: Automatic from `/docs` folder or `gh-pages` branch
- **Tool**: mdBook for Rust documentation style

---

## Complete HDL Example List

### Tier 1: Industry Standards (Must Have)
1. **Verilog** - Counter example, Icarus Verilog → VCD output
2. **SystemVerilog** - FSM example, Verilator → VCD output
3. **VHDL** - ALU example, GHDL → GHW output (demonstrates GHW support!)

### Tier 2: Modern HDLs (Should Have)
4. **SpinalHDL** - Blinky example, Scala-based, generates VCD
5. **Amaranth** - LED example, Python-based, generates VCD
6. **Spade** - Counter example, Rust-inspired syntax

### Tier 3: Additional Modern HDLs (Nice to Have)
7. **Chisel** - RISC-V focused, SiFive community
8. **MyHDL** - Simpler Python HDL
9. **Bluespec** - Academic, BSV syntax
10. **Clash** - Haskell-based, academic interest
11. **Silice** - FPGA-first design

### Example Complexity Levels
| Level | Examples |
|-------|----------|
| **Beginner** | LED Blinker, 4-bit Counter |
| **Intermediate** | UART, SPI Controller, PWM |
| **Advanced** | Simple CPU, AXI Peripheral |
