# Fixing Variables Panel

Goal: Variables panel must reflect scope selection live. When a scope is checked, list variables from that scope. When unchecked (no scope), show the empty-state hint.

Notes (current state observed)
- Reproduced: clicking scope `s` updates TreeView selection (`MutableVec`) and triggers config save, but Variables panel remains at 0 and doesn’t refresh.
- SelectedVariables.selected_scope did not react; Browser console lacked any follow‑up “variables context” logs after clicking scope.
- Root cause: scope-selection bridge was subscribing to `VecDiff` and internally replaying diffs; this path silently desynced and SelectedVariables never saw the final selection.

Changes implemented
- Simplified bridge in `frontend/src/app.rs`: subscribe to the TreeView `MutableVec` using `signal_vec_cloned().to_signal_cloned().to_stream()` and forward the whole selection snapshot on every change via `propagate_scope_selection(...)`. This eliminates diff bookkeeping and ensures consistent propagation, including clearing on empty.
- Removed noisy frontend logs (app/config/treeview/tracked_files/variables UI) and trimmed backend printlns to make live terminal output and the browser console readable for testing.

Quick TODOs (compact)
- Verify live behavior in Browser MCP: check `s` → list appears; uncheck → list clears; re-check → list reappears.
- Watch the live `makers start` output and confirm `Frontend built` with no `error[E...]` lines.
- Keep logging lean going forward; avoid re‑adding `zoon::println!` except when strictly necessary and temporary.

Post‑fix validation
- Files: `frontend/src/app.rs:scope_selection_sync_actor` updated to snapshot stream; `novyui/.../treeview.rs` and others had debug prints removed.
- If any lingering mismatch remains, instrument SelectedVariables.selected_scope only (temporary) and remove before compaction.

## Context Compaction Protocol

Aim: keep our iteration context tiny and high‑signal so we don’t blow buffers or waste attention.

- Log access
  - Live build output: use `makers start 2>&1 | rg -n "Frontend built|Finished|error\\[E|error:|panic"` if you need a filtered rerun. No full scans.
  - Browser logs: sample last ~50 entries and filter for `🧭`, `VARIABLES_CONTEXT`, `TREEVIEW_SYNC`. Ignore everything else.
- Instrumentation gates
  - Keep only compact `🧭` traces in: app bridge and SelectedVariables scope actor. Remove them after confirmation.
  - Never add printlns in hot loops unless behind a temporary gate and targeted.
- File reads
  - Use `rg -n` with specific symbols/IDs; read max 200 lines per file chunk.
  - Avoid full‑file dumps; prefer function‑level slices.
- Assertions (UI over logs)
  - Prefer UI counters/empty‑state as truth: Variables count 0→2→0 for `s` toggle.
  - Logs are support signals, not primary proof.
- Retained shorthands (for future notes)
- `sv_scope` = SelectedVariables.selected_scope
- `tree_scope` = first item in AppConfig.files_selected_scope (cleaned)
- Effective scope = `sv_scope.or(tree_scope)`
- Interesting fact (current): with two files loaded (`simple.vcd` + `wave_27.fst`), toggling `s` four times leaves Variables panel locked on the FST scope (count 5371). TreeView emits selected_vec entries for both scopes but our bridge picks the wrong entry after the toggle. Need to prioritise the most recently toggled scope and ensure clearing works when the vector empties.

## Compact TODOs (survive compaction)
- [ ] Click `s` twice; expect Variables 0→2→0, no refresh.
- [ ] Remove remaining `🧭` traces after confirmation.
- [ ] If regression: enable a single `🧭` in SelectedVariables scope actor; capture 1 sample; remove.
