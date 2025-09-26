# Smooth Zoom & Pan Roadmap

## 1. Current Symptoms

- Holding `S` or continuous panning still leaves visible gaps before transitions catch up, even after the latest request throttling tweaks.
- Zooming from *fit-all* down to sub-nanosecond scales causes the center bar and waveform to “lurch” instead of animating smoothly.
- Large cursor updates trigger full canvas re-renders, which compete with new backend fetches and queue up extra work.
- Cursor-value dropdowns flash "Loading…" when data is cleared between requests (partially mitigated, but root cause remains a cache miss).

## 2. Constraints & Acceptance

- **Acceptance = manual feel:** success means panning/zooming passes a manual smoke test on `simple.vcd` + `wave_27.fst` without visible stalls or flicker.
- **Keep it simple:** prefer incremental fixes that reuse existing actors/relays; avoid new systems that only help edge cases.
- **Architecture guardrails:** any new state lives inside actors (`frontend/src/dataflow` types). Relay names must describe observed events (e.g. `timeline_cache_window_updated_relay`).

## 3. Existing Pipeline (Quick Audit)

| Stage | What happens today | Notes |
| --- | --- | --- |
| Frontend timeline actor | Requests every viewport change with `max_transitions = 4 * width_px`, stores only the latest response, clears previous data | No reuse of overlapping windows; request flood risk when inputs spike |
| Backend | Filters transitions to the requested window, then runs bucketed downsampling (`backend/src/main.rs:498-577`) | Keeps edges per bucket but recomputes on every request; no reuse across zoom levels |
| Canvas renderer | Rebuilds full object list on each response; draws rectangles per pixel (`frontend/src/visualizer/canvas/rendering.rs:420-580`) | No incremental updates; single-threaded |
| Input loop | Key-repeat loop (~55 ms) + throttle (~60 ms) | Stalls when backend + render pipeline exceed ~120 ms |

## 4. Reference Techniques

- **GTKWave**: gesture filters gate redraws until the previous frame is ready (`wavewindow.c`).
- **PulseView (sigrok)**: tracks `samples_per_pixel` and reuses decoded edge segments (`pv/views/trace/logicsignal.cpp`).
- **deck.gl Performance Guide**: prioritise frame budgets and progressive refinement (`docs/developer-guide/performance.md`).

These reinforce three lightweight principles for NovyWave:
1. Cache overlapping windows instead of refetching immediately.
2. Respect frame budgets—skip launches when a draw/request is still in flight.
3. Draw something immediately (cached coarse data) while refined data loads.

## 5. Architecture Touchpoints

- `WaveformTimeline` already owns request throttling, cursor state, and `series_map` (latest transitions) in `frontend/src/visualizer/timeline/timeline_actor.rs`.
- `TimelineRenderState` combines viewport + series for canvas consumption; renderer rebuild cost comes from regenerating `Vec<Object2d>` every response.
- Backend responses arrive via `ConnectionAdapter` and hydrate `series_map`; there is no shared cache beyond the latest window.

## 6. Implementation Plan

### Phase 0 – Instrumentation & Safety Nets (keep existing behaviour observable)
- Add lightweight timing to `WaveformTimeline::send_request` and the canvas render entry point; log when either exceeds 80 ms.
- Surface a `timeline_debug_overlay_enabled` atom tied to a panel toggle so we can manually inspect last latency + cache hit status (no auto overlay in release).
- Harden known weak spots (RefCell double borrow around `timeline_actor.rs:476`) while touching the surrounding code.

### Phase 1 – Frontend Window Reuse (core smoothing work)
- Introduce `TimelineWindowCache` stored inside a new `window_cache_actor: Actor<TimelineWindowCache>` on `WaveformTimeline`.
  - Each entry keyed by `(variable_id, lod_bucket, range_ns)` and stores `Arc<Vec<SignalTransition>>`.
  - Keep at most the current window plus one neighbour per variable (FIFO eviction) to stay memory-light.
- Add relays to coordinate cache lifecycle:
  - `timeline_cache_window_requested_relay`: emitted before sending an up_msg.
  - `timeline_cache_window_hydrated_relay`: emitted when a backend response lands.
- Request flow update:
  1. On viewport change, derive `lod_bucket` from `TimePerPixel` (round to the nearest power-of-two bucket).
  2. Check the cache actor; if ≥80 % of the requested window is already cached, reuse those transitions immediately in `series_map` and render.
  3. Only send `UnifiedSignalQuery` for the missing segments; tag the request with `request_id` as today.
  4. Merge new segments back through `timeline_cache_window_hydrated_relay` to keep cache + `series_map` aligned.
- Cursor dropdowns read from the same cache so we do not wipe values while a refill is in flight.

### Phase 2 – Render/Interaction Polishing (only if Phase 1 still feels rough)
- Track a `render_generation` counter inside `WaveformTimeline`; renderer ignores responses that are older than the most recent generation.
- Split rendering into preview/detail passes without major refactor:
  - Preview = rectangles derived from cached window (executed synchronously).
  - Detail = label/text pass kicked by a `render_detail_ready_relay` once the full response finishes and the frame budget allows (~120 ms idle window).
- Optional: add a `ZoomAnimation` helper that animates towards the target viewport over 100 ms using `requestAnimationFrame`, cancelling early if the user releases the key.

## 7. Dataflow Updates (Actor+Relay Compliance)

- Extend `WaveformTimeline` struct with:
  - `window_cache_actor: Actor<TimelineWindowCache>`
  - `timeline_cache_window_requested_relay: Relay<CacheRequest>`
  - `timeline_cache_window_hydrated_relay: Relay<CacheHydration>`
- `TimelineWindowCache` loop listens to request/hydrate relays, performs eviction, and exposes a signal queried from `WaveformTimeline::send_request`.
- Renderer subscribes to a new `render_generation_signal` so it can drop stale draw jobs instead of rebuilding unnecessarily.

## 8. Validation & Manual Acceptance

- Use the maintainer dev server (`dev_server.log`) to confirm no warnings/errors after code changes.
- Manual smoke tests (desktop + web):
  - Hold `S` then `W` for 3 s on `simple.vcd` and verify no blank frames.
  - Drag timeline rapidly on `wave_27.fst` while watching cursor dropdowns—values should not flicker to "Loading…" unless we leave cached regions.
- Collect latency numbers from the debug overlay; log results in task notes (no automated threshold yet).

## 9. Deferred Explorations

- Backend LOD pyramid remains interesting but is not required for the current smoothness pass. Revisit only if manual tests still show stalls after Phase 1.
- Evaluate WebGL or alternative renderers later; Canvas2D remains the target for this iteration.
- TODO: Triage the existing `frontend` warnings (unused timeline constants, unused UI helpers) once the smoothing changes land so `dev_server.log` stays clean.

## 10. Open Questions

- How aggressive can eviction be before users notice cache churn on slow machines?
- Should the cache bucket rounding follow fixed pixel steps (e.g. 8 px) or a simpler nearest power-of-two scheme?
- Do we need extra telemetry (e.g. histogram of request spans) to justify LOD work if Phase 1 does not fully solve the gaps?

---

Prepared by: *Codex agent*
