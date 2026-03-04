# Design Updates Refinement — Feature Spec

Five features traced to user research interviews and issue tracker findings. Each addresses friction that real users reported.

---

## Feature 1: Analog Signal Rendering

**Why**: Physicist: "height extensions feel wonky, values normalized to current window, cannot set limits." Backend parses Real values via `wellen` (floats with 6 decimal places, sent as strings). Frontend renders them as digital text rectangles — no curves, no Y-axis.

**What**: Polyline/step-function rendering for Real signals in Fast2D canvas. Y-axis scale in the value column area. User-settable min/max limits (not just auto-normalize to visible window). Smooth interaction with zoom/pan.

**How**:
- `frontend/src/visualizer/canvas/rendering.rs`: New rendering path in `draw_pixel_run` / `add_signal_segments` — detect Real signal type via `signal_type` field in `VariableRenderSnapshot` (needs threading through from `Signal` struct). Parse float strings back to f64, compute Y positions from min/max scaling within row height. Use Fast2D `Line` primitive (exists at `crates/fast2d/src/object2d/line.rs`, accepts `Vec<(f32, f32)>` — currently unused in codebase).
- `frontend/src/selected_variables_panel.rs`: Add Y-axis min/max controls per variable (follow-up, not first version).
- `shared/src/lib.rs`: Add `AnalogLimits { min: f64, max: f64, auto: bool }` to per-variable config.
- `.novywave` config: Persist limits per signal.

**Key challenge**: Two-pass rendering needed — first pass finds min/max of visible transitions for auto-scaling, second pass draws. The pixel-bucketing approach (`PixelValue::Single`/`Mixed`) needs adaptation for analog: show min-max range per pixel instead of "Mixed" marker.

**Difficulty**: Medium. Fast2D `Line` is ready, backend data pipeline works. Main work is the parallel rendering path and Y-axis scaling. Best-effort: basic step rendering with auto-scaling first, user-settable limits as follow-up.

---

## Feature 2: Signal Row Height Resizing

**Why**: `SELECTED_VARIABLES_ROW_HEIGHT` hardcoded as `const u32 = 30` in `selected_variables_panel.rs:20`. Too small for analog traces. Used in name column (line 228), value column (line 445), footers (lines 307, 479), and panel height calculation (line 115). Canvas rendering computes height differently: `available_height / total_rows`.

**What**: Start with a global row height control (drag handle or slider). Per-variable heights as a follow-up if needed. Larger default for analog signals.

**How**:
- `frontend/src/selected_variables_panel.rs`: Replace `SELECTED_VARIABLES_ROW_HEIGHT` const with a `Mutable<u32>` in `AppConfig`. Reuse `DraggingSystem` from `frontend/src/dragging.rs` (already has `start_drag`, `process_drag_movement`, `end_drag` with min/max clamping and auto-persistence).
- `frontend/src/visualizer/canvas/rendering.rs`: Update row height calculation at line 300-301 to respect the configurable value.
- `shared/src/lib.rs`: Add `row_height: Option<u32>` to config.

**Difficulty**: Easy (global). `DraggingSystem` is ready to reuse. Per-variable heights would be Hard (data model + canvas sync + three-column layout coordination).

---

## Feature 3: Signal Grouping

