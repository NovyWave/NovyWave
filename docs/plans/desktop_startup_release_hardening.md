# Desktop Startup And Release Hardening Plan

## Purpose

Make the next desktop release resistant to the "nothing happens", blank window, and stuck "Loading workspace..." failure class across Windows, macOS, and Linux.

The immediate public trigger is GitHub issue `#1`: a Windows user installed `NovyWave_0.2.0_x64-setup.exe` on Windows 11 and launching the app produced no UI, no error, and no visible logs.

## Current Assessment

The likely direct cause of the Windows issue is fixed locally:

- `src-tauri/src/lib.rs` now chooses `backend.exe` on Windows.
- Startup failures during runtime preparation or window creation now call `show_startup_error`.
- The backend search path includes bundled resource locations such as `_up_/target/release/<backend>`.

This is not enough to call the release safe. The current code still has package-root workspace risk, incomplete installed-app smoke coverage, platform-specific Chrome package drift, weak startup readiness checks, updater/signing gaps, and stale-plugin packaging risk.

## Release Blockers

### 1. Stop Treating Packaged Resource Directories As Workspaces

**Problem**

Packaged launchers set backend cwd to bundled resources so `frontend_dist`, `_api`, `public`, and plugins resolve. The backend also uses process cwd as the initial workspace root. On a fresh profile with no valid `last_selected`, this can create or load `.novywave` from the app bundle/resource directory.

This is wrong on every OS:

- Windows installed apps can live in protected directories such as `Program Files`.
- macOS `.app/Contents/Resources` should not be the user workspace.
- Linux packages may place resources under `/usr/lib`.
- AppImage mount/extraction directories are transient implementation details.
- Chrome package directories are distribution artifacts, not user workspaces.

**Target Behavior**

- Asset lookup and workspace selection must be separate concepts.
- Packaged resource cwd must never become the default workspace.
- Fresh packaged launch with no valid `last_selected` must enter a visible no-workspace state rather than silently creating `~/.novywave` or a resource-root `.novywave`.
- `.novywave` should only be created after the user explicitly opens a workspace, or in a named app-owned default workspace if the product intentionally adds one later.
- If the platform config directory is unavailable, the app must not fall back to process cwd for global state.

**Implementation Tasks**

- Add explicit backend env vars such as:
  - `NOVYWAVE_RESOURCE_ROOT`
  - `NOVYWAVE_FRONTEND_DIST_DIR`
  - `NOVYWAVE_PUBLIC_DIR`
  - `NOVYWAVE_PLUGIN_DIR`
- Stop deriving workspace root from process cwd in packaged mode.
- In Tauri and Chrome launchers, set resource/asset env vars independently of workspace selection.
- In `backend/src/main.rs`, initialize `WorkspaceContext` from:
  1. valid global `last_selected`
  2. an explicit workspace env var only if product-approved and writable
  3. no-workspace/global-only state that still lets the frontend load roots and open a workspace
- Add resource-root detection so `.novywave` is never auto-created under:
  - Windows install/resource dirs
  - macOS `.app/Contents/Resources`
  - Linux `/usr/lib` package dirs
  - AppImage mount/extraction dirs
  - Chrome package dirs
- If no workspace is active, return a config/default state that makes the UI usable without creating workspace files.

**Files**

- `src-tauri/src/lib.rs`
- `src-chrome/src/main.rs`
- `scripts/package_chrome_appimage.sh`
- `backend/src/main.rs`
- `frontend/src/app.rs`

**Verification Gate**

- Fresh profile packaged run with empty config shows no package extraction/resource path as the workspace.
- Backend cwd may be a resource root, but workspace root is not.
- No `.novywave` is created in a resource cwd.
- Fresh profile packaged run can load platform roots and open a workspace.
- If config/global dirs are unavailable, the app shows a visible error and does not write config into cwd.

**Current Progress**

