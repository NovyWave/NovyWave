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

## 3. macOS Testing

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

## 4. Windows Testing

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
- [ ] App launches (may need "Run Anyway" for unsigned)
- [ ] File picker navigates C:\, D:\, etc.
- [ ] Load VCD/FST/GHW files
- [ ] All keyboard shortcuts work
- [ ] Config saves to `%APPDATA%\com.novywave.app\`

### SmartScreen Bypass (Unsigned Beta)
Document for users: "Click 'More info' → 'Run anyway'"

---

## 5. GHW Format Testing

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

## 6. Example Projects Testing

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

## 7. CI/CD Release Pipeline

### GitHub Actions Setup
Create `.github/workflows/release.yml`:
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
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
        with:
          tagName: v__VERSION__
          releaseName: 'NovyWave v__VERSION__'
          releaseBody: 'See CHANGELOG.md'
```

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

## 8. Code Signing (Post-Beta)

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
2. **Test GHW on all platforms** → new feature validation
3. **Test macOS/Windows builds** → platform coverage
4. **Set up GitHub Pages** → user documentation
5. **Configure CI/CD** → automated releases
6. **Code signing** → can defer to v1.0
