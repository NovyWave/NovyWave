# Reload Watcher Flow Hardening

## Pain Points
- Reload notifications arrive with canonical file system paths while the frontend often stores the original dialog string. Pure string comparisons fail, leaving duplicate `TrackedFile` entries and perpetual `Loading…` states.
- `TrackedFiles::reload_file` rebuilds the vector by removing/reinserting entries, which is brittle when the key string differs; ordering and caching depend on the exact text match.
- The timeline invalidates caches by string-matching `SelectedVariable.unique_id` prefixes, so any drift between the reload path and the stored ID stops queries from refiring.
- `process_selected_file_paths` mixes “new file” intake with reload handling, causing duplicate enqueueing when a path resolves differently.

## Proposed Architecture
- Introduce a `CanonicalPath` wrapper shared across backend + frontend that stores both the canonical key (absolute, normalised, symlink-resolved) and the original display path.
- Store `TrackedFiles` in a `HashMap<CanonicalPath, TrackedFile>` for lookups while retaining a presentation-ordered `Vec` for UI. All mutating operations key off the canonical form.
- Enrich `DownMsg::ReloadWaveformFiles` to include canonical IDs alongside the displayable strings so the frontend never recomputes canon paths.
- Split reload handling from new-file ingestion: `ConnectionMessageActor` hands canonical reload IDs to a dedicated `TrackedFiles::reload_existing_paths` helper; the dialog path continues to call the helper after normalising.
- Expose a lightweight relay from `TrackedFiles` publishing `(canonical_path, FileState)` updates; have the timeline rely on that feed instead of parsing strings from selected-variable IDs.

## Implementation TODOs
1. Backend canonical path propagation  
   - Extend `BackendPluginBridge::normalized_paths` (backend/src/plugins.rs) to yield tuples `(canonical_key, display_path)` instead of bare `PathBuf`.  
   - Update plugin host WIT (`plugins/reload_watcher/wit/plugin.wit`) and the host bridge traits so `reload_waveform_files` accepts the tuple payload.  
   - Modify `DownMsg::ReloadWaveformFiles` in `shared/src/lib.rs` to carry `Vec<CanonicalPathPayload { canonical: String, display: String }>` and adjust backend senders accordingly.  
   - Keep the plugin’s log output readable by formatting with `display`.
2. Frontend canonical path model  
   - Add `CanonicalPath` struct to `shared` (and expose serde support).  
   - Update `TrackedFile` to store both canonical and display path values; ensure `create_tracked_file` initialises both.  
   - Refactor `TrackedFiles` actor (frontend/src/tracked_files.rs) to maintain a `HashMap<String, TrackedFile>` keyed by canonical path plus an ordered `Vec<String>` for UI iteration.  
   - Adjust config persistence (`frontend/src/config.rs`) to read/write `display` paths but hydrate canonical keys on load.
3. Reload flow refactor  
   - Add `TrackedFiles::reload_existing_paths(&[CanonicalPath])` that mutates the map in place, updates state, and enqueues parse requests.  
   - Change `ConnectionMessageActor` to pass canonical payloads directly, bypassing `process_selected_file_paths` for reload scenarios.  
   - Limit `process_selected_file_paths` to new-file ingestion; canonicalise picker results via the shared helper before insertion.
4. Timeline + selected variables sync  
   - Expose a relay from `TrackedFiles` broadcasting `(canonical_key, FileState)` whenever a file state changes.  
   - Update the timeline actor to invalidate caches by canonical key, and swap all string prefixes in `SelectedVariable.unique_id` to use the canonical key while storing the display path for UI labels.  
   - Audit cursor map, cache keys, and request bookkeeping to confirm they use the canonical key end-to-end.
5. Migration + cleanup  
   - On config load, convert legacy `opened_files: Vec<String>` into canonical/display pairs (skip writing until the new struct is in place).  
   - Provide a one-shot migration that removes stale duplicate entries in `TrackedFiles` after hydration.  
   - Add an async integration test that simulates a reload with canonical vs. display path differences to ensure the timeline transitions out of `Loading`.