- Tauri and Chrome launchers pass a user-owned default-workspace hint to the backend so packaged resource cwd is no longer reused as workspace identity.
- In packaged mode with no valid `last_selected`, backend `LoadConfig` now sends a global-only config instead of creating a per-workspace `.novywave`.
- Frontend clears the active workspace path for global-only `ConfigLoaded`, leaving the workspace bar in a no-workspace state with the Open Workspace action available.
- A local packaged-layout smoke with isolated `XDG_CONFIG_HOME`, read-only resource cwd, and `NOVYWAVE_DEFAULT_WORKSPACE` proved:
  - backend cwd remained the packaged resource directory
  - backend logged `No workspace selected; sending global-only config`
  - the UI workspace input showed `No workspace`
  - platform roots loaded through the frontend test API
  - no `.novywave` was written under the resource cwd or default-workspace directory
  - any global history writes are confined to the isolated config directory

### 2. Prove Windows Installer Contains And Starts The Backend

**Problem**

The local runtime now searches for `backend.exe`, but the release build must prove the Windows NSIS/MSI artifacts actually contain it in the installed layout searched by `backend_candidates`. Tauri platform-specific config may merge `tauri.windows.conf.json` automatically, but the release workflow still has to prove the effective Windows bundle includes `backend.exe` after version overrides and all config merges.

**Target Behavior**

- Windows release build bundles `backend.exe` predictably.
- Runtime lookup has tests for the installed-resource layout.
- CI installs or extracts Windows artifacts and asserts the backend exists where runtime lookup expects it.

**Implementation Tasks**

- Prove the effective Windows Tauri config includes `src-tauri/tauri.windows.conf.json`.
- If merge order with the version override is ambiguous, pass/merge the Windows config explicitly in `Makefile.toml`.
- Keep Tauri sidecar/externalBin migration as an optional later simplification, not the default first fix, because sidecars require target-triple-suffixed binaries and extra Tauri configuration.
- Add Windows packaging assertions after build:
  - NSIS silent install contains `backend.exe` in the runtime-searched resource layout
  - MSI administrative/silent install or extraction contains `backend.exe` in the runtime-searched resource layout
  - installed layout matches `backend_candidates`
- Add unit tests for Windows-style backend candidate paths.
- Keep startup error details visible if backend lookup fails.

**Files**

- `Makefile.toml`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `src-tauri/src/lib.rs`
- `.github/workflows/build.yml`

**Verification Gate**

- NSIS artifact is silently installed and the installed app layout contains `backend.exe` where runtime lookup searches.
- MSI artifact is administratively installed or extracted and contains `backend.exe` where runtime lookup searches.
- Windows installed-app smoke run reaches backend readiness and exits successfully.

**Current Progress**

- Windows release CI now silently installs the NSIS artifact and fails if the installed layout does not contain both `NovyWave.exe` and `backend.exe`.
- Windows release CI now administratively extracts the MSI artifact and fails if the extracted layout does not contain both `NovyWave.exe` and `backend.exe`.
- Windows release CI now runs the installed NSIS app and the administratively extracted MSI app through the desktop test bridge, verifying real-window creation, config readiness, platform roots, and the fresh no-workspace state.
- The Windows smoke script is wired into CI but still needs a successful Windows runner execution before issue `#1` can be closed as fixed.

### 3. Add Package Startup Smoke Modes

**Problem**

Current CI mostly proves packages build, not that they start. The Linux Chrome AppImage fake-browser smoke proves the launcher passes `--app=http://127.0.0.1:<port>`, but not that config/workspace/root loading succeeds. Tauri AppImage, deb, rpm, DMG, NSIS, and MSI are not startup-smoked.

The Windows issue was "no UI", so backend-only smoke is not enough.

**Target Behavior**

Every release artifact has non-interactive startup proof before upload.

**Implementation Tasks**

- Add a backend/resource smoke mode:
  - resolve packaged resources
  - start embedded backend
  - verify `/` responds
  - verify `_api/pkg/build_id` responds
  - trigger `LoadConfig`
  - verify platform roots can be produced
  - assert selected workspace/root is not a package resource directory
  - exit with status 0 before interactive UI work
