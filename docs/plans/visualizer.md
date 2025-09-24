# Timeline Visualizer Reconstruction Plan

## Spec Highlights To Honor
- Timeline cursor, zoom center, and viewport operate in nanoseconds; keyboard shortcuts (Q/E, W/S, A/D, Z, R with Shift modifiers) must feel continuous and respect clamping to the tracked time span.
- Mouse interactions: canvas click moves cursor, hover updates zoom center, wheel/pinch reserved for later; never introduce fallback states—expose loading/empty explicitly.
- Requests to the backend must target the visible range only, downsampled via `max_transitions`, and debounce bursts (spec calls for 1000 ms on heavy navigation).
- Value footer shows `[min] [A] [Q] [cursor] [E] [D] [max]` with tooltips; zoom footer reports `NsPerPixel` formatting; cursor/zoom state persists through `.novywave` with ~1 s save debounce.
- Canvas draws alternating row backgrounds, peak-preserving transitions, special state colors (Z/X/U/N/A) with tooltips, yellow cursor line, purple dashed zoom-center line, and responsive tick labels that never overlap edges.
- Only files owning selected variables contribute to global bounds; removing last variable reverts to empty state instead of fallbacks.

## Current Code Audit (NovyWave/main + ongoing refactor)
### `frontend/src/visualizer/timeline/timeline_actor.rs`
- ✅ Replaces legacy mega-module with a focused `WaveformTimeline` actor: tracks cursor, viewport, zoom center, canvas size, selected variable cache, and request bookkeeping.
- ⚠️ Still lacks integration with `ConnectionMessageActor`; responses never reach the actor, so `apply_unified_signal_response` is unused.
- ⚠️ `initialize_from_config` is stubbed—timeline state always resets to zero, no persistence back to `.novywave`.
- ⚠️ Debounce timeout hard-coded to 120 ms (spec wants 1000 ms for navigation-heavy operations); no distinction between cursor nudge auto-repeat vs viewport jumps.
- ⚠️ Bounds listener consumes `MaximumTimelineRange`, but that actor ignores whether a file actually has selected variables.
- ⚠️ Cache only stores last response per variable; no incremental merging, eviction, or `cached_time_range` usage. `collect_sorted_transition_times` rebuilds vector on every jump (O(n) per call).
- ⚠️ Zoom/pan helpers don’t broadcast state back to config, and `reset_zoom_center` always snaps to 0 even if bounds start later (e.g., files beginning after 0).

### `frontend/src/visualizer/timeline/maximum_timeline_range.rs`
- ✅ Streams loaded files and computes min/max nanosecond span.
- ⚠️ Ignores selected-variable membership (spec wants bounds from files that actively drive the view). Needs to join with selections and cache empty state when none.
- ⚠️ Emits `f64` seconds; downstream constantly re-casts to `u64` nanoseconds. Prefer `TimeNs` to avoid precision drift.

### `frontend/src/connection.rs` & `frontend/src/app.rs`
- ✅ `ConnectionAdapter` cleans SendWrapper lifetime issues.
- ⚠️ `ConnectionMessageActor` routes config/file events only—no relays for unified signal responses or cursor-value batches.
- ⚠️ `create_connection_with_message_actor` bypasses `ConnectionAdapter::new`, so helper `create_connection_message_handler` is now unused dead code.
- ⚠️ Keyboard handler sends pan/zoom relays but still carries TODO comments about Shift variants (logic already in timeline actor; comments should be resolved). No mouse-wheel handling yet.

### `frontend/src/visualizer/canvas/waveform_canvas.rs`
- ✅ Wraps Fast2D renderer behind relays, tracks theme/dimensions/render-state updates.
- ⚠️ Never calls `WaveformRenderer::set_canvas`; without binding the DOM canvas the renderer never draws.
- ⚠️ Canvas size hard-coded to 800×600, ignoring layout size-fill requirements; no reactive sizing with panel width/height signals.
- ⚠️ Hover/click conversions assume viewport start is always <= cursor range; no guard for empty render_state.

### `frontend/src/visualizer/canvas/rendering.rs`
- ✅ Sketches rendering pipeline (row backgrounds, transitions, cursor/zoom lines, footer ticks).
- ⚠️ Doesn’t respect `zoom_center` styling (purple dashed line). Tick generator lacks collision avoidance with min/max labels.
- ⚠️ `WaveformRenderer::new` queues renders internally but also rebuilds objects synchronously; double rendering wastes work.
- ⚠️ No tooltip metadata for special states; color mapping needs alignment with design tokens.
- ⚠️ Ignores `total_transitions`/`actual_time_range` for densification decisions; no single-pixel highlighting for narrow pulses.

### Selected Variables & Format Selection
- ✅ Format dropdown updates relay to timeline; footers read live viewport/cursor data.
- ⚠️ Footer text still uses `NsPerPixel` naive formatting; needs spec-compliant units (μs vs us, etc.).
- ⚠️ Value rows show format dropdown only—no binding to cursor values yet (timeline never supplies them because backend response not processed).
- ⚠️ Tooltip strings built ad-hoc per row; should surface spec-defined hover messaging.

