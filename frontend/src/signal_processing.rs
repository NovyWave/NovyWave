use shared::{FileState, SelectedVariable, TrackedFile};
use zoon::*;

/// Get signal type information for a selected variable (signal-based version)
pub fn get_signal_type_for_selected_variable_from_files(
    selected_var: &SelectedVariable,
    files: &[TrackedFile],
) -> String {
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
        for tracked_file in files.iter() {
            if tracked_file.canonical_path == file_path || tracked_file.path == file_path {
                if let FileState::Loaded(waveform_file) = &tracked_file.state {
                    let full_scope_id = format!("{}|{}", file_path, scope_path);

                    if let Some(variables) =
                        shared::find_variables_in_scope(&waveform_file.scopes, &full_scope_id)
                    {
                        if let Some(signal) = variables.iter().find(|v| v.name == variable_name) {
                            return format!("{} {}-bit", signal.signal_type, signal.width);
                        }
                    }
                }
                break; // Found the file, no need to continue searching
            }
        }
    }

    String::new()
}