- Add an installed-app WebView smoke mode for Tauri:
  - launch the installed app with isolated platform config/cache locations
  - do not bypass `WebviewWindowBuilder`
  - use the existing desktop test bridge to prove `windowFound: true`
  - query test API state for config/debug and file-picker roots
  - assert no visible startup/loading error remains
- Add equivalent Chrome launcher smoke:
  - start backend
  - verify HTTP routes
  - smoke with fake browser after backend route checks
- Keep smoke modes headless, deterministic, and isolated from the maintainer's real profile.

**Per-Artifact CI Matrix**

- Linux AppImage: run with `APPIMAGE_EXTRACT_AND_RUN=1`.
- Linux deb: extract with `dpkg-deb -x` and smoke the installed layout.
- Linux rpm: extract with `rpm2cpio` or install in CI image and smoke the installed layout.
- macOS DMG: mount DMG, run `.app` smoke, then unmount.
- macOS updater tarball: extract `.app` and smoke it.
- Windows NSIS: silent install, run installed `NovyWave.exe`, smoke, uninstall.
- Windows MSI: administrative install or extraction, run installed `NovyWave.exe`, smoke.
- Chrome AppImage/archive/zip/tarball: extract or run artifact, fake-browser smoke plus backend route checks.

**Files**

- `src-tauri/src/lib.rs`
- `src-tauri/src/desktop_test_bridge.rs`
- `src-chrome/src/main.rs`
- `backend/src/main.rs`
- `.github/workflows/build.yml`

**Verification Gate**

- Each uploaded artifact family has a smoke result.
- Smoke uses isolated config locations such as `XDG_CONFIG_HOME`, `APPDATA`, and `LOCALAPPDATA`.
- Smoke failure prints the failing path, URL, backend error, and startup log path.
- Tauri installed-app smoke proves a real window was created.

**Current Progress**

- Added `scripts/smoke_tauri_package.sh` for Linux package startup smoke through the existing desktop test bridge:
  - launches an executable or AppImage under isolated `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `XDG_DATA_HOME`
  - uses `/health` to prove the real Tauri WebView window exists
  - uses `/state/config-debug` to prove config/server readiness
  - uses `/state/file-picker-roots` to prove platform roots load
  - evaluates the workspace input and fails if it remains on `Loading workspace`
  - fails if a fresh packaged run creates `.novywave` in backend cwd or the default-workspace hint
- Linux release CI now runs the Tauri AppImage smoke before upload with `APPIMAGE_EXTRACT_AND_RUN=1`.
- Linux release CI now extracts and smokes the Tauri deb and rpm packages before upload by running the installed `usr/bin/novywave` layout through the desktop test bridge.
- macOS ARM and Intel release CI now mount and smoke the generated DMG through `scripts/smoke_tauri_macos_package.sh`, verifying the `.app` executable, real-window creation, config readiness, platform roots, and fresh no-workspace state.
- Windows release CI now smoke-tests NSIS and MSI installed layouts through `scripts/smoke_tauri_windows_package.ps1`.
- Local run against the rebuilt `target/release/bundle/appimage/NovyWave_0.2.1_amd64.AppImage` passed the Tauri package smoke.
- Local runs against the rebuilt `target/release/bundle/deb/NovyWave_0.2.1_amd64.deb` and `target/release/bundle/rpm/NovyWave-0.2.1-1.x86_64.rpm` passed the extracted-package Tauri smoke.
- Linux Chrome AppImage CI fake-browser smoke now validates the backend routes while the launcher is alive:
  - fake browser extracts the `--app=http://127.0.0.1:<port>` URL
  - `curl --fail` verifies `/`
  - `curl --fail` verifies `/_api/pkg/build_id`
  - the smoke runs with isolated `XDG_CONFIG_HOME`
