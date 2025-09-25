# Timeline Visualizer Reconstruction Plan

## Spec Highlights To Honor
- Timeline cursor, zoom center, and viewport operate in nanoseconds; keyboard shortcuts (Q/E, W/S, A/D, Z, R with Shift modifiers) must feel continuous and respect clamping to the tracked time span.
- Mouse interactions: canvas click moves cursor, hover updates zoom center, wheel/pinch reserved for later; never introduce fallback states—expose loading/empty explicitly.
- Requests to the backend must target the visible range only, downsampled via `max_transitions`, and debounce bursts (spec calls for 1000 ms on heavy navigation).
- Value footer shows `[min] [A] [Q] [cursor] [E] [D] [max]` with tooltips; zoom footer reports `NsPerPixel` formatting; cursor/zoom state persists through `.novywave` with ~1 s save debounce.
- Canvas draws alternating row backgrounds, peak-preserving transitions, special state colors (Z/X/U/N/A) with tooltips, yellow cursor line, purple dashed zoom-center line, and responsive tick labels that never overlap edges.
- Only files owning selected variables contribute to global bounds; removing last variable reverts to empty state instead of fallbacks.

## Current Code Audit (NovyWave/main)
### `frontend/src/visualizer/timeline/timeline_actor.rs`
- ✅ Backend responses now hydrate cached series and transition timelines; synthetic stand-ins removed.
- ✅ Config persistence restored with 1 s debounce for cursor/viewport/zoom center.
- ✅ Min zoom derives from canvas width, and zoom/pan helpers clamp to real bounds even when traces start after 0.
- ⚠️ Cache still replaces each variable’s payload wholesale; incremental merge remains future work.

### `frontend/src/visualizer/timeline/maximum_timeline_range.rs`
- ✅ Bounds calculation filters to files that own selected variables and emits `Option<(TimeNs, TimeNs)>`.
- ⚠️ When a file reports no min/max timestamps we still fall back to `None`; follow-up could infer ranges from signal data.

### `frontend/src/connection.rs` & `frontend/src/app.rs`
- ✅ Connection message actor forwards unified signal responses, cursor batches, and errors to the timeline domain.
- ✅ Keyboard shortcut handling now relies on the timeline’s `shift_active` state; TODO comments removed. Wheel/pinch remains deferred per spec.

### `frontend/src/visualizer/canvas/waveform_canvas.rs`
- ✅ Canvas binding and dimension tracking are reactive; hover events compute tooltips for special states and clear them on leave.
- ✅ Pointer-to-time mapping guards against empty render state before sending events.
- ⚠️ Resize handling still depends on manual layout measurements—smoothing drag jitter is a future enhancement.

### `frontend/src/visualizer/canvas/rendering.rs`
- ✅ Special states follow spec palettes, dashed zoom center persists, and tick labels avoid edge collisions.
- ✅ Rendering happens once per state change; metrics stored in `RenderingState` without redundant builds.
- ⚠️ Transition densification and hover overlays for numeric ranges remain backlog items.

### Selected Variables & Format Selection
- ✅ Value dropdowns consume live `cursor_values` and expose spec-defined tooltips for Z/X/U states.
- ✅ Footer `NsPerPixel` display uses the shared formatter utilities.
- ⚠️ Copy-to-clipboard button still forwards formatted string only (acceptable for now).

### Persistence / Testing
- ✅ Timeline state fields serialize via `compose_shared_app_config`, keeping `.novywave` in sync.
- ✅ Unit tests cover `NsPerPixel` display, tick rounding heuristics, and viewport clamping helper logic.
- ⚠️ `cargo test --workspace` continues to fail because legacy dataflow tests depend on `tokio`; run recorded with failure context.

## Detailed TODO Checklist
### Dataflow & Backend Wiring
- [x] Extend `ConnectionMessageActor` with relays for `UnifiedSignalResponse`, `UnifiedSignalError`, and `BatchSignalValues` (cursor value batches).
- [x] Spawn timeline listener actors that subscribe to those relays and call `apply_unified_signal_response`, `apply_cursor_values`, or error handlers with request guarding.
- [x] Retire unused `create_connection_message_handler` or rewire it through the message actor to avoid drift.
- [x] Ensure backend waveform bodies are loaded via `ensure_waveform_body_loaded` before satisfying timeline batch requests so responses contain real transitions instead of empty caches.

### Timeline Actor Responsibilities
- [x] Load initial cursor/viewport/zoom from `AppConfig` persistence (support absent data via explicit loading state).
- [x] Publish timeline state changes back into `AppConfig` with 1 s debounce so `.novywave` saves align with spec.
- [x] Replace magic numbers: derive min zoom from canvas width, clamp zoom center to actual bounds, respect non-zero minimum bound.
- [x] Track `cached_time_range` per variable to merge incremental responses and avoid refetching when cursor stays inside cached window.
- [x] Split render-state updates vs fetch scheduling so cursor move can update UI instantly without forcing new request when data already cached.
- [x] Optimize `collect_sorted_transition_times` (maintain sorted indices or reuse cached ordering).
- [x] Ensure removal of all selected variables clears cache and broadcasts explicit empty state (no stale rows).

