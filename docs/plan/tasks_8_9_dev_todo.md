# Tasks 8 & 9 - Developer Finalization TODO

Quick reference for remaining manual steps to complete the beta release.

---

## ✅ Already Done (Automated)

- [x] GHW enabled in `is_waveform_file()`
- [x] GHW error messages updated
- [x] CONTRIBUTING.md created
- [x] SECURITY.md created
- [x] CSP security enabled in tauri.conf.json
- [x] Example docs filled (VHDL, Verilog, SpinalHDL, Amaranth, Spade)
- [x] README.md enhanced with features
- [x] Linux builds working (AppImage, deb, rpm)
- [x] Tauri workspace history commands implemented (feature parity with browser mode)
- [x] CI/CD pipelines created (GitHub Actions + Woodpecker CI alternative)
- [x] GitHub Actions: CI checks, release builds, docs deployment
- [x] Woodpecker CI: Self-hostable alternative for Gitea/Forgejo

---

## ⚠️ IMPORTANT: Test Both Versions

**Both browser and Tauri desktop versions must be tested to ensure feature parity:**

### Browser Mode Testing
```bash
makers start                    # Start dev server at http://localhost:8080
# Test workspace history:
# 1. Open workspace, expand directories, select files
# 2. Close and reopen - state should persist in ~/.config/novywave/.novywave_global
```

### Tauri Desktop Mode Testing
```bash
cd src-tauri && cargo tauri build   # Build desktop app
./target/release/bundle/appimage/NovyWave_0.1.0_amd64.AppImage  # Run AppImage
# Test workspace history:
# 1. Open workspace, expand directories, select files
# 2. Close and reopen - state should persist in ~/.config/novywave/.novywave_global
```

### Compilation Status (2024-11-28)
- ✅ Browser mode: Compiles with 0 errors, 2 warnings (unused imports)
- ✅ Tauri mode: Compiles with 0 errors, 0 warnings
- ✅ All 3 Linux bundles generated: AppImage (79MB), DEB (8.4MB), RPM (8.4MB)

---

## 1. Auto-Updater Signing Keys

```bash
# Generate Tauri signing key pair
cargo install tauri-cli
cargo tauri signer generate

# Output location: ~/.tauri/keys/
# - novywave.key (PRIVATE - keep secret, use in CI)
# - novywave.key.pub (PUBLIC - goes in tauri.conf.json)
```

**Update `src-tauri/tauri.conf.json`:**
```json
"plugins": {
  "updater": {
    "pubkey": "PASTE_YOUR_PUBLIC_KEY_HERE",
    "endpoints": [
      "https://github.com/NovyWave/NovyWave/releases/latest/download/latest.json"
    ]
  }
}
```

**CI Secret:** Add `TAURI_SIGNING_PRIVATE_KEY` to GitHub Actions secrets.

---

## 2. GitHub Pages Documentation

### Option A: Deploy from `/docs/book/book` folder
```bash
# Build docs locally
cd docs/book
mdbook build

# Commit and push
git add book/
git commit -m "Build documentation"
git push

# GitHub Settings → Pages → Source: Deploy from branch
# Branch: main, Folder: /docs/book/book
```

### Option B: GitHub Actions auto-deploy
Create `.github/workflows/docs.yml`:
```yaml
name: Deploy Docs
on:
  push:
    branches: [main]
    paths: ['docs/**']

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install mdbook
      - run: cd docs/book && mdbook build
      - uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./docs/book/book
```

**Result:** `https://novywave.github.io/NovyWave/`

---

## 3. Linux Testing

### Build
```bash
makers install
makers tauri-build

# Output: src-tauri/target/release/bundle/
#   - appimage/novy-wave_0.1.0_amd64.AppImage
#   - deb/novy-wave_0.1.0_amd64.deb
#   - rpm/novy-wave-0.1.0-1.x86_64.rpm
```

### Test Checklist
- [ ] AppImage runs directly (`chmod +x && ./NovyWave.AppImage`)
- [ ] .deb installs (`sudo dpkg -i novywave.deb`)
- [ ] .rpm installs (`sudo rpm -i novywave.rpm`)
- [ ] App launches from terminal and desktop menu
- [ ] File picker opens home directory (`~`)
- [ ] Load VCD file → renders timeline
- [ ] Load FST file → renders timeline
- [ ] Load GHW file → renders timeline (NEW!)
- [ ] Theme toggle (Ctrl+T)
- [ ] Dock mode toggle (Ctrl+D)
- [ ] Keyboard shortcuts (W/S zoom, A/D pan, Q/E cursor)
- [ ] Config persists to `~/.config/novywave/` after restart
- [ ] Global history persists to `~/.config/novywave/.novywave_global`

### AppImage Portability
```bash
# Test portable mode - should work from any location
mv NovyWave.AppImage /tmp/
cd /tmp && ./NovyWave.AppImage
# Config should still save to ~/.config/novywave/
```

---

## 4. macOS Testing