- Linux release CI now runs a live-browser Chrome AppImage smoke through a real Chromium browser, waits for the launcher's DevTools endpoint, and verifies the real app URL plus `/_api/pkg/build_id`.
- Local run against `target/release/NovyWave_0.2.1_chrome-linux-x64.AppImage` proved the route smoke logic with the packaged backend.
- Local run against the rebuilt `target/release/NovyWave_0.2.1_chrome-linux-x64.AppImage` passed the live-browser smoke with Google Chrome 146.
- Added `scripts/smoke_chrome_package_directory.sh` for extracted Chrome package directory smoke:
  - verifies launcher/backend/frontend/public/plugin payload shape
  - validates every shipped frontend copy is WEB mode
  - runs the launcher with a fake Chromium executable
  - verifies the launcher starts the packaged backend and serves `/` plus `/_api/pkg/build_id`
  - fails if `.novywave` is created under package resources
- macOS ARM, macOS Intel, and Windows release CI now extract the generated Chrome archives and run the package directory smoke before upload.
- Clean local rebuild passed with `makers build_plugins`, `makers tauri-build`, and `makers chrome-appimage`.
- Clean local Tauri package smokes passed for:
  - `target/release/bundle/appimage/NovyWave_0.2.1_amd64.AppImage`
  - `target/release/bundle/deb/NovyWave_0.2.1_amd64.deb`
  - `target/release/bundle/rpm/NovyWave-0.2.1-1.x86_64.rpm`
- Clean local Chrome AppImage live smoke passed for `target/release/NovyWave_0.2.1_chrome-linux-x64.AppImage`.
- The refreshed Desktop test folder `v0.2.1_next_release` contains only the verified Linux-compatible files: AppImage, deb, rpm, and Chrome AppImage.

### 4. Make Chrome Packaging Correct On Every OS

**Problem**

Linux Chrome AppImage now rebuilds `NOVYWAVE_PLATFORM=WEB` and rejects Tauri/dev-server frontend artifacts. macOS and Windows Chrome packages still copy `frontend_dist`, `_api`, and `frontend/pkg` after the Tauri build, so they can carry the same stale Tauri frontend bug.

**Target Behavior**

All Chrome packages are built from WEB frontend artifacts and validated the same way.

**Implementation Tasks**

- Add shared Chrome package tasks:
  - build WEB frontend immediately before packaging
  - build `novywave-chrome`
  - copy backend, frontend, public, `_api`, plugins from WEB artifacts
  - scan every shipped frontend copy: `frontend_dist`, `_api`, and `frontend/pkg`
  - validate no `window.__TAURI__`
  - validate no hardcoded `127.0.0.1:8082`
- Use the shared task in Linux, macOS ARM, macOS Intel, and Windows CI.
- Smoke every uploaded Chrome artifact with a fake browser.
- If macOS/Windows Chrome packages are not smoke-tested, remove them from uploads and release notes until they are.

**Files**

- `Makefile.toml`
- `scripts/package_chrome_appimage.sh`
- new shared package scripts if needed
- `.github/workflows/build.yml`
- `.github/workflows/release.yml`

**Verification Gate**

- Every Chrome package passes frontend artifact validation.
- Every Chrome package passes launcher/backend smoke.
- Release notes match the Chrome artifacts we actually trust.

**Current Progress**

- Added shared Chrome frontend validation that fails on Tauri bindings or dev-server `127.0.0.1:8082` references in every shipped frontend copy.
- Added a shared Chrome package directory builder for macOS and Windows archives; it copies `_api` and `frontend/pkg` from the WEB `frontend_dist` output instead of reusing stale root copies.
- macOS ARM, macOS Intel, and Windows release CI now run `makers build` immediately before Chrome packaging so the archive payload is rebuilt in WEB mode after the Tauri bundle step.
- Local validation rebuilt WEB artifacts with `makers build` and assembled a Linux-shaped Chrome directory payload with the shared packager.
- macOS ARM, macOS Intel, and Windows Chrome archives are now extracted and smoke-tested with a fake Chromium executable before upload.
- Local `scripts/smoke_chrome_package_directory.sh target/release/chrome-package-test/NovyWave_test_chrome-linux-x64 novywave-chrome-linux-x64` passed.
- Local smoke against the older Desktop `v0.2.1-local-build` Chrome package correctly failed because that package still contains Tauri frontend bindings, proving the validator catches the stale-frontend class.
- `cargo test -p novywave-chrome` passed.
- Clean local `makers chrome-appimage` rebuilt the one-file Linux Chrome AppImage from WEB frontend artifacts; the live Chrome smoke passed with Google Chrome from `/home/martinkavik/.local/bin/google-chrome-stable`.