**Why**: [GTKWave #368](https://github.com/gtkwave/gtkwave/issues/368), [#476](https://github.com/gtkwave/gtkwave/issues/476). Signal list doesn't scale past ~20 signals.

**What**: Named collapsible groups in selected variables panel. No drag & drop. No coloring.

**How**:
- `frontend/src/selected_variables.rs`: Currently stores `MutableVec<SelectedVariable>` (line 15). Add a separate `signal_groups: MutableVec<SignalGroup>` where `SignalGroup { name: String, member_ids: Vec<String>, collapsed: Mutable<bool> }`. Flat list remains for backward compatibility — ungrouped variables show outside any group.
- `frontend/src/selected_variables_panel.rs`: Add group header rows (clickable to collapse/expand) using the existing `expanded_scopes` pattern (line 19 of `selected_variables.rs`) as template for collapse state.
- `frontend/src/visualizer/canvas/rendering.rs`: Filter out collapsed variables from render list.
- `shared/src/lib.rs`: `SelectedVariable` (line 509) currently has `unique_id` and `formatter` only. Add `signal_groups: Vec<SignalGroupConfig>` to `WorkspaceSection` serialization. Old configs with no groups load fine (default to empty).

**Key challenge**: Keeping three columns (name, value, wave) synchronized when some rows are collapsed.

**Difficulty**: Medium.

---

## Feature 4: Named Markers / Time Bookmarks

**Why**: [vcdrom #27](https://github.com/wavedrom/vcdrom/issues/27) — navigation too manual. [GTKWave #308](https://github.com/gtkwave/gtkwave/issues/308) — extensibility demand. Jump-to-transition exists but no persistent bookmarks.

**What**: Place named markers at specific time points on the timeline. Render as labeled vertical lines on the waveform canvas (distinct color from cursor/zoom-center). Markers persist in `.novywave` workspace config. UX for creating/managing markers TBD.

**How**:
- `frontend/src/visualizer/timeline/timeline_actor.rs`: Add `markers: MutableVec<Marker>` where `Marker { time_ps: TimePs, name: String }`.
- `frontend/src/visualizer/canvas/rendering.rs`: Render marker lines using same pattern as `add_cursor_lines` (line 676-723) — vertical `Rectangle` at marker X position. Add text label using Fast2D `Text` (already used for timeline labels at line 831 and signal values at line 584).
- `shared/src/lib.rs`: Add `Marker` type and serialization in `TimelineConfig`.
- `frontend/src/visualizer/canvas/rendering.rs`: Add `markers: Vec<Marker>` field to `RenderingParameters` (line 65).

**Key challenge**: Text label overlap when markers are close together. Need a positioning strategy. Rendering itself is straightforward — direct copy of cursor line code.

**Difficulty**: Easy-Medium. Rendering: easy (2-3 hours). Marker management UX: medium, depends on desired complexity.

---

## Feature 5: File Picker Platform Roots

**Why**: macOS M1 developer blocked on first use. File picker showed `/home` instead of `/Users`. Files in `/tmp` invisible. First-minute blocker.

**What**: Platform-aware default roots. macOS: `/Users`, `~/`, `/Volumes`. Windows: drive letters. Linux: keep current. Clear "no waveform files found" empty-state when a directory has no supported files.

**How**:
- `frontend/src/file_picker.rs`: Currently hardcodes `"/"` as root (line 50-51) and tries `HOME`/`USERPROFILE` env vars (lines 55-57). The `build_tree_data("/", ...)` call at line 503 always starts from `/`. Replace with dynamic roots received from backend.
- Backend: Add `UpMsg::GetPlatformRoots` / `DownMsg::PlatformRoots(Vec<String>)`. Backend detects OS via `std::env::consts::OS` and returns appropriate root paths. Windows: enumerate drive letters. macOS: `/`, plus quick-access paths. Linux: `/`.
- Frontend tree initialization in `initialize_directories_and_request_contents`: handle multiple roots as top-level entries.
- `std::env::var("HOME")` on line 55 runs in WASM context — likely always fails in browser mode. Platform detection must happen on the backend side.

**Difficulty**: Easy. No architectural changes. Main effort is cross-platform testing.

---

## Implementation Priority

1. **File picker platform roots** — Easy, unblocks first-minute experience
2. **Signal row height resizing** — Easy (global), prerequisite for useful analog rendering
3. **Analog signal rendering** — Medium, biggest user-reported gap
4. **Named markers / time bookmarks** — Easy-Medium, well-patterned addition
5. **Signal grouping** — Medium, scales the tool for larger designs
