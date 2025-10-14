# Files Discovery Plugin Plan

## Research Highlights
- `.novywave:76-92` already wires plugin entries through `plugins.entries`, so the discovery plugin should reuse `PluginConfigEntry` rather than introducing a parallel schema.
- `shared/src/lib.rs:1353-1459` defines `PluginConfigEntry` and `PluginWatchConfig`, giving us a serde-ready spot for per-plugin settings and watcher tuning.
- `backend/src/plugins.rs:1-270` shows how the backend bridge normalizes paths, manages watchers, and translates plugin callbacks into `DownMsg::ReloadWaveformFiles`; new behaviour must slot into this manager.
- `backend/crates/plugin_host/src/lib.rs:1-210` maps plugin IDs to Wasmtime worlds and exposes host helpers (`get_opened_files`, `register_watched_files`, `reload_waveform_files`); the discovery plugin needs a new world with an `open_waveform_files` host import.
- `plugins/reload_watcher/src/lib.rs:1-60` demonstrates current plugin ergonomics: configuration goes through `.config`, host helpers are accessed under `novywave::<plugin>::host`, and startup re-registers watchers via `refresh_opened_files`.

## Target Behaviour
- Allow users to supply gitignore-style patterns (relative to the `.novywave` file) that identify directories/files to monitor for new waveform dumps.
- On startup, expand the patterns, filter to supported extensions (`.fst`, `.vcd`), and open any matching files that are not already loaded.
- Watch the minimal directory set implied by the patterns; when new filesystem entries appear, open each fresh file exactly once and skip duplicates already tracked by the session.
- Leave reload responsibilities to `reload_watcher`; if a discovered file is later modified, no extra action is required from this plugin.
- Keep failure states explicit: log configuration mistakes, invalid patterns, and filesystem errors through the existing plugin log channel.

## Configuration Design
- Reuse the existing `[plugins.entries]` config entry with `id = "novywave.files_discovery"` (disabled by default until tested).
- Inside `[plugins.entries.config]`, store:
  - `patterns = ["test_files/to_discover/**/*.vcd", "fixtures/**/*.fst"]`
  - `debounce_ms = 250` (optional override; fallback to `PluginWatchConfig::default_debounce_ms()`).
  - `allow_extensions = ["fst", "vcd"]` (defaults to the built-in list so we can extend to `ghw` later without schema churn).
  - `base_dir = "relative/or/absolute/path"` (optional; defaults to the directory containing `.novywave`).
- The plugin should resolve patterns relative to the directory containing `.novywave` unless `base_dir` is supplied, mirroring how `gitignore` files behave.
- When a pattern resolves to a directory, recursively monitor it; when it resolves to files, watch their parent directories.
- Skip symlinks to keep the first iteration simple and avoid recursion traps.
- Leave `PluginConfigEntry.watch` unset; directory targets are derived from `patterns` to avoid duplication between config tables.

## Runtime Flow
1. **Initialization (`init`)**
   - Parse config table into a strongly typed struct (`DiscoveryConfig`).
   - Build an `ignore::gitignore::Gitignore` matcher for each pattern.
   - Compute seed directories for watchers (unique, absolute).
   - Enumerate current matches using `ignore::WalkBuilder` and issue `open_waveform_files` host calls for unseen files.
   - Register directory watchers with debounce derived from config.
2. **Refresh (`refresh_opened_files`)**
   - Store the latest opened file set from `host::get_opened_files()` to avoid double-opens.
   - Rebuild matcher state if the set of already opened files grows outside our catalogue.
3. **Directory events (`paths_discovered`)**
   - Canonicalize incoming paths, filter by extension, and re-check against ignore matchers to ensure the new file actually matches a pattern.
   - Discard paths already in the opened set; for the rest, call `open_waveform_files`.
4. **Shutdown**
   - Clear watchers and cached matcher state.
   - Log a concise summary (e.g., total files opened during plugin lifetime).

