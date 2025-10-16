# Workspace History & Dialog State Plan

## Objectives

- Persist Open Workspace dialog state (scroll position, expanded directories, selected path) using the global `.novywave` config.
- Remember the last selected workspace and a capped list of recent workspaces (latest 3 entries).
- Remove the legacy JSON workspace-history file; rely exclusively on the shared configuration pipeline.
- Keep storage simple for migration (no users yet) while documenting the new layout so future tooling understands the split between global vs. per-workspace data.

## High-Level Approach

1. **Extend Global AppConfig**
   - Introduce a new `global` section (top-level field) in the existing global `.novywave` file.
   - Within `global`, add a `workspace_history` struct that tracks:
     - `last_selected: Option<String>`
     - `recent_paths: Vec<String>` (max length 3, most-recent-first, deduplicated)
     - `tree_state: HashMap<String, WorkspaceTreeState>` keyed by absolute workspace path.
   - `WorkspaceTreeState` holds UI metadata we need to restore:
     - `scroll_top: f64`
     - `expanded_paths: Vec<String>` (relative to workspace root, matching FilePicker API semantics).

2. **Shared Types**
   - Define `WorkspaceHistory` and `WorkspaceTreeState` in `shared/src/config.rs` (or equivalent shared module).
   - Ensure both frontend and backend rely on the same serde models to keep serialization symmetric.

3. **Backend Responsibilities**
   - On app start, load the global `.novywave`, defaulting `global.workspace_history` when missing.
   - When sending `ConfigLoaded` to the frontend, include the `workspace_history` payload alongside existing theme/timeline data.
   - Accept updates from the frontend (new last-selected path, updated recent list, tree state deltas) and write them back to the global config file.
   - Drop support for the old custom JSON file altogether (delete read/write paths).

4. **Frontend Responsibilities**
   - Hydrate the Open Workspace dialog domain from the received `workspace_history`:
     - Restore tree expanded state and scroll offset using the same actors we use for the Load Files dialog (see `SelectedFilesSyncActors`, `TreeViewSyncActors`, scroll-position actor).
     - Populate the “Recent workspaces” section from `recent_paths`, falling back to fixtures if empty.
     - Restore the current selection using `last_selected` when the dialog opens.
   - Emit updates to the backend whenever:
     - The user expands/collapses directories.
     - The scroll position changes (debounce or snapshot on dialog close to avoid chatty writes).
     - A workspace is opened (update `last_selected` + recents list before closing dialog).
   - Cap the recents list at 3, removing duplicates (most recent wins).

5. **Actor & Relay Wiring (Frontend)**
   - Reuse the actor patterns from `file_picker.rs`:
     - `WorkspaceTreeViewSyncActors`: newtype wrapper around the existing tree sync but scoped to the workspace picker domain.
     - `WorkspaceScrollPositionActor`: track DOM scroll offset (send updates via relay).
   - Introduce a small `WorkspaceHistoryActor` that:
     - Listens for UI events (open workspace, directory expanded/collapsed, scroll change).
     - Combines them into a `WorkspaceHistoryUpdate` message sent to backend via the existing config channel.
     - Receives the initial history payload and plays it back into the picker domain (one-time initialization flag to avoid wiping user actions).

6. **Backend Actor Adjustments**
   - Extend the config save actor to handle `WorkspaceHistoryUpdate`.
   - When saving, trim `recent_paths` to length ≤ 3 and write to disk immediately (consistent with other config saves).
   - Provide a helper to compute `WorkspaceTreeState` from updates (outside of async loops to keep actor deterministic).

7. **Cleanup**
   - Remove the legacy JSON history file (delete file path constants + any tests referencing it).
   - Update documentation (`docs/actors_relays/novywave/migration_strategy.md`) noting that workspace history is now part of `global.workspace_history`.
   - Mention new config layout in `.claude/extra/project/specs/specs.md` if relevant.

## Open Questions / Follow-ups

- **Tauri parity:** Confirm that the desktop build uses the same config loader so history works identically.
- **Maximum stored tree state:** Do we need to prune old entries in `tree_state` (e.g., keep only data for the current last-selected + recents)? Proposed default: cap to the same 3 paths to avoid unbounded growth.
- **Relative vs. absolute paths:** The backend likely deals with absolute paths already; ensure consistent normalization so we don’t end up with duplicate keys due to casing/symlinks.
- **Dialog lifecycle:** Decide whether to snapshot scroll/expansion only on dialog close or continuously. Recommended: throttle updates (e.g., send at most every few hundred ms) and always send final snapshot when workspace changes.

## Next Steps

- [ ] Define shared `WorkspaceHistory` / `WorkspaceTreeState` structs in `shared/src/config.rs` (with serde defaults).
- [ ] Update global `AppConfig` schema to include a `global` section containing `workspace_history`.
- [ ] Remove legacy workspace-history JSON handling (delete read/write paths, remove file).
- [ ] Adjust backend config loader to populate `workspace_history` when reading global `.novywave`.
- [ ] Send `workspace_history` data alongside existing config payload in `ConfigLoaded`.
- [ ] Add backend handler for `WorkspaceHistoryUpdate` messages (apply deltas, clamp recents, persist).
- [ ] Implement frontend history actor: hydrate picker from payload, manage scroll/expanded sync, emit updates on changes.
- [ ] Integrate tree scroll + expansion actors in Open Workspace dialog (mirroring file picker behavior).
- [ ] Update “Recent workspaces” section to use incoming recents list (fallback to fixtures when empty).
- [ ] Ensure Open Workspace dialog updates `last_selected` + recents before closing/launching workspace switch.
- [ ] Cap stored `tree_state` entries to match recents (remove stale paths).
- [ ] Update relevant docs (`docs/actors_relays/novywave/migration_strategy.md`, `.claude/extra/project/specs/specs.md`) to describe the new `global.workspace_history`.
