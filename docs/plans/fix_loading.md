# Loading/Startup Reliability Plan (server-first, last_selected)

## Start Here — Execution Checklist
1) Read this doc fully (assumptions, Boot Protocol, Persistence Flows).
2) Backend: ensure SelectWorkspace persists last_selected server‑side and that WorkspaceLoaded precedes ConfigLoaded. Files: backend/src/main.rs (load_config, select_workspace, handle_loaded_config).
3) Frontend: verify these are in place:
   - Bar updates strictly on WorkspaceLoaded; default label “Default ([path])”. frontend/src/app.rs.
   - Only LoadConfig retries at boot; SaveConfig/UpdateWorkspaceHistory are deferred until ConfigLoaded and ignore transient errors. frontend/src/platform/web.rs, frontend/src/config.rs.
   - Open Workspace dialog: clear selection on open; recents filtered by current+default; per‑item X removes. frontend/src/app.rs, frontend/src/config.rs.
4) Validate with the Test Plan using dev_server.log (no cargo): confirm WorkspaceLoaded and ConfigLoaded order, no early Save/History POSTs, and bar matches backend.
5) Remove temporary traces after stability.

This doc reflects observations from October 22, 2025 and folds in new constraints: the backend reads `.novywave_global` at start-of-process (before any frontend request). The server is therefore the source of truth for the active workspace at boot. The frontend must not “guess” and display a different workspace than the backend actually loads.

## Current Problems (observed)
- At fresh dev starts, the backend loads the default workspace even when the bar shows a different path; the bar was populated by a UI fallback (from last_selected/recent) before the server confirmed a workspace. Result: visual mismatch.
- Early POSTs (SaveConfig/UpdateWorkspaceHistory) fail with net::ERR_EMPTY_RESPONSE while the handler swaps. Flooding obscures the real first mile (`LoadConfig`).
- The Open Workspace dialog sometimes reopens with stale selection (checked path), disabling intent to select another.
- The Recent workspaces list sometimes includes the default or current workspace; filtering uses stale state.
- Persisting last_selected too late on the frontend means the next fresh start uses the default, not the last user selection — even though the bar shows otherwise.

## Non‑negotiable Principles
1) Server‑first truth: The only authoritative signal for the active workspace is `DownMsg::WorkspaceLoaded { root, default_root }`. The bar must bind to this, not to UI fallbacks.
2) Start‑of‑process load: The backend reads `.novywave_global` (last_selected + recents) before any frontend message. If the user wants a different workspace on next run, persist last_selected immediately when the user switches, so the next boot sees it.
3) No POST storms at boot: Only `LoadConfig` should attempt early; defer Save/History until after `ConfigLoaded`.
4) UI should never block user intent due to stale selection; dialog opens with no selection; “Open” enables on actual user choice.
5) Recents must exclude both the current workspace and the current default root (computed live).

## Boot Protocol (server‑first)
1) Frontend sends only `LoadConfig` (with a minimal, local retry) until `ConfigLoaded` is received.
2) Backend performs:
   - Read `.novywave_global` (global section); apply `last_selected` if valid.
   - Set workspace context; emit `WorkspaceLoaded { root, default_root }`.
   - Emit `ConfigLoaded(app_config)`.
3) Frontend updates UI strictly from `WorkspaceLoaded` (bar label) and unblocks from either `WorkspaceLoaded` or `ConfigLoaded`.

## Persistence Flows
- User switches workspace (SelectWorkspace):
  - Immediately persist last_selected on the frontend via `UpdateWorkspaceHistory` (best effort, quiet errors) AND request the switch.
  - Backend on SelectWorkspace should also update and persist the global history (server‑side guarantee). If not already, add this to the backend.
  - Next fresh start: server loads the newly persisted last_selected without waiting for the frontend.

## Dialog Invariants (Open Workspace)
- On open: clear any previous selection and target; “Open” is disabled until the user chooses.
- Recent list filtering uses live state:
  - Exclude `current_workspace` (from `WorkspaceLoaded`).
  - Exclude `default_root` (live default, not a stale snapshot).
