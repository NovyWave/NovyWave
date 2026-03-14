# Design Updates Refinement

Five user-visible features complete the gap described in the NovyWave blog post: analog signal rendering, per-signal row resizing, signal grouping, named markers, and platform-aware file-picker roots. This document describes the intended final behavior, persistence model, edge cases, and the verification procedure for both AI-assisted checks and manual maintainer checks.

## Final State Ownership

- `SelectedVariables` is the live owner of per-signal metadata:
  - `formatter`
  - `signal_type`
  - `row_height`
  - `analog_limits`
- `WaveformTimeline` is the live owner of markers and marker ordering behavior.
- `AppConfig` persists snapshots. It must not become a second live state store for row heights, analog limits, groups, or markers.

## Shared Types and Persistence

- `shared::SelectedVariable` persists:
  - `unique_id`
  - `formatter`
  - `signal_type`
  - `row_height`
  - `analog_limits: Option<AnalogLimits>`
- `AnalogLimits` persists as:
  - `auto: bool`
  - `min: f64`
  - `max: f64`
- `WorkspaceSection.signal_groups` stores named group membership and collapse state.
- `TimelineConfig.markers` stores named time bookmarks.
- New fields stay backward-compatible with `#[serde(default)]`.
- Restore-time metadata backfill fills missing `signal_type`, `row_height`, and default analog limits after tracked files load.
- Defaults:
  - digital rows: `30px`
  - real/analog rows: `90px`
  - real/analog limits: `auto = true`

## Feature 1: Analog Signal Rendering

### Behavior

- Real-valued signals render as analog line traces instead of digital rectangles.
- Auto range uses only the visible time window, not the full file.
- Manual limits override auto range when `min < max`.
- Flat signals render as a centered line instead of collapsing or disappearing.
- Invalid numeric samples are skipped without panicking.
- Rendering is clipped to the signal row bounds.

### UI

- The value column for analog rows shows:
  - current value
  - current range summary (`Auto range` or `min .. max`)
  - action to switch back to auto
  - action to edit manual bounds
- Manual bounds are edited through the analog limits dialog.
- Invalid manual bounds are rejected unless both values are finite and `min < max`.

### Acceptance Criteria

- Analog rows are visibly different from digital rows.
- Zooming or panning changes auto-scaled analog traces based on the visible window.
- Manual limits survive workspace reload.

## Feature 2: Signal Row Height Resizing

### Behavior

- Every visible signal row can be resized by dragging its divider.
- Resizing updates the name column, value column, and waveform column immediately.
- Heights clamp to `20..=300`.
- Group headers keep a fixed `30px` height and are not waveform rows.

### Acceptance Criteria

- Dragging a divider changes row height in all three columns without misalignment.
- Heights persist after workspace reload.
- Analog rows default taller than digital rows.

## Feature 3: Signal Grouping

### Behavior

- Groups are named, collapsible, and rendered as their own visible rows.
- Each signal can belong to at most one group at a time.
- Re-grouping removes a signal from its previous group.
- Groups with fewer than two members are dropped automatically.
- Stale group member IDs are removed when variables are removed or restored.

### UI

- Header actions:
  - enter grouping mode
  - select variables for grouping
  - create group with a name dialog
- Group header actions:
  - collapse / expand
  - rename
  - delete group

### Acceptance Criteria

- Group headers reserve vertical space in the name, value, and waveform columns.
- Collapsed members disappear consistently from all three columns and the timeline render state.
- Deleting a group leaves its variables selected but ungrouped.

## Feature 4: Named Markers

### Behavior

- `M` adds a marker at the current cursor time.
- `1`-`9` jump to the nth marker in time order.
- Markers restore from workspace config and refresh render state immediately.
- Marker lines always render when visible.
- Marker labels use three fixed top lanes.
- If every label lane would overlap, the line still renders and only the label text is suppressed.

### UI

- Marker manager lists markers sorted by time.
- Each marker can be renamed, jumped to, or deleted.

### Acceptance Criteria

- Keyboard jumps follow time order, not insertion order.
- Marker names and positions persist after workspace reload.
- Dense marker sets still show the correct vertical lines.

## Feature 5: File Picker Platform Roots

### Behavior

- Platform roots are requested on startup.
- If the file picker opens before roots are known, it requests them again.
- macOS roots:
  - `/`
  - home
  - Desktop
  - Downloads
  - `/Volumes`
- Windows roots:
  - available drives
  - home
  - Desktop
  - Downloads
- Linux roots:
  - `/`
  - home
  - Desktop
  - Downloads
- Browsing `"/"` on Unix shows the real directory contents.
- Old empty or invalid expanded-directory entries are discarded.
- Empty directories show `No waveform files in this directory`.

### Acceptance Criteria

