# Waveform Reload Plugin Plan

## Research Highlights
- `docs/plugins/authoring_guide.md:1-88` confirms the current plugin surface only exposes `init`, `greet`, and `shutdown`, with host-side filesystem capabilities listed as future work. The new plugin must extend this surface rather than relying on undocumented hooks.
- `shared/src/lib.rs:1322-1459` already defines `PluginConfigEntry.watch` (directories + debounce) but nothing in the backend consumes it, leaving an intentional gap for watcher-based plugins.
- `backend/src/plugins.rs:18-118` maintains a global `PluginManager`, reloads plugins on config load/save, and caches `PluginHandle`s but never calls back into them after `greet()`. We will need additional plumbing to dispatch watcher events and lifecycle callbacks.
- `backend/crates/plugin_host/src/lib.rs:1-170` instantiates Wasmtime components and only calls the exported trio (`init/greet/shutdown`). No host functions are linked today, so watcher support must add host imports plus per-plugin state inside `HostState`.
- `frontend/src/tracked_files.rs:80-160` shows the reload workflow: sending `UpMsg::LoadWaveformFile` reuses the parsing pipeline and updates tracked files, which in turn feeds `MaximumTimelineRange` and timeline caches.
- `frontend/src/app.rs:372-520` wires `ConnectionMessageActor` so `DownMsg::FileLoaded` refreshes tracked files and the timeline pipeline. After a reload, `WaveformTimeline::send_request` (`frontend/src/visualizer/timeline/timeline_actor.rs:1345-1412`) clears caches if there are no selected variables, but we still need an explicit cache bust when a watched file changes.

## Target Behaviour
- Watch all configured waveform paths (or directories) and react within the debounce budget.
- On change, re-parse the affected files, keep existing `TrackedFiles` + `SelectedVariables` entries live, and drive the timeline to refresh without manual user input.
- Avoid fallbacks: surface explicit error states when a reload fails (missing file, parse error, watcher panic).
- Keep actors/relays authoritative—no new global mutables or implicit singletons beyond the existing plugin host.

## TODOs
- [ ] **Design WIT surface:** Extend `plugins/wit/plugins.wit` with host imports for registering directory/file watchers, cancelling them, and requesting waveform reloads; add a guest export for change notifications (e.g., `on_paths_changed(paths: list<string>)`).
- [ ] **Update Wasmtime bindings:** Regenerate bindings in `backend/crates/plugin_host` and the new plugin crate once the WIT surface grows; ensure `HostState` stores watcher handles and any channels required to call back into the component.
- [ ] **Introduce watcher backend:** Add a `notify`-based (or `watchexec`) watcher module under `backend/crates/plugin_host` that honours `PluginWatchConfig.directories` plus `debounce_ms`, and hook it into the plugin lifecycle (`init` registers, `shutdown` drops).
- [ ] **Plumb reload requests:** Expose a host-side API that lets the wasm plugin trigger the existing `load_waveform_file` path. This likely means adding a `PluginHostCommand` channel consumed in `backend/src/plugins.rs` to call into `load_waveform_file` (or enqueue an `UpMsg::LoadWaveformFile` equivalent).
- [ ] **Implement reload plugin crate:** Scaffold `plugins/reload_watcher` (mirroring `hello_world`) that, on `init`, reads `PluginConfigEntry.config` for include/exclude rules, registers watchers, and calls the host reload API for each affected path.
- [ ] **hello_world demo tweak:** Update `plugins/hello_world` so `greet()` demonstrates plugin-specific behaviour instead of a global placeholder constant, keeping the sample aligned with best practices.
- [ ] **Config wiring:** Update `.novywave` templates and `shared::AppConfig` persistence so plugin-specific settings (selected directories, debounce overrides, maybe glob filters) round-trip cleanly. Ensure the new plugin entry is disabled by default until signed off.
- [ ] **Timeline cache invalidation:** Add a relay or signal in `WaveformTimeline` to flush `window_cache` and force fresh `UnifiedSignalQuery` requests when `TrackedFiles` reports a state transition from `Loading` back to `Loaded` for the same file ID.
- [ ] **SelectedVariables resilience:** Verify `SelectedVariables` keeps formats and scope selections after reload; add targeted tests or guards so missing variables emit explicit warnings instead of silently dropping selections.
- [ ] **Diagnostics & logging:** Feed watcher lifecycle and reload attempts into existing logging (backend console + `dev_server.log`) with clear icons, matching the current `🔌`/`🔍` style. Surface actionable errors to the UI via `DownMsg::ParsingError`.
- [ ] **Documentation pass:** Expand `docs/plugins/authoring_guide.md` with the new APIs, usage examples, and guidance on safe watcher scopes. Include manual QA steps so contributors can verify live reloads without guessing.

## Clarified Points
- Watcher scope should focus on the set of opened waveform files stored in `.novywave`, ideally by listening to the `TrackedFiles` actor so the plugin reacts to the live list rather than broad directory globs.
- We assume a single reload plugin instance manages all opened files, so overlapping directory watchers across plugins are out of scope for now.
- Skip extra throttling for rapid save bursts initially; rely on debounce from the watcher configuration and revisit if real-world tools surface issues.
- Reuse the existing error flow: log via `zoon::eprintln` (frontend toast + console) and `eprintln!` on the backend so failures surface consistently without new UI wiring.

## Validation Plan
- Manual: run `makers start`, load `test_files/simple.vcd`, then modify it externally and confirm tracked files flip to `Loading` → `Loaded`, selected variables remain, and the timeline redraws automatically.
- Backend: add integration coverage that simulates a file change event and asserts `load_waveform_file` executes once per debounce window.
- Frontend: watch `dev_server.log` for new plugin warnings/errors; ensure no stray panics when watchers vanish (e.g., directory deleted mid-run).
