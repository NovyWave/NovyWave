# Releasing NovyWave

## Version Locations

The version must be updated in **2 places** before a release:

| File | Field | Used by |
|------|-------|---------|
| `Cargo.toml` | `workspace.package.version` | All Rust crates (frontend, backend, shared, src-tauri, src-chrome) via `version.workspace = true` |
| `src-tauri/tauri.conf.json` | `"version"` | Tauri bundler, auto-updater, `novywave --version` in Tauri mode |

The UI version display in the selected variables panel reads from:
- **Web/Chrome mode**: `CARGO_PKG_VERSION` compile-time env (from workspace `Cargo.toml`)
- **Tauri mode**: `tauri::app::getVersion()` (from `tauri.conf.json`)

CI overrides `tauri.conf.json` via `TAURI_BUILD_VERSION` env var, so for automated releases only the tag version matters. But for local/dev builds, both files should match.

## Release Steps

### 1. Update version

```bash
# Edit the version in both files:
#   Cargo.toml          →  [workspace.package] version = "X.Y.Z"
#   src-tauri/tauri.conf.json  →  "version": "X.Y.Z"
```

### 2. Commit and push

```bash
jj describe -m "Bump version to X.Y.Z"
jj new
jj bookmark set main -r @-
jj git push --bookmark main
```

### 3. Trigger the release

**Option A: Git tag (automatic)**
```bash
jj bookmark create vX.Y.Z -r @-
jj git push --bookmark vX.Y.Z
```
The `release.yml` workflow triggers on `v*` tags.

**Option B: Manual dispatch (recommended)**
1. Go to GitHub Actions → Release workflow
2. Click "Run workflow" on the `main` branch
3. CI reads the version from `Cargo.toml` automatically — no version input needed

### 4. Review and publish

CI creates a **draft** release with all artifacts:

| Platform | Artifacts |
|----------|-----------|
| **Linux** | AppImage, .deb, .rpm, novywave-chrome |
| **macOS ARM** | .dmg, novywave-chrome |
| **macOS Intel** | .dmg, novywave-chrome |
| **Windows** | NSIS installer, .msi, novywave-chrome.exe |

Plus `latest.json` for the auto-updater.

1. Go to GitHub Releases
2. Review the draft release
3. Edit release notes if needed
4. Click "Publish release"

## What Each Binary Is

- **NovyWave** (AppImage/DMG/NSIS/MSI) — Main desktop app using Tauri with native webview. Includes auto-updater.
- **novywave-chrome** — Lightweight launcher (~4MB) that opens NovyWave in Chrome/Chromium `--app` mode. Alternative for users experiencing performance issues with native webview (especially WebKitGTK on Linux). Requires Chrome/Chromium installed.

## Verifying a Release

After publishing, verify:
1. Auto-updater works: existing installations detect the new version
2. `latest.json` is accessible at the GitHub release URL
3. All platform binaries download and run correctly
4. `novywave-chrome` detects Chrome and launches NovyWave
