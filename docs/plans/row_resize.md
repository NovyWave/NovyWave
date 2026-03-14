# Efficient Live Row Resizing

Row resizing is currently faster than before, but it is still much slower than panel divider resizing. The remaining slowness is no longer mainly caused by config saves. The hot path still does too much layout and rendering work, and startup restore still emits duplicate queries, browses, and saves that make the app feel heavier before interaction even begins.

This document defines the next refactor as an implementation-ready spec. It keeps true live row resizing, but changes the drag path so it is treated as a lightweight layout update instead of a full structural timeline update.

## Goals

- Make live row resizing feel close to panel divider resizing.
- Keep waveform rendering visible during drag.
- Prevent row drag from triggering waveform data queries once signals are already loaded.
- Prevent row drag from saving config repeatedly while the pointer is still moving.
- Reduce duplicate startup queries, browses, and config saves so the restored app reaches a stable state faster.
- Preserve the current persisted workspace format and row-height behavior.

## Current Problems

### Drag-Time Hot Path

- `frontend/src/visualizer/timeline/timeline_actor.rs` still rebuilds the full render state from `variables_vec_actor`, `visible_items`, and `row_heights` even when only one row height changed.
- `frontend/src/visualizer/canvas/waveform_canvas.rs` still reacts separately to state updates and dimension updates, so the canvas can render more than once per animation frame.
- Row drag changes vertical layout for all rows below the resized row, but the code does not have a dedicated layout-only update path for that case.
- `frontend/src/selected_variables_panel.rs` still recomputes total content height from the full visible-items snapshot on every row-height change.

### Restore-Time Duplicate Work

- `frontend/src/config.rs::complete_initialization` fires platform-root and directory-browse requests eagerly and independently from file loading.
- Startup currently allows several initial `UnifiedSignalQuery` requests before the UI settles on a stable viewport width and loaded-file state.
- Startup also emits repeated `SaveConfig` calls before restore is fully complete.
- Equivalent expanded-directory paths such as `"/tmp/novywave_ai_workspace/."` and the normalized directory path can both participate in the restore flow.

## Architecture Decisions

### State Ownership

- `SelectedVariables.row_heights` remains the live row-height store.
- `SelectedVariable.row_height` remains the persisted row-height field.
- During drag:
  - only `row_heights` changes
  - `variables_vec_actor` does not change
  - `SelectedVariable.row_height` does not change
- On drag release:
  - commit the final height from `row_heights` into `SelectedVariable.row_height`
  - publish one persisted save if the committed value changed

### Resize Interaction State

- `DraggingSystem` owns transient drag interaction state.
- The effective resize mode is represented by:
  - `DraggingSystem.drag_state.active_divider`
  - `DraggingSystem.pending_row_resize`
- A signal-row drag is active when `active_divider` is `DividerType::SignalRowDivider { .. }`.
- The latest drag-time row-height value is buffered in `pending_row_resize` and applied at most once per animation frame.
- `AppConfig.row_resize_in_progress` stays as the save-suppression signal only.
- Drag interaction state is not persisted and is not stored in config.

### Timeline State Split

- Replace the monolithic `update_render_state()` path with three internal steps in `WaveformTimeline`:
  - `rebuild_structure_snapshot()`
  - `rebuild_layout_snapshot()`
  - `publish_render_state()`
- `StructureSnapshot` contains:
  - visible row ordering without concrete heights
  - variable identity and formatter
  - signal type
  - analog limits
  - transition arcs
  - cursor values
  - markers
- `LayoutSnapshot` contains:
  - canvas width
  - canvas height
  - row heights
  - cumulative row top offsets
  - total logical content height
  - tooltip vertical mapping inputs
- Pure row-height changes call only:
  - `rebuild_layout_snapshot()`
  - `publish_render_state()`
- Structural changes such as add/remove variable, formatter change, signal-type restore, group membership change, marker change, file reload completion, or series-map mutation call all three steps.

### Render Scheduling

- `WaveformCanvas` must move to one request-animation-frame scheduler.
- Any incoming trigger stores the latest pending state and sets dirty flags:
  - `structure_dirty`
  - `layout_dirty`
  - `theme_dirty`
  - `dimensions_dirty`
- One scheduled frame consumes the latest pending state and renders once.
- Render modes:
  - `FullRender`: structure changed, theme changed, markers changed, or drag commit completed
  - `LayoutRender`: only row heights, row offsets, or canvas dimensions changed
- `LayoutRender` still uses exact waveform geometry. It is not a preview-only mode.
- If several drag updates arrive before the next frame, only the latest one is rendered.

### Query and Save Dedupe

- `UnifiedSignalQuery` dedupe is based on a `RequestFingerprint` built from:
  - normalized visible range
  - selected visible variable IDs
  - formatter per variable
  - requested `max_transitions`
  - cursor time
- `request_id` is not part of the fingerprint.
- If the newly requested fingerprint matches the latest in-flight or latest completed fingerprint, do not send another query.
- Config-save dedupe is based on a `ConfigFingerprint` built from the composed shared config payload.
- If the composed config fingerprint matches the last saved fingerprint, do not send `SaveConfig`.

## Detailed Implementation Plan

### 1. Tighten the Current Row-Height Model

- In `frontend/src/selected_variables.rs`:
  - keep `row_heights` as the only live row-height map
  - stop any drag-time code from mutating persisted `SelectedVariable.row_height`
  - add helpers to:
    - read committed row height
    - read live row height
    - commit live row height to persisted state
- `visible_items` must stay height-agnostic.
- `SelectedVariableOrGroup::Variable` should continue to carry the variable identity and metadata, but not become a second live row-height source.

### 2. Make Row Drag Strictly Layout-Only

