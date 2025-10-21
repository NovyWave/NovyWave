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
- Trace the flow when the picker opens: confirm the snapshot log shows the restored expansion list, then check `.novywave_global` after the backend write to ensure it matches.
- If the backend still drops `picker_tree_state`, capture `UpMsg::UpdateWorkspaceHistory` payloads or backend logs to diagnose why the data is ignored.
- Verify the TreeView reflects the cleared selection by inspecting the actual checkbox state after the restore Task completes.
- Once persistence works, capture the resulting `.novywave_global` diff in the plan and remove temporary logging.