- Per‑item “X” removes recent entries and persists quietly; UI updates immediately.

## Network and Error Policy
- Only `LoadConfig` retries locally (small backoff, finite attempts). Nothing else adds gates/queues.
- `SaveConfig` and `UpdateWorkspaceHistory` are prevented before `ConfigLoaded` and run as best‑effort after it. Errors are ignored in dev to avoid UI breakage.
- No `FrontendTrace` or other non‑critical posts during boot.

## UI Policy (bar and labels)
- The bar shows exactly the `WorkspaceLoaded.root` path. If root equals `default_root`, show `Default ([path])`.
- Remove any bar fallback that derives from last_selected/recent; it can lie if the server chose a different root.

## Concrete Tasks
Frontend
- [x] Remove bar fallback; update strictly on `WorkspaceLoaded`.
- [x] On user switch, call `record_workspace_selection()` which sends `UpdateWorkspaceHistory` immediately (quiet errors) so the next fresh start uses the chosen workspace; keep the SelectWorkspace flow the same.
- [x] Delay `SaveConfig` until `ConfigLoaded`; debounce normally afterward; ignore transient errors in dev.
- [x] Delay `UpdateWorkspaceHistory` (tree/scroll) until `ConfigLoaded`; ignore transient errors in dev.
- [x] On dialog open: clear selection and reset target; wire “Open” to actual selection.
- [x] Filter recents by current workspace and current default root (read live, not snapshotted).
- [x] Add small “X” button next to each recent to remove one entry.
- [x] Minimal `LoadConfig` retry loop in the platform (no global gates/queues).

Backend (recommended)
- [ ] On `SelectWorkspace`, persist last_selected (and recents) server‑side in addition to the current frontend best‑effort. This guarantees the next start reflects the user’s last choice even if the frontend didn’t manage to POST before shutdown.
- [ ] Ensure `WorkspaceLoaded` is always sent before `ConfigLoaded`, and that the values reflect the actual root/default used.

## State / Message Order (target)
```
Browser boot → POST LoadConfig (retry locally if needed)
Backend: read .novywave_global → set context
Backend: DownMsg WorkspaceLoaded {root, default_root}
Backend: DownMsg ConfigLoaded(app_config)
Frontend: bar ← WorkspaceLoaded.root (Default([path]) if root == default_root)
Frontend: unblock loading on WorkspaceLoaded or ConfigLoaded
Frontend: AFTER ConfigLoaded → SaveConfig / UpdateWorkspaceHistory (debounced, quiet errors)
```

## Test Plan (new)
1) Fresh start (no prior switch)
   - Expect: default workspace loads; bar shows `Default ([path])`; no Save/History errors before ConfigLoaded.
2) Switch to `workspace_b`, reload
   - Expect: `workspace_b` loads (server reads updated last_selected); bar shows `workspace_b` (not Default); no fallback.
3) Restart dev server while app open
   - Expect: a handful of LoadConfig attempts until success; after ConfigLoaded, normal Save/History resumes quietly.
4) Dialog re-open
   - Expect: no pre-checked rows; “Open” enables when a new folder is picked.
5) Recent list correctness
   - Expect: current workspace and default root absent; removing a recent updates immediately.

## Minimal Log Filters
```
rg -n "WorkspaceLoaded|ConfigLoaded|SelectWorkspace|UpdateWorkspaceHistory|SaveConfig" dev_server.log
```

## Open Items
- Backend persistence on `SelectWorkspace` (server-side guarantee) — recommended for true server-first behavior.
- If backend plugin discovery continues to panic, guard it behind catch_unwind and never let it block WorkspaceLoaded/ConfigLoaded.

## Rationale
The server reads `.novywave_global` before any frontend message. A UI that “fills in” the workspace bar from last_selected or recents without waiting for `WorkspaceLoaded` can mislead users. This plan makes the backend authoritative for the active workspace, defers non-essential traffic until after `ConfigLoaded`, and ensures last_selected is persisted as soon as the user switches so a fresh load reflects reality without relying on timing.
