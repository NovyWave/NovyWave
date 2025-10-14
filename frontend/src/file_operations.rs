use crate::dataflow::atom::Atom;
use shared::{CanonicalPathPayload, TrackedFile};
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
        known_paths.insert(tracked.canonical_path.clone());
    }

    let mut new_files: Vec<PathBuf> = Vec::new();
    let mut reload_payloads: Vec<CanonicalPathPayload> = Vec::new();
    let mut reload_seen: HashSet<String> = HashSet::new();

    for selected_path in selected_files {
        let selected_pathbuf = PathBuf::from(&selected_path);
        let display_path = selected_pathbuf.to_string_lossy().to_string();
        let canonical_path = std::fs::canonicalize(&selected_pathbuf)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| display_path.clone());

        if let Some(existing) = tracked_files_snapshot
            .iter()
            .find(|tracked| tracked.canonical_path == canonical_path)
        {
            if reload_seen.insert(existing.canonical_path.clone()) {
                reload_payloads.push(CanonicalPathPayload::new(existing.canonical_path.clone()));
            }
            continue;
        }

        if known_paths.insert(canonical_path.clone()) {
            new_files.push(selected_pathbuf);
        }
    }

    if !reload_payloads.is_empty() {
        tracked_files.reload_existing_paths(reload_payloads);
    }
    if !new_files.is_empty() {
        tracked_files.files_dropped_relay.send(new_files);
    }
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
