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

    selected_variables.select_scope(None);
}

/// Process file picker selection and handle new/existing files (synchronous)
pub fn process_selected_file_paths(
    tracked_files: &crate::tracked_files::TrackedFiles,
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
        tracked_files.add_dropped_files(new_files);
    }
}

/// Process file picker selection (synchronous - no task spawning needed)
pub fn process_file_picker_selection(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_files: Vec<String>,
    file_dialog_visible: &Mutable<bool>,
) {
    process_selected_file_paths(tracked_files, selected_files);
    file_dialog_visible.set(false);
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

    tracked_files.clear_all_files();
}

/// Extract filename from a full path
pub fn extract_filename(path: &str) -> String {
    path.split('/').last().unwrap_or(path).to_string()
}
