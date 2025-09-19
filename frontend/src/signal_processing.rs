use shared::{FileState, SelectedVariable, TrackedFile, UpMsg};
use zoon::*;

/// Get signal type information for a selected variable (signal-based version)
pub fn get_signal_type_for_selected_variable_from_files(
    selected_var: &SelectedVariable,
    files: &[TrackedFile],
) -> String {
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
        for tracked_file in files.iter() {
            if tracked_file.path == file_path {
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

/// Get signal type information for a selected variable (legacy synchronous version)
pub fn get_signal_type_for_selected_variable(
    selected_var: &SelectedVariable,
    tracked_files: &[TrackedFile],
) -> String {
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
        for tracked_file in tracked_files.iter() {
            if tracked_file.path == file_path {
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

/// Check if cursor time is within a variable's file time range
pub fn is_cursor_within_variable_time_range(
    unique_id: &str,
    cursor_time: f64,
    tracked_files: &[TrackedFile],
) -> bool {
    let parts: Vec<&str> = unique_id.splitn(3, '|').collect();
    if parts.len() < 3 {
        return true; // Assume valid if we can't parse (maintains existing behavior)
    }
    let file_path = parts[0];

    if let Some(tracked_file) = tracked_files.iter().find(|f| f.path == file_path) {
        if let shared::FileState::Loaded(loaded_file) = &tracked_file.state {
            if let (Some(min_time), Some(max_time)) = (
                loaded_file
                    .min_time_ns
                    .map(|ns| ns as f64 / 1_000_000_000.0),
                loaded_file
                    .max_time_ns
                    .map(|ns| ns as f64 / 1_000_000_000.0),
            ) {
                cursor_time >= min_time && cursor_time <= max_time
            } else {
                true
            }
        } else {
            false
        }
    } else {
        true
    }
}

/// Trigger signal value queries when variables are present
pub fn trigger_signal_value_queries(tracked_files: &[TrackedFile]) {
    let has_loaded_files = tracked_files
        .iter()
        .any(|f| matches!(f.state, shared::FileState::Loaded(_)));

    if !has_loaded_files {
        return; // Don't query if no files are loaded yet
    }
}