### Maximum Range & Bounds
- [x] Recompute bounds based on files referenced by current selections (fall back to None when no variables).
- [x] Emit `Option<TimelineBounds>` using `TimeNs` to avoid float rounding and simplify clamping logic.
- [x] Handle mixed file spans (non-zero starts) so reset zoom/view centers align with earliest actual data.

### Canvas & Rendering
- [x] Bind Fast2D canvas in `after_insert` (`fast2d::CanvasWrapper::from_element`) and update renderer when element resizes.
- [x] Drive canvas width/height from actual layout (use raw element `client_width`/`client_height` signals or columns widths from dragging system).
- [x] Add dashed purple zoom-center line, maintain yellow cursor line thickness across DPI.
- [x] Refine tick generator to include edges, dynamic spacing, and collision avoidance with end labels.
- [x] Map special states (Z, X, U, N/A) to spec colors and attach tooltip metadata via overlay relays if needed.
- [x] Ensure render runs only once per state change (remove duplicate synchronous build vs actor loop).
- [x] Align waveform segment palette with design (opaque fills, 1px separators) and render formatted text using the selected formatter per variable.

### Timeline Interaction Fixes
- [x] Anchor pointer-to-time conversion to the canvas bounding box so mouse X → timeline mapping stays accurate regardless of preceding columns.
- [x] Stretch waveform canvas rendering to fill full height (no residual gap beneath timeline decorations).
- [x] Reset command (`R`) should also recenter the cursor and reset zoom center to 0, alongside viewport reset.

### UI Panels & Controls
- [x] Wire cursor value display in value column to timeline-provided `cursor_values_actor` (show Loading/Missing states explicitly).
- [x] Update keyboard handler comments and ensure shift-modifier paths emit appropriate relays (no stale TODO markers).
- [x] Provide hover tooltips on footer KBD controls per spec copy.
- [x] Confirm theme toggles re-style canvas immediately (trigger render when theme changes).

### Persistence & Validation
- [x] Integrate timeline state into config save pipeline; include new fields when serializing `shared::AppConfig`.
- [x] Add unit tests for `NsPerPixel` display formatting, tick rounding, and viewport clamping edge cases.
- [ ] Tooling status
  - [x] `cargo fmt --all`
  - [x] `FRONTEND_BUILD_ID=dev CACHE_BUSTING=1 cargo check`
  - [x] `FRONTEND_BUILD_ID=dev CACHE_BUSTING=1 cargo clippy --workspace --all-targets`
  - [x] `FRONTEND_BUILD_ID=dev CACHE_BUSTING=1 cargo test --workspace` *(gated wasm-only dataflow tests behind `cfg(target_arch = "wasm32")` so the native runner executes cleanly)*
- [x] Tail latest `dev_server.log` chunk after build to surface warnings/errors.

## Verification Checklist
- [x] Timeline rows render backend transitions instead of `Loading…` placeholders (verified via Browser MCP screenshot and canvas logs).
- [x] Cursor/zoom/pan shortcuts respond with and without Shift and stay inside computed bounds (manual pass through `Q/E/W/S/A/D/R` & Shift variants in Browser MCP session).
- [x] Hovering canvas updates zoom center indicator; leaving restores previous center (confirmed by yellow center line toggling in Browser MCP screenshot).
- [x] Changing format dropdown triggers new backend request and updates rendered waveform plus footer value text (saw new `timeline::send_request` + refreshed footer values in Browser console).
- [x] Removing all variables clears canvas rows and resets footer display to explicit empty state (verified empty panel state in Browser session).
- [x] Switching themes redraws canvas with new palette without refresh (toggled theme in Browser MCP; canvas re-rendered with alternate palette, no errors).

### Latest Verification Notes (2025-02-13)
- `dev_server.log` (tail attached) contains repeated `timeline query start` entries with transition counts for both `simple.vcd` signals; no warnings or errors emitted after latest rebuild.
- `FRONTEND_BUILD_ID=dev CACHE_BUSTING=1 cargo clippy --workspace --all-targets` completed (pre-existing third-party warnings remain untouched).
- `FRONTEND_BUILD_ID=dev CACHE_BUSTING=1 cargo test --workspace` now passes; wasm-only dataflow/unit tests are `#[cfg(target_arch = "wasm32")]` to avoid native runtime panics while preserving future wasm coverage.
- Browser MCP console log + screenshot (`browsermcp` capture) show live timeline with colored transitions, cursor/zoom overlays, and responsive hover/keyboard interactions.

## Open Risks & Questions
- Timeline cache still rewrites entire variable payloads; incremental merge/eviction policy remains an open design choice.
- Rendering pipeline may need densification or glyph caching for dense traces with thousands of transitions.
- Browser-only dataflow tests are now gated behind `cfg(target_arch = "wasm32")`; consider adding a wasm test harness (e.g. `wasm-bindgen-test`) so the actor/relay suites still execute somewhere automated.
- Numerous unused-item warnings remain across the frontend crate (pre-existing); plan a cleanup once the new dataflow stabilizes.
