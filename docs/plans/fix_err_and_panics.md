# ERR_EMPTY_RESPONSE on startup and backend panics — findings and plan

This note captures what causes the red "ERR_EMPTY_RESPONSE" entries and the
visible backend panics during dev startup/switches, and what we changed so a
future session can continue from a stable baseline.

## Observations
- Browser console showed multiple `POST /_api/up_msg_handler net::ERR_EMPTY_RESPONSE`
  on fresh page load and when switching workspaces.
- dev_server.log periodically contained plugin panic lines (e.g. plugin manager
  mutex poisoned; catch_unwind continued; repeated panic prints).
- In some runs the UI label said "No workspace selected" or stayed on
  "Loading workspace…" even though the default workspace was already active.
- Files & Scopes sometimes carried files from the previous workspace when the
  target workspace had no opened files.

## Root causes (dev mode)
- During dev server rebuild/hot‑swap the up_msg handler is briefly unavailable;
  any in‑flight POST naturally returns `ERR_EMPTY_RESPONSE`.
- Non‑critical posts were being sent during that window:
  - `FrontendTrace` (debug logging)
  - Debounced `SaveConfig`
  - `UpdateWorkspaceHistory`
  - `BrowseDirectory` requests emitted by tree initializers
- Focus‑based re‑sync and readiness POST probes amplified traffic in the same
  window and produced extra red entries.
- Backend plugin discovery/reload can panic in dev; printing multiple panic
  lines looks alarming but is already wrapped in `catch_unwind`, so it doesn’t
  block `WorkspaceLoaded/ConfigLoaded`.

## What was changed (current baseline)
- Startup sends exactly one `LoadConfig` (with a single small retry if it fails)
  and no readiness POST probes.
- Suppress non‑critical posts until the first DownMsg arrives
  (i.e., until the server has responded once): no `FrontendTrace`, `SaveConfig`,
  `UpdateWorkspaceHistory`, or `BrowseDirectory` before first DownMsg.
- On workspace switch, pause the config saver until the next `ConfigLoaded` to
  avoid SaveConfig during handler swap.
- Remove focus‑based re‑sync and debug relay traces that produced noisy logs.
- Server‑first workspace label: shows "Default ([path])" or `[path]` only from
  `WorkspaceLoaded`. Before that, shows "Loading workspace…" (never
  "No workspace selected").
- Always clear Files & Scopes and Selected Variables on `WorkspaceLoaded` and
  clear on `ConfigLoaded` if `opened_files` is empty so state never carries across
  workspaces.

## Recommended invariant (dev and prod)
- Frontend must send only `LoadConfig` until `ConfigLoaded` is received.
- On user switch, send `SelectWorkspace` and do not send any other posts until
  the next `ConfigLoaded`.
- Treat plugin discovery/reload failures as non‑blocking; never let them delay
  `WorkspaceLoaded/ConfigLoaded`.

## Quick triage checklist
1) Fresh reload
   - Expect: at most one `LoadConfig` POST (one fallback is acceptable).
   - No other POSTs until the first DownMsg is seen.
   - Label: "Loading workspace…" → switches to `Default ([path])` or `[path]`.
2) Switch workspace (especially to an empty one)
   - Expect: one `SelectWorkspace` POST; no `SaveConfig`/`UpdateWorkspaceHistory`
     until `ConfigLoaded`.
   - Files & Scopes cleared on `WorkspaceLoaded`; repopulated only if new config
     has `opened_files`.
3) Plugin panics
   - Acceptable: panic lines in dev_server.log; must still see
     `WorkspaceLoaded` → `ConfigLoaded` sequence immediately after.

## Minimal log filters
```
rg -n "LoadConfig received|WorkspaceLoaded|ConfigLoaded|SelectWorkspace|SaveConfig|UpdateWorkspaceHistory" dev_server.log
```
Interpretation:
- During boot: only `LoadConfig received` → `WorkspaceLoaded` → `ConfigLoaded`.
- During switch: `SelectWorkspace requested root=…` → `WorkspaceLoaded` →
  `ConfigLoaded`.
- `SaveConfig`/`UpdateWorkspaceHistory` must appear only after the corresponding
  `ConfigLoaded`.

## Next steps / housekeeping
- Delete the unused `wait_until_server_ready` stub and any dead readiness code.
- Consider throttling or silencing repeated plugin panic prints in dev to keep
  logs readable.
- Keep tree initializers from auto‑`BrowseDirectory` before first DownMsg if the
  directory cache is already warm.

## Why `ERR_EMPTY_RESPONSE` can still happen
- In dev, a single red POST right around a hot‑swap is normal. The important bit
  is: those should be rare (≤1) and never from saver/history/trace traffic in
  the sensitive window. If more appear, re‑check that only critical messages are
  allowed pre‑DownMsg and that the saver stays paused during workspace changes.

