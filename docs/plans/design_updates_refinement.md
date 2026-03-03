# Design Updates Refinement — Feature Spec

Five features traced to user research interviews and issue tracker findings. Each addresses friction that real users reported.

---

## Feature 1: Analog Signal Rendering

**Why**: Physicist: "height extensions feel wonky, values normalized to current window, cannot set limits." Backend parses Real values via `wellen` (floats with 6 decimal places). Frontend renders them as digital text rectangles — no curves, no Y-axis.

**What**: Polyline/step-function rendering for Real signals in Fast2D canvas. Y-axis scale in the value column area. User-settable min/max limits (not just auto-normalize to visible window). Smooth interaction with zoom/pan.

**How**:
- `frontend/src/visualizer/canvas/rendering.rs`: New rendering path — detect Real encoding in signal metadata, draw line segments between (time, normalized_value) points instead of rectangles. Use Fast2D line drawing API.
- `frontend/src/selected_variables_panel.rs`: Add Y-axis min/max controls (input fields or drag handles) per variable.
- `shared/src/lib.rs`: Add `AnalogLimits { min: f64, max: f64, auto: bool }` to per-variable config.
- `.novywave` config: Persist limits per signal.

**Difficulty**: Medium-High. New rendering pipeline path + UI controls + config persistence. Best-effort: basic step rendering first, Y-axis controls as follow-up.

---

## Feature 2: Signal Row Height Resizing

**Why**: `SELECTED_VARIABLES_ROW_HEIGHT` hardcoded at 30px in `selected_variables_panel.rs`. Too small for analog traces. Physicist needs vertical space.

**What**: Drag handle on row dividers to resize individual signal heights. Per-signal height stored in workspace config. Larger default for analog signals (e.g. 80px vs 30px for digital).

**How**:
- `frontend/src/selected_variables_panel.rs`: Replace `SELECTED_VARIABLES_ROW_HEIGHT` constant with per-variable `Mutable<f64>`. Add drag interaction on row bottom edges (reuse existing `DraggingSystem` pattern from panel dividers).
- `frontend/src/visualizer/canvas/rendering.rs`: Accept variable row heights in rendering params. Adjust Y-coordinate calculations per row.
- `shared/src/lib.rs`: Add `row_height: Option<f64>` to per-variable config.

**Difficulty**: Medium. Drag interaction pattern already exists in the codebase (panel dividers). Main challenge: coordinating height changes between the 3-column selected variables panel and the waveform canvas rendering.

---

## Feature 3: Signal Grouping and Coloring

**Why**: [GTKWave #368](https://github.com/gtkwave/gtkwave/issues/368), [#476](https://github.com/gtkwave/gtkwave/issues/476). Signal onboarding unintuitive, flat list doesn't scale past ~20 signals.

**What**: Named collapsible groups in selected variables panel. Drag signals between groups. Drag to reorder within groups. Color picker per signal or per group. Color reflected in waveform rendering (segment fill color).

**How**:
- `frontend/src/app.rs` / `selected_variables_panel.rs`: Extend selected variables data model from flat `Vec<SelectedVariable>` to `Vec<SignalGroup>` where each group contains a name, color, collapsed state, and `Vec<SelectedVariable>`.
- Group UI: Use NovyUI accordion or treeview pattern for collapse/expand. Add group header row with name, color swatch, collapse toggle.
- `frontend/src/visualizer/canvas/rendering.rs`: Accept per-variable color override. Use it instead of `value_bus_color` / `value_high_color` etc.
- `.novywave` config: Serialize group hierarchy, colors, ordering.

**Difficulty**: Medium-High. Data model restructuring + new UI interactions + rendering color integration. Group collapse/expand is the core; coloring is incremental on top.

---

## Feature 4: Named Markers / Time Bookmarks

**Why**: [vcdrom #27](https://github.com/wavedrom/vcdrom/issues/27) — navigation too manual. [GTKWave #308](https://github.com/gtkwave/gtkwave/issues/308) — extensibility demand. Jump-to-transition exists but no persistent bookmarks.

**What**: Place named markers at specific time points on the timeline. Render as labeled vertical lines on the waveform canvas (distinct color from cursor/zoom-center). Keyboard shortcuts: M to add marker at cursor, Shift+N / Shift+P to jump to next/prev marker. Markers persist in `.novywave` workspace config.

**How**:
- `frontend/src/visualizer/timeline/timeline_actor.rs`: Add `markers: MutableVec<Marker>` where `Marker { time_ns: u64, name: String, color: Option<Color> }`. Add `add_marker_at_cursor()`, `jump_to_next_marker()`, `jump_to_prev_marker()` methods.
- `frontend/src/visualizer/canvas/rendering.rs`: Render marker lines using same pattern as `cursor_line` (vertical line at x-position + text label at top). Use distinct color (e.g. cyan or user-chosen).
- `frontend/src/app.rs`: Add keyboard handlers for M, Shift+N, Shift+P.
- `shared/src/lib.rs`: Add `Marker` type and serialization for `.novywave` config.

**Difficulty**: Medium. Well-patterned — cursor line rendering in `rendering.rs` provides exact template. Text label rendering on canvas is the main new challenge (Fast2D text support needed).

---

## Feature 5: File Picker Platform Roots

**Why**: Interview C — macOS M1 user blocked on first use. File picker showed `/home` instead of `/Users`. Files in `/tmp` invisible. First-minute blocker that made every other feature invisible.

**What**: Platform-aware default roots. macOS: `/Users`, `~/`, `/Volumes`, `~/Desktop`, `~/Downloads`. Windows: drive letters + `%USERPROFILE%` shortcuts. Linux: keep current `/home`, `/tmp`, `/usr`. Clear "no waveform files found" empty-state when a directory has no `.vcd`/`.fst`/`.ghw` files. Verify `/tmp` on macOS shows files correctly.

**How**:
- `frontend/src/file_picker.rs`: Add platform-conditional root paths. Use `#[cfg(target_os = "macos")]` for compile-time detection in Tauri mode, or runtime OS detection via Tauri API for the browser mode (backend can report OS). Add "Home" quick-navigation shortcut button in the file picker header.
- Test matrix: macOS M1, macOS Intel, Windows 10/11, Linux (Ubuntu, Arch), WSL2.

**Difficulty**: Low. Platform detection + path configuration. No architectural changes. Main effort is cross-platform testing.

---

## Implementation Priority

1. **File picker platform roots** — Low difficulty, unblocks first-minute experience
2. **Signal row height resizing** — Medium difficulty, prerequisite for useful analog rendering
3. **Analog signal rendering** — Medium-High, biggest user-reported gap
4. **Named markers / time bookmarks** — Medium, well-patterned addition
5. **Signal grouping and coloring** — Medium-High, scales the tool for larger designs
