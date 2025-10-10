use crate::dataflow::atom::Atom;
use shared::TrackedFile;
use std::collections::HashSet;
use std::path::PathBuf;
use zoon::*;

/// Clean up file-related state when a file is removed
pub fn cleanup_file_related_state(
    file_id: &str,
    tracked_files: &[TrackedFile],
    selected_variables: &crate::selected_variables::SelectedVariables,
) {
    let (_filename, _file_path) = tracked_files
        .iter()
        .find(|f| f.id == file_id)
        .map(|f| (f.filename.clone(), f.path.clone()))
        .unwrap_or_else(|| (String::new(), String::new()));

    selected_variables.scope_selected_relay.send(None);
}

/// Process file picker selection and handle new/existing files
pub async fn process_selected_file_paths(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_files: Vec<String>,
) {
    if selected_files.is_empty() {
        return;
    }

    let tracked_files_snapshot = tracked_files.get_current_files();
    let mut known_paths: HashSet<String> = HashSet::new();
    for tracked in &tracked_files_snapshot {
        known_paths.insert(PathBuf::from(&tracked.id).to_string_lossy().to_string());
        known_paths.insert(PathBuf::from(&tracked.path).to_string_lossy().to_string());
    }

    let mut new_files: Vec<PathBuf> = Vec::new();

    for selected_path in selected_files {
        let selected_pathbuf = PathBuf::from(&selected_path);
        let normalized_path = selected_pathbuf.to_string_lossy().to_string();

        // Attempt to reload any existing tracked file first so UI state updates immediately.
        tracked_files.reload_file(normalized_path.clone());

        if !known_paths.contains(&normalized_path) {
            known_paths.insert(normalized_path);
            new_files.push(selected_pathbuf);
        }
    }

    if !new_files.is_empty() {
        tracked_files.files_dropped_relay.send(new_files);
    }
    // Existing files have already been enqueued via reload_file calls above.
}

pub fn process_file_picker_selection(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_files: Vec<String>,
    file_dialog_visible: Atom<bool>,
) {
    let dialog_visibility = file_dialog_visible.clone();
    Task::start({
        let tracked_files = tracked_files.clone();
        async move {
            if selected_files.is_empty() {
                dialog_visibility.set(false);
            } else {
                process_selected_file_paths(tracked_files, selected_files).await;
                dialog_visibility.set(false);
            }
        }
    }); // End of async Task::start block
}

/// Clear all files and related state
pub fn clear_all_files(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
) {
    let file_ids: Vec<String> = tracked_files
        .get_current_files()
        .iter()
        .map(|f| f.id.clone())
        .collect();

    let current_tracked_files = tracked_files.get_current_files();
    for file_id in &file_ids {
        cleanup_file_related_state(file_id, &current_tracked_files, selected_variables);
    }

    tracked_files.all_files_cleared_relay.send(());
}

/// Extract filename from a full path
pub fn extract_filename(path: &str) -> String {
    path.split('/').last().unwrap_or(path).to_string()
}
