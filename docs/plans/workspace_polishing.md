# Workspace Picker Polishing – Current Findings

## What Works
- `last_selected` and `recent_paths` are properly serialized into `.novywave_global` whenever a new workspace is chosen.
- The Open Workspace tree renders checkboxes for folders, so workspaces can be selected directly.
- The dialog clears prior selection when it opens (checkboxes are unchecked until the user picks a folder).

## Outstanding Issues

1. **Picker tree state never persisted (critical)**
   - `.novywave_global` still lacks any `[global.workspace_history.picker_tree_state]` block—only `last_selected` / `recent_paths` are present.
   - Restore playback now schedules a `🛰️ FRONTEND TRACE [workspace_picker_snapshot]` line (emitted in `dev_server.log`) after each restore or expansion; verify it shows non-empty paths when the dialog opens or folders are toggled.
   - If those logs are emitted but the backend still writes empty vectors (`🔧 BACKEND: UpdateWorkspaceHistory … expanded_paths: []`), capture the full log chunk and inspect the payload sent in `UpMsg::UpdateWorkspaceHistory`.

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

## Next Steps / Ideas
- Added full tracing for expanded-path persistence: relay send/subscribe counts, actor snapshots, and history task events.
- Current logs show relay occasionally broadcasting with zero subscribers, so the history task only receives empty vectors; backend still writes `expanded_paths: []`.
- Action items: capture subscriber lifecycle across dialog toggles, ensure broadcast happens after `state.set_neq`, and keep the history task subscribed across reopen cycles.
- **TODO 1 – Verify AppConfig ownership:** Instrument `AppConfig::update_workspace_picker_tree_state` / `_scroll` to trace the pointer identity of `workspace_history_state` just before sending through `workspace_history_update_relay`; confirm the same instance is observed by the relay actor.
- **TODO 2 – Inspect relay debouncing:** Add temporary logging inside the debounce loop in `AppConfig::new` (workspace_history_actor) to log every pending snapshot before persistence; stress-test expand/collapse spam and ensure the final payload still carries expanded paths.
- **TODO 3 – Restore playback snapshot:** During dialog open, log `expanded_directories_actor.state` immediately after `apply_workspace_picker_tree_state` and once `restoring_flag` flips false; ensure a non-empty vec triggers a persistence call.
- **TODO 4 – TreeView replay diff:** Temporarily remove the dedupe guard (`is_same`) or log the set before/after to verify that restored paths actually mutate the actor state; cross-check that the TreeView sync writes the full vector into `expanded_directories_actor`.
- **TODO 5 – Backend clamp/persist check:** In `handle_workspace_history_update`, log the incoming history and the serialized output right before `save_global_section`; ensure no later call overwrites `picker_tree_state` with an empty vec.

## Latest Debugging Notes
- Added relay instrumentation (`relay_subscribe`, `relay_send`) and confirmed the expanded-path relay often reports `before=1 after=0` once the picker closes—subscribers vanish, so follow-up snapshot calls see `[]`.
- Actor state remains accurate (`expanded_actor_state` logs the full path list) and the new `workspace_picker_tree_state_handle` mirrors that state into config; however, `workspace_history_expanded_actor` still restarts on every reopen and only ever logs `received paths=[]`.
- Snapshot helper now skips empty vectors; backend `UpdateWorkspaceHistory` continues to show `expanded_paths: []`, matching the empty inputs.
- Next session should: (1) keep `workspace_history_expanded_actor` subscribed once at app start (outside the dialog scope); (2) log the result of `config.workspace_history_state` immediately before sending `UpMsg::UpdateWorkspaceHistory`; (3) inspect backend serialization after a non-empty payload is confirmed.
- Dead ends so far: pushing snapshots from the relay before `state.set_neq` (still produced empties) and forcing a snapshot during selection changes (history overwrite persisted empty arrays).
