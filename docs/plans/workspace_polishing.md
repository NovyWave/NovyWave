# Workspace Picker Polishing – Current Findings

## What Works
- `last_selected` and `recent_paths` are properly serialized into `.novywave_global` whenever a new workspace is chosen.
- The Open Workspace tree renders checkboxes for folders, so workspaces can be selected directly.
- The dialog clears prior selection when it opens (checkboxes are unchecked until the user picks a folder).

## Outstanding Issues

1. **Picker tree state still volatile (critical)**
   - Even after multiple reloads on 2025-10-22, `.novywave_global` only ever contains the auto-restored entries (`"/"`, `"/home/martinkavik"`); newly expanded folders never persist.
   - Recent instrumentation shows `🛰️ FRONTEND TRACE [expanded_state_snapshot] action=insert …` firing for each expansion, yet no corresponding `workspace_picker_snapshot` log or backend update follows.
   - We continue to see the backend receive `expanded_paths: []` immediately after dialog restore (`UpdateWorkspaceHistory … expanded_paths: []`), indicating an empty payload still wins the race somewhere in the pipeline.

2. **Workspace selection not cleared on dialog open (regression)**
   - Opening the picker shows the previously selected folder still checked despite `clear_selection_relay.send(())`.
   - `SelectedFilesSyncActors` log `config.rs:selected_files clear_selection_relay`, verifying the domain vector is emptied. If the checkbox persists, inspect the tree-view sync to ensure `external_selected_vec` is also cleared.

3. **Single-folder selection enforcement (fixed)**
   - ActorVec now trims selections to a single path (`config.rs:selected push single=<path>`). Manual testing shows checking a second folder replaces the first as expected.

4. **Scroll position persistence missing**
   - `.novywave_global` still lacks `picker_tree_state.scroll_top`. `workspace_history_scroll_actor` now reads from the scroll actor signal after restore completes; combine its behaviour with the snapshot log to ensure we send the latest value.

## Latest Debug Logs
- `🛰️ FRONTEND TRACE [workspace_picker_expand_request]` / `[workspace_picker_collapse_request]`: TreeView requests.
- `🛰️ FRONTEND TRACE [workspace_picker_expanded_applied]` / `[workspace_picker_collapsed_applied]`: actor commits.
- `🛰️ FRONTEND TRACE [workspace_picker_expanded_state]`: snapshot stream; should precede persistence when restoring=false.
- `🛰️ FRONTEND TRACE [workspace_picker_snapshot]`: published payload (expanded paths + scroll).
- `🛰️ FRONTEND TRACE [workspace_picker_selection]`: selection changes (includes expanded set + scroll).
- `🛰️ FRONTEND TRACE [workspace_picker_scroll]`: scroll actor updates during dialog use.
- `🛰️ FRONTEND TRACE [workspace_picker_restore]`: restore lifecycle.
- `frontend/src/config.rs:selected push single`, `… removed`, `… clear_selection_relay`: show when selection actors receive clear/select events.
- Backend persistence runs through `UpMsg::UpdateWorkspaceHistory`; watch `dev_server.log` for any errors after the snapshot log fires.
- Keep logs active until persistence works; remove afterwards.

## Latest Snapshot – 2025-10-22
- Added `expanded_state_snapshot` traces inside `FilePickerDomain::expanded_directories_actor` to confirm each expand/collapse produces the expected vector.
- `workspace_history_expanded_actor` now remembers the last non-empty vector and skips “hidden” empty updates, yet we still observe empty persistence events immediately after restore.
- Backend merge now ignores empty picker snapshots unless no prior data exists; empty writes have stopped, but new expansions still never reach disk.
- Picker scroll updates are now gated on the picker snapshot being non-empty; no more scroll-only writes of an empty tree.

## Current Hypotheses / Next Tests
1. Restore actor clears selection before the initial non-empty snapshot can publish, causing the first payload to be empty.
2. `workspace_history_expanded_actor` may subscribe after the first `expanded_state_snapshot`; confirm subscription ordering on dialog open.
3. TreeView sync might emit a collapse for every node when the dialog closes, overwriting expanded state with the minimal set.
4. Debounce loop inside `workspace_history_actor` could still flush an older `pending` value (empty) after a new non-empty snapshot arrives; inspect `pending` evolution.
5. Picker snapshot helper relies on `domain.expanded_directories_actor.state.lock_ref()` which may already be cleared by the restore reset.
6. Backend receives the correct non-empty payload but the clamp limit or merge logic drops paths outside the recent workspace list; need to trace `history.clamp_to_limit`.
7. Scroll actor gating may skip updates when expanded paths change; verify scroll + expanded sequences during manual interaction.
8. TreeView UI expansions might not mutate the `IndexSet` if the paths were already inserted during restore; dedupe prevents change detection.
9. Config save relay could be lagging—ensure `workspace_history_update_relay` fires after each new snapshot and reaches the backend.
10. The persisted paths we see (`"/"`, `"/home/martinkavik"`) might be inserted by the initialization actor, hiding the fact that user-driven expansions never make it in; compare logs against manual expand events.

## TODOs
- Capture a full dialog open → expand → close cycle with the new traces enabled; verify whether `workspace_picker_snapshot` ever logs during manual expansion.
- Inspect `workspace_history_actor`'s `pending` state by adding temporary logs around the debounce loop to confirm it switches from `[]` to the expanded list.
- Validate that the backend’s `UpdateWorkspaceHistory` handler now only prints non-empty picker snapshots; if any `[]` writes remain, gather the preceding frontend traces.

## Latest Debugging Notes
- Added relay instrumentation (`relay_subscribe`, `relay_send`) and confirmed the expanded-path relay often reports `before=1 after=0` once the picker closes—subscribers vanish, so follow-up snapshot calls see `[]`.
- Actor state remains accurate (`expanded_actor_state` logs the full path list) and the new `workspace_picker_tree_state_handle` mirrors that state into config; however, `workspace_history_expanded_actor` still restarts on every reopen and only ever logs `received paths=[]`.
- Snapshot helper now skips empty vectors; backend `UpdateWorkspaceHistory` continues to show `expanded_paths: []`, matching the empty inputs.
- Next session should: (1) keep `workspace_history_expanded_actor` subscribed once at app start (outside the dialog scope); (2) log the result of `config.workspace_history_state` immediately before sending `UpMsg::UpdateWorkspaceHistory`; (3) inspect backend serialization after a non-empty payload is confirmed.
- Dead ends so far: pushing snapshots from the relay before `state.set_neq` (still produced empties) and forcing a snapshot during selection changes (history overwrite persisted empty arrays).