- In `frontend/src/dragging.rs`:
  - keep the current RAF-coalesced pointer update behavior
  - during drag, update only `row_heights`
  - on drag end, commit only once through a dedicated `commit_live_row_height()` helper
- During drag:
  - no config save request
  - no structural snapshot rebuild
  - no waveform query scheduling

### 3. Split Timeline Structure and Layout

- In `frontend/src/visualizer/timeline/timeline_actor.rs`:
  - add separate mutable/internal snapshots for structure and layout
  - replace the single `update_render_state()` hot path with:
    - structure rebuild from `variables_vec_actor`, `visible_items`, `series_map`, `cursor_values`, and markers
    - layout rebuild from `row_heights`, visible row ordering, and canvas size
    - render-state publish from both snapshots
- Structural subscriptions:
  - selected variables committed snapshot
  - visible-items ordering changes
  - markers changes
  - series-map updates
  - cursor-value updates
  - file reload completion
- Layout subscriptions:
  - `row_heights`
  - `canvas_width`
  - `canvas_height`
- `schedule_request()` must be callable only from structural or viewport/data-affecting paths, never from layout-only row-height changes.

### 4. Replace Reactive Immediate Rendering with a Frame Scheduler

- In `frontend/src/visualizer/canvas/waveform_canvas.rs`:
  - replace the current `select!` loop behavior that renders immediately on state updates and dimension changes
  - keep the latest pending render state in memory
  - queue one RAF callback if one is not already scheduled
  - render once using the latest pending data
- Do not recreate the Fast2D canvas wrapper on ordinary size changes.
- Only recreate the canvas wrapper when the actual DOM canvas element changes.
- Dimension updates should update renderer dimensions and dirty flags, not render immediately.

### 5. Keep the Variables Panel Cheap

- In `frontend/src/selected_variables_panel.rs`:
  - keep per-row height binding from `live_row_height_signal`
  - stop recomputing large derived structures in response to drag-time row-height updates beyond what is needed for row DOM height and total content height
  - compute total content height from:
    - visible row IDs/order
    - current row heights
  - do not tie total content height to `variables_vec_actor` changes during drag

### 6. Make Restore Initialization Deterministic

- In `frontend/src/config.rs`:
  - add an explicit restore phase flag:
    - inactive
    - replaying config
    - loading files
    - waiting for stable canvas width
    - running initial query
    - active
- While restore phase is not `active`:
  - suppress config saves
  - suppress duplicate timeline queries
- Normalize expanded-directory paths before browsing:
  - strip trailing `/.`
  - strip trailing `/` except for root
  - preserve stable first-seen order
  - dedupe identical normalized paths
- Restore sequence:
  1. load config payload
  2. normalize expanded-directory paths
  3. request platform roots once
  4. browse unique normalized expanded directories once
  5. start loading restored files once
  6. restore selected variables and groups once
  7. wait for first non-zero canvas width and loaded-file availability
  8. emit one authoritative initial timeline query
  9. switch restore phase to `active`

### 7. Add Instrumentation First and Keep It Temporary

- Add temporary counters/logging for:
  - row-drag updates received
  - row-drag updates applied
  - full renders
  - layout renders
  - `UnifiedSignalQuery` sends
  - `SaveConfig` sends
  - startup browse requests
  - startup initial-query sends
- Instrument both:
  - panel-divider drag
  - row-divider drag
- Remove or reduce the logging once the performance target is reached.

## Ordered Implementation Slices

Implement in this order:

1. Add instrumentation and request/save fingerprints.
2. Split layout rebuild from structure rebuild in `timeline_actor.rs`.
3. Make row drag commit-only and verify zero drag-time queries/saves.
4. Add RAF-coalesced canvas rendering in `waveform_canvas.rs`.
5. Tighten `selected_variables_panel.rs` so drag-time row-height changes do not cause unnecessary subtree churn.
6. Refactor `complete_initialization()` into the explicit restore phase sequence.
7. Remove temporary diagnostics once the performance target is verified.

Do not start with the startup state machine first. The row-drag hot path must be fixed before restore cleanup.

## Non-Goals

- No preview-only drag mode.
- No config format changes.
- No backend protocol redesign.
- No redesign of panel divider behavior.
- No attempt to make row drag update only one visual row; all lower row offsets remain part of the layout update.

## Success Criteria

- Dragging a row no longer feels dramatically slower than dragging a panel divider.
- A pure row drag emits zero `UnifiedSignalQuery` requests after signals are loaded.
- A pure row drag emits zero `SaveConfig` calls until release, then at most one commit save.
- Drag-time rendering is bounded to at most one applied canvas render per animation frame.
- Startup restore performs one stable initial timeline query path instead of several duplicate initial queries.
- Startup restore does not spam repeated identical config saves.
- The waveform stays visible throughout drag in both browser and Tauri.

## Verification

### Instrumented Checks

- Compare panel-divider drag and row-divider drag with counters for:
  - applied drag updates
  - layout renders
  - full renders
  - canvas dimension changes
  - `UnifiedSignalQuery` count
  - `SaveConfig` count
- Verify row drag on:
  - one analog signal
  - one grouped digital signal
- Verify startup restore on a saved workspace with:
  - groups
  - markers
  - analog limits
  - custom row heights
  - expanded picker directories

### Manual Checks

- Resize analog and grouped digital rows in browser and Tauri.
- Confirm waveforms remain visible during drag.
- Confirm final row height persists after reload.
- Confirm startup restore reaches a stable state with no visible repeated reloading behavior.
- Confirm panel divider behavior is unchanged.

## Assumptions

- Live drag is required.
- Exact final rendering on release is mandatory.
- Drag-time rendering may skip intermediate pointer events, but it may not switch to a preview-only divider.
- Startup cleanup is part of this work because duplicate initialization materially affects perceived performance.