### 5. Make Startup And Config Errors Visible

**Problem**

Several errors currently only reach console logs. In release builds, users may see a blank window or unchanged loading state without an actionable message.

**Target Behavior**

Startup/config failures produce visible UI and a persistent startup log.

**Implementation Tasks**

- Add Tauri startup log file under user cache/config directory.
- Reuse or align with Chrome startup logging.
- Log:
  - searched backend candidates
  - chosen backend path
  - resource dir
  - backend cwd
  - asset directories
  - selected workspace root or no-workspace state
  - backend readiness route failures
  - config read/parse/write failure
- On Windows, replace PowerShell-only error dialog with a native message box or guaranteed fallback.
- In frontend, surface `ConfigError` as a persistent banner/toast, not only `zoon::println!`.
- The startup/config error banner must include:
  - technical detail
  - startup log path when available
  - an "Open Workspace" action when the app can continue
- Add a startup timeout banner if `WorkspaceLoaded`/`ConfigLoaded` does not arrive.
- Platform-root UI must stop showing only "Loading platform roots..." if roots fail or time out.

**Files**

- `src-tauri/src/lib.rs`
- `src-chrome/src/main.rs`
- `backend/src/main.rs`
- `frontend/src/app.rs`
- `frontend/src/error_display.rs`

**Verification Gate**

- Corrupt `.novywave` produces visible error and log.
- Read-only workspace config produces visible error and log.
- Missing backend produces visible startup dialog and log.
- Platform-root timeout produces visible error and leaves workspace opening usable.

**Current Progress**

- Tauri startup errors now append to a persistent `novywave-tauri-startup.log` under the platform cache/log directory and include that path in the visible startup dialog.
- Embedded backend spawn failures now report the backend path, cwd, frontend asset env vars, `COMPRESSED_PKG`, and selected port.
- Tauri backend readiness now detects an embedded backend process that exits before accepting connections and reports that exit status instead of waiting for a generic timeout.
- Frontend `ConfigError` messages now show a persistent configuration-error toast instead of only writing to the console, and successful config/workspace loads dismiss the stale error.
- Initial frontend startup now has a 10-second workspace/config watchdog; if `WorkspaceLoaded` or the no-workspace `ConfigLoaded` equivalent never arrives, it clears the loading state and shows a persistent visible configuration error that points the user to Open Workspace.
- Platform-root startup now has a 10-second watchdog; if roots never resolve, the file picker and workspace picker enter an empty-roots state and show a visible directory error instead of staying on "Loading platform roots...".
- Local focused Tauri lib tests passed for backend candidate handling and Linux WebKit profile parsing; local `makers tauri-build` and AppImage startup smoke passed after this change.
- Local `makers build` passed after the frontend config-error toast and startup watchdog changes.

### 6. Harden Config Recovery

**Problem**

Invalid or old config can return `ConfigError` and leave the UI partially initialized. Missing top-level sections may fail deserialization before `validate_and_fix` can run.

**Target Behavior**

Recoverable config problems should not strand the app.

**Implementation Tasks**

- Split recovery paths for:
  - per-workspace `.novywave`
  - global `.novywave_global`
- Add serde defaults for safe top-level config sections, or implement a migration loader that can tolerate missing `app`, `ui`, and `workspace` sections.
- On invalid TOML:
  - move the bad file to a unique backup path
  - if backup fails, do not overwrite the original
  - load defaults
  - show a visible warning with the backup path or backup failure
