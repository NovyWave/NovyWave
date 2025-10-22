# Loading/Startup Reliability Plan (dev server restarts, last_selected)

## Current Status
- After a MoonZoon dev‑server restart, the frontend can get stuck in a "Loading…" state and spam POST `/_api/up_msg_handler` with `net::ERR_EMPTY_RESPONSE`.
- Browser Network tab shows many failed POSTs; static assets (fonts, css) load fine; the handler sometimes returns 200 briefly, then drops again while the server swaps.
- Only `LoadConfig` needs to succeed to unblock the UI, but it’s the first message and collides with restart windows.
- `last_selected` in `.novywave_global` should be applied on startup; we added backend support, but it only helps once `LoadConfig` completes.

## Constraints / Guidelines
- Don’t run `cargo` locally. Use the maintainer dev server; rely on `dev_server.log` for truth. Share minimal relevant excerpts.
- Avoid flooding the server during rebuilds. Use explicit readiness and a small queue instead of repeated posts.
- Keep logs temporary and low‑noise; remove after green runs.

## Repro Steps (minimal)
1. Start the app in the browser; observe normal load.
2. Restart the MoonZoon dev server (the one backing `/_api`).
3. Watch:
   - Network tab: `HEAD/GET /_api/up_msg_handler` readiness probes vs `POST /_api/up_msg_handler`.
   - Console: any `load_config_retry`, `workspace_switch_retry` traces.
   - `dev_server.log`: look for `WorkspaceLoaded` / `ConfigLoaded` lines; if absent, server never processed `LoadConfig`.

## Hypotheses
1. Handler swap window: the HTTP server accepts static GETs earlier than it accepts POST to `/_api/up_msg_handler`; `HEAD` may succeed while the POST path is still closed, causing ERR_EMPTY_RESPONSE.
2. Multi‑flight: concurrent `LoadConfig` senders (initial boot + internal retries) create a thundering herd; even if the server becomes ready, overlapping requests fail more often.
3. UI stuck: `workspace_loading` flips to true and never flips false when `LoadConfig` fails early; no `WorkspaceLoaded`/`ConfigLoaded` arrive to clear it.
4. Trace/noise: repeated `FrontendTrace` (or other non‑critical posts) during restart expand the outage window and mask real failures.

## Fix Strategy (phased)

### Phase A — Make startup robust and quiet
- Platform gate (frontend/src/platform/web.rs):
  - Use an API‑path probe that matches the transport used by `LoadConfig`.
  - Switch to a tiny `GET /_api/up_msg_handler` (or `POST` with empty body) readiness probe; treat any non‑network error status (e.g. 405/400) as readiness.
  - Add a short “circuit breaker” after `net::ERR_EMPTY_RESPONSE`: don’t re‑probe for 400–600 ms; then try again with exponential backoff (cap ~5–6 s).
  - Keep a single‑flight guard for `LoadConfig` so only one in‑flight attempt exists.
  - Drop non‑critical `UpMsg` while server is not ready (we already skip) and suppress repeated console logs.

- App boot (frontend/src/app.rs::new):
  - Retry `LoadConfig` only via the platform gate (remove extra manual loops once gate is proven) to avoid double retry stacks.
  - Do not set `workspace_loading` to true at boot; only flip it after `WorkspaceLoaded` or when a user explicitly switches workspaces.
  - Add a toast on prolonged server unavailability (optional; remove later).

### Phase B — Ensure last_selected is respected
- Backend (backend/src/main.rs::load_config): already applies `last_selected` by canonicalizing the path and setting the workspace root before sending messages.
- Fallbacks:
  - If `last_selected` is invalid or missing, use `INITIAL_CWD` and log a concise `ConfigError` once.
  - Ensure message order remains: `WorkspaceLoaded` (with `root` and `default_root`) then `ConfigLoaded`.

### Phase C — Queue + flush strategy
- While `server_ready=false`, queue critical UpMsgs (`SelectWorkspace`, `SaveConfig`, `UpdateWorkspaceHistory`) into a small ring (size ~8).
- When readiness flips true (successful probe), flush the queue in order with 150–300 ms spacing (avoid stampede) and clear.
- Always send `LoadConfig` first if pending, then others.

## Code Pointers
- Frontend:
  - `frontend/src/platform/web.rs` — server readiness probe, single‑flight guard, message gating and logging.
  - `frontend/src/app.rs` — initial `LoadConfig` request; `workspace_loading` handling; `WorkspaceLoaded`/`ConfigLoaded` actors.
  - `frontend/src/app.rs::start_workspace_switch` — user‑initiated workspace changes; ensure rollback on retry exhaustion.
- Backend:
  - `backend/src/main.rs::load_config`, `handle_loaded_config`, `select_workspace`, `workspace_context`.

## Implementation Tasks (checklist)
- [ ] Replace HEAD probe with GET (or empty POST) to `/_api/up_msg_handler`; treat any non‑network response as ready.
- [ ] Add short circuit‑breaker after `net::ERR_EMPTY_RESPONSE`; exponential backoff (250ms → 500ms → 1s → max 1.5–2s) with cap ~6s.
- [ ] Keep single‑flight `LoadConfig`; ensure we never log "Sending message…" more than once per attempt.
- [ ] Remove duplicate retry loop in `NovyWaveApp::new()`; rely on platform gate only.
- [ ] Don’t set `workspace_loading` at startup; only on user switch. Ensure it is cleared on `WorkspaceLoaded` OR `ConfigLoaded` OR `ConfigError`.
- [ ] Confirm backend `last_selected` path resolution works with relative and absolute paths and invalid entries.
- [ ] Add small queue for critical UpMsgs during outage; flush after readiness.
- [ ] Clean temporary traces and unused-variable warnings.

## Test Plan (manual)
1. Restart server while app is open.
   - Expect: a handful of GET probes to `/_api/up_msg_handler`, no POST flood; once ready, exactly one `LoadConfig` POST; UI unblocks.
2. Cold reload the app.
   - Expect: app opens the `last_selected` workspace automatically (`WorkspaceLoaded` then `ConfigLoaded`); no stuck loading.
3. Switch workspaces during rebuild.
   - Expect: switch either completes after server is back or rolls back path with a toast; no indefinite loading and no POST storm.
4. Verify `.novywave_global` remains strictly global (no per‑workspace tree_state entries) and picker scroll persists.

## Minimal Log Filters
```
rg -n "load_config_retry|workspace_switch_retry|WorkspaceLoaded|ConfigLoaded|ConfigError|UpdateWorkspaceHistory" dev_server.log
```

## Notes
- If MoonZoon rejects `GET /_api/up_msg_handler`, probe `/_api/` or `/_api/public/content.css` and then do a single guarded `LoadConfig` POST.
- Keep the queue/gate small to reduce complexity; the key is to send one `LoadConfig` at the right time.

