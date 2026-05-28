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

### 3. Run package-smoke preflight

Before creating a draft release, run the package build workflow directly:

1. Go to GitHub Actions -> Build workflow
2. Click "Run workflow" on the `main` branch
3. Keep `release-build` enabled
4. Enter the same version as `Cargo.toml`

This builds the release bundles on Linux, macOS ARM, macOS Intel, and Windows, runs the packaged startup smokes, and uploads the proved artifacts without creating a GitHub release. Each successful platform job also uploads a `smoke-proof-*.json` file listing the artifacts proved by that job.

### 4. Trigger the release

1. Go to GitHub Actions → Release workflow
2. Click "Run workflow" on the `main` branch
3. CI reads the version from `Cargo.toml` automatically — no version input needed

### 5. Review and publish

CI creates a **draft** release with all artifacts:

| Platform | Artifacts |
|----------|-----------|
| **Linux** | AppImage, .deb, .rpm, Chrome launcher AppImage |
| **macOS ARM** | .dmg, Chrome launcher archive |
| **macOS Intel** | .dmg, Chrome launcher archive |
| **Windows** | NSIS installer, .msi, Chrome launcher archive |

Plus `latest.json` for the auto-updater.

Release CI fails before publishing updater metadata unless `TAURI_SIGNING_PRIVATE_KEY` is configured, the key is not the known development key, every updater bundle has a matching `.sig`, every signature verifies against `src-tauri/tauri.conf.json`, and `latest.json` contains exactly the supported updater platforms: `linux-x86_64`, `darwin-aarch64`, `darwin-x86_64`, and `windows-x86_64`.

Release CI also generates `release-manifest-X.Y.Z.json` and uses it to generate the draft release notes. The manifest contains every uploaded filename, size, SHA-256, source commit, platform, architecture, bundle type, updater/signing status, and smoke result. Release creation fails if an expected artifact, smoke proof file, or updater platform is missing.

Apple notarization and Windows Authenticode are not enforced by the current workflow. Until those policies are added, the manifest marks macOS DMG and Windows installer code-signing status explicitly instead of implying operating-system trust signing.

1. Go to GitHub Releases
2. Review the draft release
3. Edit release notes if needed
4. Click "Publish release"

## What Each Binary Is

- **NovyWave** (AppImage/DMG/NSIS/MSI) — Main desktop app using Tauri with native webview. Includes auto-updater.
- **novywave-chrome** — Linux one-file AppImage that opens NovyWave in Chrome/Chromium `--app` mode. Alternative for users experiencing performance issues with native webview (especially WebKitGTK on Linux). It includes the launcher, backend, frontend assets, public assets, API package, and plugins. Requires Chromium 109+ through Chrome, Chromium, Edge, Brave, Vivaldi, Snap, or Flatpak.

## Verifying a Release

After publishing, verify:
1. Auto-updater works: existing installations detect the new version
2. `latest.json` is accessible at the GitHub release URL
3. `release-manifest-X.Y.Z.json` is attached and its checksums match the uploaded files
4. All platform binaries download and run correctly
5. `NovyWave_VERSION_chrome-linux-x64.AppImage` detects Chrome, starts the bundled backend, and launches NovyWave