## Implementation Steps
- **WIT surface**
  - Extend `plugins/plugin_example.wit` with a new `files-discovery` world that imports logging, watcher-control, and a new `open-waveform-files` host function.
  - Export lifecycle callbacks: `init`, `refresh-opened-files`, `paths-discovered`, and `shutdown` for symmetry with reload watcher semantics.
  - Generate new bindings under `plugins/files_discovery/src/bindings.rs` and `backend/crates/plugin_host`.
- **Host bridge**
  - Add `fn open_waveform_files(&self, plugin_id: &str, paths: Vec<CanonicalPathPayload>)` to `HostBridge`.
  - Make `BackendPluginBridge` broadcast `DownMsg::LoadWaveformFile` (using the same pipeline as manual loads) for each path, deduplicating and ensuring paths are canonical.
  - Wire the new API through `PluginHost`, `PluginHandle`, and `PluginWorld` enum; ensure watcher callbacks dispatch to the discovery runtime instead of the reload handler.
- **Plugin crate**
  - Scaffold `plugins/files_discovery` mirroring `reload_watcher`, with a `DiscoveryPlugin` struct that caches:
    - `HashSet<String>` of opened canonical paths.
    - Compiled matcher (`Gitignore`, `GlobSet`, or custom) plus the root directory list.
  - Implement config parsing with serde (`DiscoveryConfig`), validation (`normalize` absolute paths, extension allowlist), and logging for user mistakes.
  - Use `host::register_watched_directories` (new helper) instead of raw files because discoveries are directory-driven.
- **Backend manager**
  - Update `PluginManager::handle_watched_files_changed` to route events based on plugin world (reload vs discovery).
  - Introduce a new dispatch helper `dispatch_directory_created` that packages the canonical path payloads.
  - Ensure `PluginManager::reload()` seeds the discovery plugin with the latest opened file set so it can avoid reopening.
- **Config tooling**
  - Add default entry stub to `.novywave` comments/templates (disabled with example patterns).
  - Update `docs/plugins/authoring_guide.md` with discovery-specific guidance.
  - Note interaction expectations in `docs/actors_relays/novywave/migration_strategy.md` if the plugin affects migrations.

## TODOs
- [x] Describe the new `files_discovery` WIT world and regenerate bindings in `plugin_host` and the plugin crate.
- [x] Extend `HostBridge`/`BackendPluginBridge` with `open_waveform_files` and directory watcher support.
- [x] Implement the `plugins/files_discovery` crate with config parsing, matcher, and watcher handling.
- [x] Teach `PluginManager` to route watcher notifications to the discovery runtime (and to keep reload routing intact).
- [x] Add user-facing documentation covering configuration, limitations, and manual QA steps.
- [ ] Populate `test_files/to_discover` with sample `.vcd`/`.fst` fixtures referenced in manual testing docs.
- [ ] Write integration coverage that simulates directory creation events and asserts `DownMsg::LoadWaveformFile` is broadcast once per new file.
- [ ] Verify `dev_server.log` after wiring the plugin to capture any warnings from startup enumeration.

## Validation Plan
- Manual QA: run `makers start`, enable the plugin with `patterns = ["test_files/to_discover/**/*.vcd"]`, drop a new `.vcd` file in that directory, and confirm it appears in the UI without manual loading.
- Backend test: mock a watcher event via `BackendPluginBridge::dispatch_watched_files_changed` and assert a synthetic discovery plugin calls `open_waveform_files` once.
- Config round-trip: load and save `.novywave` to ensure the new config table persists (including default values when fields are omitted).

## Notes
- Discovery keeps a conservative extension allowlist (`fst`, `vcd`) so we can extend to `ghw` later without breaking validation.
- Directory symlinks are ignored to avoid accidental infinite recursion; regular file symlinks are treated like files once canonicalized.
- Future enhancement: share watcher infrastructure with `reload_watcher` once both plugins stabilize, avoiding duplicated watcher registrations on the same directories.
