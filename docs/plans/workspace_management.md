# Workspace Management Plan

## Goals
- Let users open any folder as a NovyWave workspace; auto-create `.novywave` if it is missing.
- Persist the most recently opened workspace (and eventually a history list) outside the workspace itself.
- Expose clear UI affordances showing which workspace is active and giving users a fast way to switch.
- Reset application state aggressively on workspace switches to avoid leaking data across projects.
- Treat every relative path stored in `.novywave` as relative to the directory containing that file.

## Current Behavior Review
- Backend config loader (`backend/src/main.rs`) reads and writes `.novywave` via the hard-coded relative path `CONFIG_FILE_PATH`. This depends on the process working directory matching the workspace root.
- Plugin helpers (`backend/src/plugins.rs`) fall back to `env::current_dir()` when resolving relative `base_dir` entries, so as long as we keep the working directory synced with the active workspace, plugin resolution remains correct.
- Frontend saves whatever paths were selected by users directly into `AppConfig.workspace.opened_files` without normalizing them relative to the workspace.
- There is no global persistence for “recent workspaces”; the only remembered state lives inside `.novywave`, so switching to a sibling folder drops all context.

## Target User Flow
1. User clicks a new “Open Workspace…” action in the UI.
2. Frontend launches a folder picker and obtains the chosen directory.
3. Frontend sends `UpMsg::SelectWorkspace { root: PathBuf }` to the backend and transitions the UI into a temporary “Workspace loading” state (disables timeline/panels, shows spinner + path).
4. Backend sets the active workspace root (stored in shared state; see *Implementation Notes*), resolves `<root>/.novywave`, and creates it from defaults if absent.
5. Backend loads the config, validates, and responds with a new `DownMsg::WorkspaceLoaded { root, config }`, then follows it with the existing `DownMsg::ConfigLoaded` so legacy listeners continue to hydrate without change.
6. Backend appends the resolved absolute path to a global `recent_workspaces.json` (initially in the project root; later we will move it to OS-specific app data).
7. Frontend tears down existing session actors, repopulates them from the new config, and renders the workspace chip/banner anchored to the active root. Once the state finishes replaying, normal interactions resume.

### Auto-Creation Feedback
- When the backend creates a new `.novywave`, it includes that fact in a toast/log message so the user is aware of the write.
- If creation fails (permission issues, read-only media, etc.), the backend emits `DownMsg::ConfigError` and the frontend exits the loading state with a visible error banner.

## UI Additions
- Workspace Bar Placement: introduce a slim global bar rendered as an additional `Stack::layer` in `frontend/src/app.rs::App::root`. It sits at the top of the viewport above the main panel layout (z-index just below toasts) so the active workspace is visible regardless of dock mode. The bar stretches full width, uses the app theme colors, and contains:
  - Workspace Chip: shows the current folder name with tooltip for the full path, plus a chevron to open a dropdown.
  - Dropdown Actions: `Open Workspace…` (folder picker) and, once implemented, recent workspace shortcuts.
- Global Controls: relocate the Theme toggle and Dock-to-Right/Bottom toggle from the Selected Variables panel header into this bar so all shell-level actions share a single home. This keeps panel headers focused on panel-specific tasks and reduces redundant global controls.
- Loading Overlay: reuses the existing stack layering in `App::root` to display “Loading workspace: <path>” centered in the bar while `WorkspaceLoaded` is pending; the main content receives a translucent scrim to signal the reset in progress.
- Files Panel Header: optionally mirrors the current workspace name in its title tooltip, but the primary interaction lives in the global bar to avoid crowding panel controls.

## Path Handling Rules
- All paths saved inside `.novywave` (opened files, plugin directories, etc.) are stored relative to the workspace root whenever they reside inside the workspace; absolute paths are preserved only for entries outside the workspace.
- Backend normalizes user-selected absolute paths before writing them into the config. The existing helper `BackendPluginBridge::normalized_paths` already canonicalizes file paths; we will augment it to convert to relative when the path is under `workspace_root`.
- Frontend mirrors the same rule when emitting state updates so the backend does not need to guess.
- Whenever the backend resolves a value from `.novywave`, it joins it against the active workspace root unless the entry is already absolute. This matches today’s behavior because our current working directory equals the workspace root, but we will make it explicit so future refactors (like running the backend from another directory) do not change semantics accidentally.

## Implementation Notes
- Introduce a global `WorkspaceContext` (lazy static) storing the currently active root path plus helper methods to compute absolute/relative conversions. `load_config` and `save_config_to_file` will accept the root instead of relying on `CONFIG_FILE_PATH`.
- Update plugin subsystems to read the root from `WorkspaceContext` instead of calling `env::current_dir()`. While we could continue to `set_current_dir`, keeping the root in our state gives us more predictable behavior when multiple watchers are running.
- UpMsg/DownMsg additions require updates in `shared/src/lib.rs` and the frontend message handlers in `frontend/src/config.rs`.
- Connection routing: extend `ConnectionMessageActor` to listen for `WorkspaceLoaded` so the UI can show the active root and trigger the reset while still processing the subsequent `ConfigLoaded`.
- Frontend actor teardown can reuse existing session-reset code paths: emit synthetic “close all” relays, reset timeline actors to defaults, clear caches, then hydrate from the new config.
- The global `recent_workspaces.json` should be read lazily at startup. If it is missing we create an empty structure; on each workspace switch we rewrite it atomically.
- Because the backend already auto-creates `.novywave` today, we only need to ensure the new flow still hits `save_config_to_file` immediately after creation so the defaults are persisted for other processes (plugins, editors).
- For local testing, add sample directories under `test_files/my_workspaces/workspace_a` and `test_files/my_workspaces/workspace_b`. They provide ready-made targets for workspace switching without polluting real project data.

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