### Build
```bash
# On macOS machine:
makers install
makers tauri-build

# Output: src-tauri/target/release/bundle/
#   - macos/NovyWave.app
#   - dmg/NovyWave_0.1.0_x64.dmg (or universal)
```

### Test Checklist
- [ ] App launches (may need right-click → Open for unsigned)
- [ ] File picker opens home directory
- [ ] Load VCD file → renders timeline
- [ ] Load FST file → renders timeline
- [ ] Load GHW file → renders timeline (NEW!)
- [ ] Theme toggle (Ctrl+T or Cmd+T)
- [ ] Dock mode toggle (Ctrl+D or Cmd+D)
- [ ] Keyboard shortcuts (W/S zoom, A/D pan, Q/E cursor)
- [ ] Config persists after restart

### Universal Binary (M1 + Intel)
```bash
# Requires both targets installed
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo tauri build --target universal-apple-darwin
```

---

## 5. Windows Testing

### Build
```bash
# On Windows machine (or cross-compile):
makers install
makers tauri-build

# Output: src-tauri/target/release/bundle/
#   - msi/NovyWave_0.1.0_x64_en-US.msi
#   - nsis/NovyWave_0.1.0_x64-setup.exe
```

### Test Checklist
- [ ] MSI installer works
- [ ] NSIS installer works
- [ ] App launches (may need "Run Anyway" for unsigned)
- [ ] File picker navigates C:\, D:\, etc.
- [ ] Load VCD/FST/GHW files
- [ ] All keyboard shortcuts work
- [ ] Config saves to `%APPDATA%\novywave\config.toml`
- [ ] Global history saves to `%APPDATA%\novywave\.novywave_global`

### SmartScreen Bypass (Unsigned Beta)
Document for users: "Click 'More info' → 'Run anyway'"

---

## 6. GHW Format Testing

### Get Test Files
```bash
# Option 1: Generate with GHDL
cd examples/vhdl/counter
make  # Generates counter.ghw

# Option 2: Download samples
# https://github.com/ghdl/ghdl/tree/master/testsuite
```

### Test Checklist
- [ ] GHW file loads without error
- [ ] Hierarchy tree shows scopes
- [ ] Signals extractable and selectable
- [ ] Timeline renders correctly
- [ ] Value formatting works (Hex, Bin, etc.)
- [ ] Large GHW file (~100MB) handles gracefully

---

## 7. Example Projects Testing

### Run Each Example
```bash
# Verilog (requires iverilog)
cd examples/verilog/counter && make && novywave counter.vcd

# VHDL (requires ghdl)
cd examples/vhdl/counter && make && novywave counter.ghw

# SpinalHDL (requires Java 11+, sbt, verilator)
cd examples/spinalhdl/counter && make && novywave counter.vcd

# Amaranth (requires Python 3.8+)
cd examples/amaranth/counter && make setup && make && novywave counter.vcd

# Spade (requires cargo, iverilog)
cd examples/spade/counter && make && novywave counter.vcd
```

### Verify Each Example
- [ ] Makefile runs without errors
- [ ] Waveform file generated
- [ ] File loads in NovyWave
- [ ] Signals visible and correct

---

## 8. CI/CD Release Pipeline ✅ IMPLEMENTED

### CI/CD Files Created

| File | Purpose |
|------|---------|
| `.github/workflows/ci.yml` | Continuous integration (clippy, fmt, build, test examples) |
| `.github/workflows/release.yml` | Multi-platform releases when tags pushed (Linux, macOS, Windows) |
| `.github/workflows/docs.yml` | Auto-deploy documentation to GitHub Pages |
| `.woodpecker.yml` | Self-hostable alternative (Woodpecker CI) |

### GitHub Actions Setup
The release workflow (`.github/workflows/release.yml`) builds for all platforms:
- **Linux**: AppImage, DEB, RPM (ubuntu-22.04)
- **macOS**: DMG for both Intel and Apple Silicon, plus universal binary
- **Windows**: MSI and NSIS installers

### Woodpecker CI Alternative
For self-hosted CI, use `.woodpecker.yml` which provides:
- Matrix builds for Linux (amd64, arm64)
- Gitea/Forgejo release integration
- Documentation deployment pipeline

### Release Process
```bash
# 1. Update version
# Edit: Cargo.toml, src-tauri/Cargo.toml, src-tauri/tauri.conf.json

# 2. Create CHANGELOG entry

# 3. Tag and push
git tag v0.2.0-beta.1
git push origin v0.2.0-beta.1

# 4. CI builds and creates GitHub Release automatically
```

---

## 9. Code Signing (Post-Beta)

### macOS ($99/year Apple Developer)
```bash
# After obtaining Developer ID:
# 1. Add to Keychain
# 2. Update tauri.conf.json:
"bundle": {
  "macOS": {
    "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
  }
}

# 3. Notarize
xcrun notarytool submit NovyWave.dmg --apple-id $APPLE_ID --team-id $TEAM_ID
```