- macOS and Linux no longer fake `"/"` with a hand-picked directory list.
- `/tmp` or another non-home root path is reachable through normal browsing.
- The tree shows platform roots even before any directory is manually expanded.

## Build/Watcher Observation Policy

- Treat the live `makers start`, `makers watch_plugins`, and `makers tauri` terminal output as the canonical source of compile status.
- If another maintainer owns the process, have them share the relevant terminal output directly instead of redirecting it to repo log files.
- When reporting watcher state, quote only the newest relevant output lines rather than dumping the full terminal transcript.

## AI Verification Procedure

Follow this sequence after implementation:

1. Inspect the newest live `makers start` output and list every warning or `error[E...]` line if any exist.
2. Inspect live `makers watch_plugins` or `makers tauri` output too if the touched code path depends on them.
3. Run repository-allowed local checks for touched logic where feasible:
   - `cargo fmt --all`
   - targeted `cargo test` invocations or the workspace test command when allowed by the maintainer workflow
4. Open the app with Browser MCP.
5. Use `window.__novywave_test_api` to inspect:
   - `getSelectedVariables()`
   - `getVisibleRows()`
   - `getMarkers()`
   - `getFilePickerRoots()`
   - `getTimelineState()`
6. Verify file-picker roots:
   - open the file picker
   - confirm roots match the running platform
   - browse `"/"` and confirm real directories are shown
   - open an empty directory and confirm the empty-state message
7. Verify analog rendering:
   - load the analog fixture
   - confirm a Real signal draws as a line trace
   - pan or zoom and confirm auto range reacts to the visible window
   - set manual min/max
   - confirm invalid limits are rejected
8. Verify row resizing:
   - resize one digital row and one analog row
   - confirm alignment across name/value/wave columns
   - reload and confirm persistence
9. Verify grouping:
   - create a named group
   - rename it
   - regroup one signal into another group
   - collapse and expand the groups
   - delete a group
10. Verify markers:
   - add several markers with `M`
   - rename and delete via the marker manager
   - use `1`-`9` jumps
   - reload and confirm persistence
11. If the Tauri desktop app is under test, use the desktop bridge too:
   - `GET /health`
   - `POST /eval`
   - `GET /state/selected-variables`
   - `GET /state/visible-rows`
   - `GET /state/markers`
   - `GET /state/file-picker-roots`
   - `POST /action/set-cursor-ps`
   - `POST /action/add-marker`
   - `POST /action/rename-marker`
   - `POST /action/remove-marker`
   - `POST /action/jump-to-marker`
   - `POST /action/set-row-height`
   - `POST /action/set-analog-limits`
   - `POST /action/create-group`
   - `POST /action/rename-group`
   - `POST /action/toggle-group-collapse`
   - `POST /action/delete-group`
   - `POST /workspace/select`
   - prefer the action endpoints over focus-based desktop input so AI verification does not interrupt the active desktop session
12. Report pass/fail per feature and identify the first failing step if anything breaks.

## Manual Verification Procedure

Follow this checklist from a clean start:

1. Launch the existing maintainer-run development environment.
2. Open the waveform file picker.
3. Confirm the expected platform roots appear for your OS.
4. Browse to `"/tmp"` or another non-home directory and confirm real filesystem navigation works.
5. Open a directory with no waveform files and confirm the empty-state message appears.
6. Load one digital fixture and one analog/real-valued fixture.
7. Find a real-valued signal and confirm it renders as an analog line.
8. Zoom in and out and confirm the analog trace remains readable.
9. Open the analog limits UI:
   - leave it on auto and note the default behavior
   - switch to manual
   - enter a valid `min` and `max`
   - confirm the waveform rescales
   - try an invalid pair where `min >= max` and confirm it is rejected
10. Resize one digital row and one analog row by dragging their dividers.
11. Confirm row alignment remains correct across all three columns.
12. Enter grouping mode and create a named group from at least two signals.
13. Rename that group.
14. Create a second group and move one signal into it.
15. Confirm the moved signal is no longer shown in the first group.
16. Collapse and expand both groups.
17. Delete one group and confirm its signals remain selected.
18. Move the cursor to several points and press `M` to add markers.
19. Open the marker manager:
   - rename one marker
   - delete one marker
   - jump to markers from the manager
20. Press `1`-`9` and confirm jumps follow marker time order.
21. Reload the workspace.
22. Confirm all persisted state restores:
   - row heights
   - analog limits
   - groups
   - collapsed state
   - markers
23. Re-check existing behavior that must not regress:
   - binary/hex formatting
   - Dock to Bottom / Right
   - waveform pan/zoom
   - jump to next/previous change

## Completion Rule

This feature batch is complete only when:

- watcher logs show no unresolved compile problems
- automated checks used for the touched areas pass
- AI verification passes end-to-end
- manual verification passes end-to-end