- On permission denied:
  - do not overwrite
  - show visible error
  - allow workspace switch
- Keep global workspace history corruption isolated from per-workspace config.

**Files**

- `shared/src/lib.rs`
- `backend/src/main.rs`
- `frontend/src/app.rs`
- `frontend/src/error_display.rs`

**Verification Gate**

- Tests cover invalid TOML, partial old config, missing top-level sections, permission denied, corrupt global history, invalid `last_selected`, and no home/config directory.

**Current Progress**

- `AppConfig` now deserializes missing top-level `app`, `ui`, and `workspace` sections with defaults, so partial old config files no longer fail before migration/validation can run.
- Invalid per-workspace `.novywave` files are moved to a unique backup path before default workspace settings are loaded.
- Invalid `.novywave_global` files are moved to a unique backup path before default workspace history is used for the session.
- Config recovery warnings are sent to the frontend after the usable config loads, so the UI can continue while still showing the backup path or recovery failure.
- Focused tests passed for missing top-level config sections and unique invalid-config backup paths.

### 7. Make Platform Roots Deterministic

**Problem**

The stuck-loading class includes root loading. Roots are requested after server readiness, directory probes can misclassify permission errors, and Windows drive probing can be slow or misleading.

**Target Behavior**

`GetPlatformRoots` always produces either `PlatformRoots` or a visible roots error. It never leaves the picker indefinitely loading.

**Implementation Tasks**

