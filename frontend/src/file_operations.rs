use shared::TrackedFile;
use zoon::*;
use crate::dataflow::atom::Atom;

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
pub fn process_file_picker_selection(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_files: Vec<String>,
    file_dialog_visible: Atom<bool>
) {
    Task::start(async move {

        if !selected_files.is_empty() {

            use std::path::PathBuf;

            let tracked_files_snapshot = tracked_files.get_current_files();

            let mut new_files: Vec<PathBuf> = Vec::new();
            let mut reload_files: Vec<String> = Vec::new();

            for selected_path in selected_files {
                let selected_pathbuf = PathBuf::from(&selected_path);

                if let Some(existing_file) = tracked_files_snapshot
                    .iter()
                    .find(|f| f.id == selected_path || f.path == selected_path)
                {
                    reload_files.push(existing_file.id.clone());
                } else {
                    new_files.push(selected_pathbuf);
                }
            }

            let tracked_files = &tracked_files;

            if !new_files.is_empty() {
                tracked_files.files_dropped_relay.send(new_files);
            }

            if !reload_files.is_empty() {
                for file_id in reload_files {
                    tracked_files.reload_file(file_id);
                }
            }

            file_dialog_visible.set(false);
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

/// Monitor directory expansions for file picker
/// NOTE: This function is currently unused - directory expansion monitoring is handled
/// directly by the FilePickerDomain Actor system through directory_expanded_relay
pub fn monitor_directory_expansions(expanded: std::collections::HashSet<String>, app_config: &crate::config::AppConfig) {
    // This function is deprecated - use FilePickerDomain actors instead
    zoon::println!("⚠️ monitor_directory_expansions is deprecated - use FilePickerDomain actors");
}

/// Extract filename from a full path
pub fn extract_filename(path: &str) -> String {
    path.split('/').last().unwrap_or(path).to_string()
}