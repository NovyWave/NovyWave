# Workspace Management Plan

## Goals
- Let users open any folder as a NovyWave workspace; auto-create `.novywave` if it is missing.
- Persist the most recently opened workspace (and eventually a history list) outside the workspace itself.
- Expose clear UI affordances showing which workspace is active and giving users a fast way to switch.
- Reset application state aggressively on workspace switches to avoid leaking data across projects.
- Treat every relative path stored in `.novywave` as relative to the directory containing that file.

## Current Behavior Review
- Backend config loader (`backend/src/main.rs`) now routes all filesystem work through a `WorkspaceContext`, so `.novywave` paths follow the currently selected root instead of a fixed `CONFIG_FILE_PATH`.
- Plugin helpers (`backend/src/plugins.rs`) resolve relative entries against the active workspace root, avoiding accidental lookups in the agent’s process working directory.
- Frontend consumers receive absolute paths when a config is loaded, but saves go through a relative-normalisation pass so `.novywave` only stores workspace-local entries.
- A lightweight `recent_workspaces.json` (currently in the repository root) records the most recently opened folders, ready to graduate to platform-specific locations later.

## Target User Flow
1. User clicks a new “Open Workspace…” action in the UI.
2. Frontend launches a folder picker and obtains the chosen directory.
3. Frontend sends `UpMsg::SelectWorkspace { root: PathBuf }` to the backend and transitions the UI into a temporary “Workspace loading” state (top bar shows a spinner, panels clear themselves).
4. Backend sets the active workspace root (stored in shared state; see *Implementation Notes*), resolves `<root>/.novywave`, and creates it from defaults if absent.
5. Backend loads the config, validates, and responds with a new `DownMsg::WorkspaceLoaded { root, config }`, then follows it with the existing `DownMsg::ConfigLoaded` so legacy listeners continue to hydrate without change.
6. Backend appends the resolved absolute path to a global `recent_workspaces.json` (initially in the project root; later we will move it to OS-specific app data).
7. Frontend tears down existing session actors, repopulates them from the new config, and renders the workspace chip/banner anchored to the active root. Once the state finishes replaying, normal interactions resume.

### Auto-Creation Feedback
- When the backend creates a new `.novywave`, it includes that fact in a toast/log message so the user is aware of the write.
- If creation fails (permission issues, read-only media, etc.), the backend emits `DownMsg::ConfigError` and the frontend exits the loading state with a visible error banner.

## UI Additions
- Workspace Bar Placement: the top-most layer in `frontend/src/app.rs::NovyWaveApp::root` renders a slim bar that always displays the active workspace. Theme-aware colours keep it readable in both light and dark modes.
  - Workspace Label: shows the current folder name with a tooltip exposing the full path; when a switch is in-flight it drops to “(loading)”.
  - Workspace Actions: `Open Workspace…` prompts for a path, and two quick shortcuts target `test_files/my_workspaces/workspace_a` / `workspace_b` for fast manual testing.
- Global Controls: the Theme toggle and Dock-to-Right/Bottom toggle now live in the workspace bar, keeping panel headers focused on domain-specific actions.
- Loading Indicator: while the backend processes `SelectWorkspace`, the bar shows a subtle “Loading workspace…” hint and buttons ignore additional clicks until the new config arrives.

## Path Handling Rules
- All paths saved inside `.novywave` (opened files, plugin directories, etc.) are stored relative to the workspace root whenever they reside inside the workspace; absolute paths are preserved only for entries outside the workspace.
- Backend normalizes user-selected absolute paths before writing them into the config. The existing helper `BackendPluginBridge::normalized_paths` already canonicalizes file paths; we will augment it to convert to relative when the path is under `workspace_root`.
- Frontend mirrors the same rule when emitting state updates so the backend does not need to guess.
- Whenever the backend resolves a value from `.novywave`, it joins it against the active workspace root unless the entry is already absolute. This matches today’s behavior because our current working directory equals the workspace root, but we will make it explicit so future refactors (like running the backend from another directory) do not change semantics accidentally.

## Implementation Notes
- `WorkspaceContext` (lazy static) records the active root, offers helpers for absolute/relative conversions, and feeds every load/save operation. Config writes clone the in-memory state, relativise paths, and then emit to disk.
- Plugin bridges resolve discovery/watch roots through the same context, so relative entries in `.novywave` work no matter which directory launched the binary.
- The shared schema gained `UpMsg::SelectWorkspace` / `DownMsg::WorkspaceLoaded`, and `ConnectionMessageActor` raises dedicated relays for workspace success/error events.
- `recent_workspaces.json` is rewritten after each successful load (deduped, capped to a handful of entries) and will migrate to platform-specific storage later.
- `NovyWaveApp` clears tracked files and selected variables as soon as a new switch is requested, flips into a loading state, and resets once the backend confirms the new root.
- Sample fixtures live under `test_files/my_workspaces/workspace_a` and `workspace_b` to make manual switches trivial.

## Edge Cases
- Non-writable directory: auto-creation fails with permissions; frontend shows the failure, leaves the previous workspace active, and keeps the recent list unchanged.
- Deleted workspace while active: when the backend receives `SelectWorkspace`, it verifies the directory exists before proceeding; if a previously opened workspace disappears, the recent list entry remains but selecting it will produce a descriptive error.
- Long file paths: relative conversion falls back to absolute if `Path::strip_prefix` fails.
- Cross-platform separators: conversions use `PathBuf`/`Path` to preserve platform conventions automatically.

## Future Enhancements
- Promote the recent workspace registry to platform-native locations (`AppData` on Windows, `~/Library/Application Support` on macOS, `~/.config` on Linux).
- Add multi-entry recent workspace switching in the UI.
- Introduce command palette integration for quick workspace switching.
- Provide an onboarding wizard the first time a folder is opened to explain `.novywave` contents and recommended repo structure.
- Consider allowing multiple concurrent workspace windows once the single-instance flow is stable.