- Add a roots error message if platform roots cannot be determined.
- Replace Windows `A:\` through `Z:\` probing with native drive enumeration where practical.
- Use `dirs::desktop_dir()` and `dirs::download_dir()` instead of assuming `home/Desktop` and `home/Downloads`.
- Use `try_exists()` to preserve permission errors instead of reporting them as "not found".
- Surface `DirectoryError` in the tree and as a transient visible error when it blocks a requested action.
- Either implement `BatchDirectoryContents` handling in the frontend or remove the unused message path.

**Files**

- `backend/src/main.rs`
- `frontend/src/app.rs`
- `frontend/src/file_picker.rs`
- `shared/src/lib.rs`

**Verification Gate**

- Platform roots load on Linux, macOS, and Windows smoke runs.
- Roots unavailable/permission-denied cases produce visible UI, not indefinite loading.
- Windows smoke does not hang on inaccessible removable/network drives.

**Current Progress**

- Platform root discovery now returns both roots and inspection warnings; if no roots are available, the backend sends `PlatformRoots([])` plus a `DirectoryError` instead of leaving the frontend in an indefinite loading state.
- Root discovery and directory browsing now use explicit metadata inspection, preserving permission and access errors separately from missing paths.
- Desktop and Downloads quick-access roots now use `dirs::desktop_dir()` and `dirs::download_dir()` instead of assuming `home/Desktop` and `home/Downloads`.
- `DirectoryError` now creates a visible frontend toast as well as updating file-picker tree state.
- File-picker and workspace-picker trees show a terminal empty-roots state instead of "Loading platform roots..." once roots resolution has completed with no available folders.
- Focused backend tests passed for directory availability classification, and `makers build` passed after the frontend roots UI changes.

### 8. Rebuild And Prove Plugin Artifacts

**Problem**

Release packaging currently copies `plugins/dist`, but release CI does not rebuild plugins. This can ship stale or missing plugin wasm artifacts.

**Target Behavior**

Plugin artifacts are rebuilt and verified before every package upload.

**Implementation Tasks**

- Install plugin build prerequisites in CI, including `cargo-component` if required.
- Run `makers build_plugins` before all Tauri and Chrome packaging.
- Assert exact nonempty plugin artifacts exist:
  - `hello_world_plugin.wasm`
  - `reload_watcher_plugin.wasm`
  - `files_discovery_plugin.wasm`
- Ensure plugin discovery failure cannot block `WorkspaceLoaded` or `ConfigLoaded`.

**Files**

- `Makefile.toml`
- `.github/workflows/build.yml`
- `backend/src/plugins.rs`

**Verification Gate**

- CI fails if any expected plugin artifact is missing or empty.
- Package smoke proves plugin discovery failure does not block startup.

**Current Progress**

- Added `scripts/verify_plugin_artifacts.sh` to fail when `hello_world_plugin.wasm`, `reload_watcher_plugin.wasm`, or `files_discovery_plugin.wasm` is missing or empty.
- `makers build_plugins` now runs the verifier after copying component outputs to `plugins/dist`.
- Linux, macOS ARM, macOS Intel, and Windows release CI now install `cargo-component` and run `makers build_plugins` before release packaging.
- Local `makers build_plugins` passed and regenerated all three expected plugin artifacts as nonempty wasm files.

### 9. Fix Updater And Distribution Signing

**Problem**

Updater artifacts are enabled, but release CI only warns when signing secrets are missing. Local/dev signing defaults and configured pubkey mismatches can produce unusable or untrustworthy update artifacts.

**Target Behavior**

Updater artifacts are either correctly signed and verified, or not produced.

**Implementation Tasks**

- Remove hardcoded dev signing defaults from release paths.
- Fail release builds if updater signing secrets are missing while updater artifacts are enabled.
- Reject known dev/default signing keys in release mode.
- Verify every generated updater signature against the configured public key before generating `latest.json`.
- Fail if `latest.json` omits an expected updater platform.
- Decide and document Apple notarization and Windows Authenticode policy.

**Files**

- `Makefile.toml`
- `src-tauri/tauri.conf.json`
- `.github/workflows/build.yml`
- `.github/workflows/release.yml`
- `docs/src/development/releasing.md`

**Verification Gate**

- Release CI verifies every `.sig` against the configured pubkey.
- `latest.json` has exactly the supported updater platform keys.
- Release fails rather than publishing unsigned or mismatched updater artifacts.

**Current Evidence**

- Hardcoded development updater signing defaults were removed from local Tauri bundle tasks.
- Local Tauri bundles disable updater artifact creation when `TAURI_SIGNING_PRIVATE_KEY` is absent, instead of generating signatures with a development key.
- Release CI now fails before packaging if `TAURI_SIGNING_PRIVATE_KEY` is missing, if it matches the known development key fingerprint, or if the password is the known development password.
- Required updater artifact uploads now fail when generated updater bundles or signatures are missing.
- `scripts/generate_updater_latest.py` verifies every updater signature with `minisign` against the configured updater public key before writing `latest.json`.
- The updater metadata generator requires exactly the supported updater platforms: `linux-x86_64`, `darwin-aarch64`, `darwin-x86_64`, and `windows-x86_64`.
- Local checks passed for script syntax, workflow YAML parsing, validator rejection paths, and synthetic signed updater artifacts for all supported updater platforms.
- Existing older local updater signatures are correctly rejected against the configured updater public key, which confirms the previous mismatch would not be published by the hardened release flow.
- Clean local unsigned `makers tauri-build` printed `TAURI_SIGNING_PRIVATE_KEY is not set; disabling updater artifacts for this local bundle.` and produced no `.sig` files.
- The local Tauri bundler still warns that `__TAURI_BUNDLE_TYPE` could not be patched into the binary for deb, rpm, and AppImage. This does not block local manual package smoke tests, but final updater confidence still requires GitHub release CI with real signing secrets.

### 10. Add Release Artifact Manifest And CI Proof

**Problem**

Release notes and uploads can drift from what CI actually built and proved.

**Target Behavior**

Release creation is generated from machine-readable proof, not from hand-written assumptions.

**Implementation Tasks**

- Each build job emits an artifact manifest with:
  - artifact filename
  - size
  - sha256
  - source commit
  - OS/architecture
  - bundle type
  - updater/signing status
  - smoke result
  - startup log path or attached smoke log artifact
- `release.yml` collects manifests and fails if:
  - expected artifacts are missing
  - any uploaded artifact lacks a smoke result
  - release notes mention a missing or unproved artifact
  - updater metadata does not match uploaded artifacts
- Use `if-no-files-found: error` for required uploads.

**Files**

- `.github/workflows/build.yml`
- `.github/workflows/release.yml`
- `docs/src/development/releasing.md`

**Verification Gate**

- Draft release notes are generated from the manifest.
- Artifact count, checksums, smoke status, and updater status are visible in CI.

**Current Progress**

- Added `scripts/generate_release_manifest.py` to validate the complete expected release file set, verify `latest.json` updater platform coverage, compute file size and SHA-256, and write a machine-readable release manifest.
- Release CI now generates `release-manifest-X.Y.Z.json` from collected release files and creates draft release notes from that manifest instead of a hand-written artifact table.
- The manifest includes source commit, platform, architecture, bundle type, updater/signing status, and smoke result for each expected release artifact.
- Release creation now fails if expected release files are missing, unexpected non-signature files appear, or updater metadata omits a supported updater platform.
- The release guide documents the manifest and explicitly states that Apple notarization and Windows Authenticode are not enforced yet.
- Local checks passed for Python syntax, workflow YAML parsing, and a synthetic complete release directory.

## Suggested Execution Order

1. Fix default workspace/resource cwd separation.
2. Add visible config/startup errors and Tauri startup log.
3. Add backend/resource and installed-WebView smoke modes.
4. Prove Windows backend packaging with installed-artifact CI assertions.
5. Unify Chrome package creation for every OS or stop publishing unproven Chrome packages.
6. Harden config recovery.
7. Make platform roots deterministic.
8. Rebuild and prove plugin artifacts.
9. Fix updater and distribution signing.
10. Add release artifact manifest and CI proof.
11. Rebuild/retest release artifacts.
12. Update docs and GitHub issue response.

## Required CI Gates Before Release

- `makers build_plugins` ran and expected plugin artifacts are nonempty.
- `TAURI_BUILD_VERSION=<version> makers tauri-build` passed on each OS.
- `TAURI_BUILD_VERSION=<version> makers chrome-appimage` passed on Linux.
- Shared Chrome packaging task passed on every OS that uploads a Chrome artifact.
- Linux Chrome AppImage live smoke with real Chromium passed.
- Linux Chrome AppImage fake-browser smoke passed.
- Tauri package smoke passed for every uploaded package family.
- Windows installed-artifact smoke proves `backend.exe`, real window creation, startup config, and platform roots.
- macOS `.app` smoke proves backend starts from packaged resources and creates a real window.
- Updater signatures verify against configured pubkey.
- `latest.json` has exactly the supported updater keys.
- Artifact manifest includes filename, sha256, source commit, bundle type, signing status, and smoke result for every upload.
- Release notes are generated from or checked against the artifact manifest.
- `cargo test -p novywave-chrome`
- Focused backend/shared tests for config recovery and workspace root selection.

## Required Scenario Tests

- Temp-profile first run.
- Read-only cwd/resource cwd.
- Read-only selected workspace.
- Corrupt `.novywave`.
- Corrupt `.novywave_global`.
- Missing top-level config sections.
- Invalid `last_selected`.
- No home/config directory.
- Platform roots unavailable or permission denied.
- Frontend loading timeout/error rendering through DOM or desktop test bridge.
- Missing backend binary.
- Existing user profile does not affect smoke results.

## Release Issue Response Criteria

Only close or respond as fixed on GitHub issue `#1` after a Windows release artifact built from the fixed commit proves:

- exact uploaded installer filename and sha256
- NSIS silent install proof, or MSI install/extraction proof for the artifact being recommended
- installed layout contains `backend.exe` in the path searched by runtime
- app starts as a normal user after install with an isolated fresh profile
- a real WebView window is created
- startup log is written and attached or linked from CI
- `WorkspaceLoaded` or no-workspace equivalent arrives
- platform roots load
- no blank/no-window failure occurs when backend is missing or misconfigured; instead a visible error appears