### Persistence / Testing
- ⚠️ `.novywave` timeline fields remain inert (never read/written).
- ⚠️ No automated coverage for time conversions, tick spacing, or request generation; spec calls for unit tests on pure helpers.
- ⚠️ `dev_server.log` not consulted post-change—must check before completion per workflow rules.

## Detailed TODO Checklist
### Dataflow & Backend Wiring
- [ ] Extend `ConnectionMessageActor` with relays for `UnifiedSignalResponse`, `UnifiedSignalError`, and `BatchSignalValues` (cursor value batches).
- [ ] Spawn timeline listener actors that subscribe to those relays and call `apply_unified_signal_response`, `apply_cursor_values`, or error handlers with request guarding.
- [ ] Retire unused `create_connection_message_handler` or rewire it through the message actor to avoid drift.

### Timeline Actor Responsibilities
- [ ] Load initial cursor/viewport/zoom from `AppConfig` persistence (support absent data via explicit loading state).
- [ ] Publish timeline state changes back into `AppConfig` with 1 s debounce so `.novywave` saves align with spec.
- [ ] Replace magic numbers: derive min zoom from canvas width, clamp zoom center to actual bounds, respect non-zero minimum bound.
- [ ] Track `cached_time_range` per variable to merge incremental responses and avoid refetching when cursor stays inside cached window.
- [ ] Split render-state updates vs fetch scheduling so cursor move can update UI instantly without forcing new request when data already cached.
- [ ] Optimize `collect_sorted_transition_times` (maintain sorted indices or reuse cached ordering).
- [ ] Ensure removal of all selected variables clears cache and broadcasts explicit empty state (no stale rows).

### Maximum Range & Bounds
- [ ] Recompute bounds based on files referenced by current selections (fall back to None when no variables).
- [ ] Emit `Option<TimelineBounds>` using `TimeNs` to avoid float rounding and simplify clamping logic.
- [ ] Handle mixed file spans (non-zero starts) so reset zoom/view centers align with earliest actual data.

### Canvas & Rendering
- [x] Bind Fast2D canvas in `after_insert` (`fast2d::CanvasWrapper::from_element`) and update renderer when element resizes.
- [x] Drive canvas width/height from actual layout (use raw element `client_width`/`client_height` signals or columns widths from dragging system).
- [ ] Add dashed purple zoom-center line, maintain yellow cursor line thickness across DPI.
- [ ] Refine tick generator to include edges, dynamic spacing, and collision avoidance with end labels.
- [ ] Map special states (Z, X, U, N/A) to spec colors and attach tooltip metadata via overlay relays if needed.
- [ ] Ensure render runs only once per state change (remove duplicate synchronous build vs actor loop).

### Timeline Interaction Fixes
- [x] Anchor pointer-to-time conversion to the canvas bounding box so mouse X → timeline mapping stays accurate regardless of preceding columns.
- [x] Stretch waveform canvas rendering to fill full height (no residual gap beneath timeline decorations).
- [x] Reset command (`R`) should also recenter the cursor and reset zoom center to 0, alongside viewport reset.

### UI Panels & Controls
- [ ] Wire cursor value display in value column to timeline-provided `cursor_values_actor` (show Loading/Missing states explicitly).
- [ ] Update keyboard handler comments and ensure shift-modifier paths emit appropriate relays (no stale TODO markers).
- [ ] Provide hover tooltips on footer KBD controls per spec copy.
- [ ] Confirm theme toggles re-style canvas immediately (trigger render when theme changes).

### Persistence & Validation
- [ ] Integrate timeline state into config save pipeline; include new fields when serializing `shared::AppConfig`.
- [ ] Add unit tests for `NsPerPixel` display formatting, tick rounding, and viewport clamping edge cases.
- [ ] Run `cargo fmt`, `cargo clippy --workspace --all-targets`, and `cargo test --workspace` (note pre-existing failures, capture details).
- [ ] Tail latest `dev_server.log` chunk after build to surface warnings/errors.

## Verification Checklist
- [ ] Cursor/zoom/pan shortcuts respond with and without Shift and stay inside computed bounds.
- [ ] Hovering canvas updates zoom center indicator; leaving restores previous center.
- [ ] Changing format dropdown triggers new backend request and updates rendered waveform plus footer value text.
- [ ] Removing all variables clears canvas rows and resets footer display to explicit empty state.
- [ ] Switching themes redraws canvas with new palette without refresh.

## Open Risks & Questions
- Backend `UnifiedSignalQuery` contract: confirm `cursor_values` uses same `unique_id` keying as UI; otherwise add adapter.
- Need guidance on caching strategy (per-variable window vs shared global) to avoid over-fetch; may require coordination with backend owners.
- Fast2D text rendering under heavy row counts could stutter—may need batching or glyph caching follow-up.
- `cargo test --workspace` currently fails (missing frontend harness). Decide whether to patch tests or document skip rationale for this sprint.