### Windows (Azure Trusted Signing - Free for OSS)
```bash
# 1. Apply at: https://azure.microsoft.com/services/trusted-signing/
# 2. Configure in CI with certificate thumbprint
```

---

## 10. Configuration File Handling ✅ IMPLEMENTED

> **Documentation**: See `docs/configuration.md` for full config system documentation.
> **Example**: See `.novywave_global_example` in repo root for global config format.

### Current Implementation (Fixed)

**Unified config resolution across both modes:**

| Mode | Global Config | Workspace History | Per-Project Config |
|------|---------------|-------------------|-------------------|
| **Tauri (Desktop)** | `dirs::config_dir()/novywave/config.toml` | `dirs::config_dir()/novywave/.novywave_global` ✅ | `{cwd}/.novywave` (checked first) ✅ |
| **Browser (Dev)** | `dirs::config_dir()/novywave/.novywave_global` ✅ | (same as global) | `{workspace}/.novywave` |

### Platform-Specific Global Paths

| Platform | `dirs::config_dir()` resolves to |
|----------|----------------------------------|
| **Linux** | `~/.config/novywave/` |
| **macOS** | `~/Library/Application Support/novywave/` |
| **Windows** | `%APPDATA%\novywave\` |

### Current Issues (Hardcoded Paths)

1. **Browser mode global config uses CWD:**
   ```rust
   // backend/src/main.rs:2229-2231
   fn global_config_path() -> PathBuf {
       INITIAL_CWD.join(GLOBAL_CONFIG_FILENAME)  // ❌ Not portable
   }
   ```

2. **Tauri mode ignores per-project configs:**
   ```rust
   // src-tauri/src/commands.rs:17
   let config_path = config_dir.join("novywave").join("config.toml");
   // Always uses global - never checks for .novywave in workspace
   ```

3. **Different file names:**
   - Tauri: `config.toml`
   - Browser: `.novywave`

### Recommended Config Resolution Order

```
1. Per-project:  {workspace}/.novywave     (if exists)
2. Global:       {platform_config_dir}/novywave/config.toml
```

### Fix Checklist

- [x] **Unify file naming**: `.novywave` for per-project, `.novywave_global` for global history, `config.toml` for Tauri global
- [x] **Browser global path**: Use `dirs::config_dir()` instead of CWD ✅
- [x] **Tauri per-project**: Check workspace for `.novywave` before falling back to global ✅
- [ ] **Test all scenarios:**
  - [x] Fresh install (no config files) → creates global config
  - [x] Global config only → loads global
  - [x] Per-project `.novywave` exists → loads per-project, ignores global
  - [ ] AppImage portable → still saves to `~/.config/novywave/`
  - [x] Config corruption → shows error, doesn't crash

### Files Modified ✅

| File | Change |
|------|--------|
| `backend/src/main.rs` | ✅ Use `dirs::config_dir()` for `global_config_path()` |
| `src-tauri/src/commands.rs` | ✅ Add per-project `.novywave` check before global, use `dirs::config_dir()` for fallback |
| `src-tauri/src/commands.rs` | ✅ Add `load_workspace_history` and `save_workspace_history` commands for feature parity with browser mode |
| `src-tauri/src/lib.rs` | ✅ Register workspace history commands in invoke_handler |

### Testing Matrix

| Scenario | Linux | macOS | Windows |
|----------|-------|-------|---------|
| Global config load | ✅ | ⬜ | ⬜ |
| Global config save | ✅ | ⬜ | ⬜ |
| Per-project load | ✅ | ⬜ | ⬜ |
| Per-project save | ✅ | ⬜ | ⬜ |
| Workspace history load | ⬜ | ⬜ | ⬜ |
| Workspace history save | ⬜ | ⬜ | ⬜ |
| Config migration | ✅ | ⬜ | ⬜ |
| AppImage portable | ⬜ | N/A | N/A |
| Installer mode | ⬜ | ⬜ | ⬜ |

---

## Quick Command Reference

| Task | Command |
|------|---------|
| Dev server | `makers start` |
| Linux build | `makers tauri-build` |
| macOS build | `makers tauri-build` (on macOS) |
| Windows build | `makers tauri-build` (on Windows) |
| Build docs | `cd docs/book && mdbook build` |
| Serve docs locally | `cd docs/book && mdbook serve` |
| Generate signing key | `cargo tauri signer generate` |
| Create release tag | `git tag v0.2.0 && git push origin v0.2.0` |

---

## Priority Order

1. **Generate signing keys** → enables auto-updater
2. **Test Linux/macOS/Windows builds** → platform coverage
3. **Test GHW on all platforms** → new feature validation
4. ~~**Fix config file handling**~~ ✅ → cross-platform reliability (DONE)
5. ~~**Set up GitHub Pages**~~ ✅ → user documentation (workflow created)
6. ~~**Configure CI/CD**~~ ✅ → automated releases (GitHub Actions + Woodpecker)
7. **Code signing** → can defer to v1.0
