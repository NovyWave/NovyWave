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
1. Backend canonical path propagation *(done)*  
   - `BackendPluginBridge::normalized_paths` now returns `CanonicalPathPayload` entries and deduplicates on canonical keys.  
   - The plugin → host bridge still exchanges `list<string>` in WIT, but `HostState` adapts those canonical strings back into payloads so the backend can preserve display labels.  
   - `DownMsg::ReloadWaveformFiles` carries `Vec<CanonicalPathPayload>` and backend logging formats with the display path when available.
2. Frontend canonical path model *(done)*  
   - `CanonicalPathPayload` lives in `shared`, and `TrackedFile` stores both canonical and display paths.  
   - `TrackedFiles` maintains canonical-aware snapshots and exposes `reload_existing_paths` for reuse.  
   - Config persistence serialises display strings but rehydrates canonical keys during load.
3. Reload flow refactor *(done)*  
   - `ConnectionMessageActor` routes canonical payloads straight into the new `TrackedFiles::reload_existing_paths`, keeping `process_selected_file_paths` focused on new files.  
   - File picker ingestion normalises paths via `process_selected_file_paths` before inserting.
4. Timeline + selected variables sync *(done)*  
   - Timeline invalidation, cache keys, and selected-variable IDs now use canonical paths end-to-end, ensuring reloads refresh UI state.  
   - Cursor/cache eviction listens to the canonical-key relay emitted by `TrackedFiles`.
5. Migration + cleanup *(done)*  
   - Legacy configs with `opened_files: Vec<String>` migrate to canonical/display pairs on load.  
   - Duplicate tracked entries are collapsed during hydration, and a unit test covers the new config deserialisation path.
