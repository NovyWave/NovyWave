# NovyWave Configuration Migration Notes

## Global Workspace History

- Workspace picker state (last selected workspace, the three most recent workspace paths, and per-workspace tree view scroll/expansion data) now lives under the `global.workspace_history` section of the shared configuration schema.
- The backend persists this data in a dedicated `.novywave_global` file that sits alongside the traditional `.novywave` workspace files. In production the file is stored in the same platform-specific config directory as the main app config (e.g., `~/.config/novywave/.novywave_global` on Linux).
- The legacy `recent_workspaces.json` file has been removed. Any tooling that previously tailed that JSON store should now read from the TOML config instead.

### TOML Layout

```toml
[global.workspace_history]
last_selected = "/home/user/repos/NovyWave"
recent_paths = [
  "/home/user/repos/NovyWave",
  "/home/user/projects/example",
  "/home/user/work/waveforms"
]

[global.workspace_history.tree_state."/home/user/repos/NovyWave"]
scroll_top = 128.0
expanded_paths = [
  "/home/user/repos/NovyWave/hardware",
  "/home/user/repos/NovyWave/test_files"
]
```

- `recent_paths` is automatically deduplicated and clamped to three entries (most recent first).
- `tree_state` entries are pruned to match the recent list so stale workspaces do not accumulate.

## Workspace `.novywave` Files

- Per-workspace `.novywave` files no longer carry the recent list; they focus strictly on project-specific UI state.
- When the active workspace happens to be the same path as the global config (development mode default), the workspace file may still include a `global` table—this is expected and keeps the single-file workflow intact.
