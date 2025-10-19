# Workspace Picker Polishing – Current Findings

## What Works
- `last_selected` and `recent_paths` are properly serialized into `.novywave_global` whenever a new workspace is chosen.
- The Open Workspace tree renders checkboxes for folders, so workspaces can be selected directly.
- The dialog clears prior selection when it opens (checkboxes are unchecked until the user picks a folder).

## Outstanding Issues

1. **Picker tree state persists as empty**
   - Browser console traces show `expanded /…` events firing for every folder click.
   - The actor currently subscribed to `expanded_directories_actor.state.signal_cloned()` often receives `{}` right after each expansion, so the payload forwarded to `update_workspace_picker_tree_state` becomes an empty vector.
   - Backend logs confirm the message arrives (`🔧 BACKEND: UpdateWorkspaceHistory … picker=Some(WorkspaceTreeState { expanded_paths: [] })`), but `.novywave_global` never gains `picker_tree_state` because the vector is always empty.
   - Suspected cause: the actor snapshot happens before `expanded_directories_actor` mutates its state. Need to mirror the domain’s expand/collapse relays (and possibly wait a `next_tick`) or maintain an independent set updated from the relay events.

2. **Multiple folders remain checked**
   - TreeView currently allows multi-select for non-scope items. Even after clearing selection on dialog open, users can check more than one workspace at a time.
   - Tree component exposes `single_scope_selection`, but there is no equivalent for directory nodes; we may need to handle folder clicks manually (clear before setting) or post-process the selected vector inside the dialog.

3. **Config reload wipes picker state**
   - On dialog open we reapply `picker_tree_state`, but the domain immediately publishes the (still empty) snapshot back to the backend, overwriting the stored data.
   - Need to short-circuit the initial sync—or write a guard that suppresses the first publish until after we have replayed the saved state.

## Next Steps / Ideas
- Subscribe directly to `directory_expanded_relay` / `directory_collapsed_relay`, keep a local `IndexSet` in the actor, and only flush after the mutation (possibly after `Task::next_tick().await`).
- Defer the first persistence until after we replay config (`workspace_history_restore_actor`), so we don’t immediately send an empty snapshot.
- For single-folder selection, intercept `selected_files_vec_signal` updates and trim to one entry before writing to `workspace_history_state`.
- Once the above sticks, remove all temporary debug logging (`zyon::println!`, backend println) before final hand-off.
- Feel free to sprinkle temporary debug prints directly inside actors/relays (with file:line comments) to trace message flow; just clean them up when the investigation is done.
