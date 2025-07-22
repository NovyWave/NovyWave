use shared::Signal;

// ===== FRONTEND-SPECIFIC UTILITY FUNCTIONS =====


#[allow(dead_code)]
pub fn get_all_variables_from_files() -> Vec<Signal> {
    use crate::state::LOADED_FILES;
    use shared::collect_variables_from_scopes;
    
    let loaded_files = LOADED_FILES.lock_ref();
    let mut variables = Vec::new();
    for file in loaded_files.iter() {
        collect_variables_from_scopes(&file.scopes, &mut variables);
    }
    variables
}

/// Get variables from a specific scope using TRACKED_FILES (enables per-file loading)
pub fn get_variables_from_tracked_files(selected_scope_id: &str) -> Vec<Signal> {
    use crate::state::TRACKED_FILES;
    use shared::{FileState, find_variables_in_scope};
    
    let tracked_files = TRACKED_FILES.lock_ref();
    
    // Search through all loaded files in TRACKED_FILES
    for tracked_file in tracked_files.iter() {
        if let FileState::Loaded(waveform_file) = &tracked_file.state {
            if let Some(variables) = find_variables_in_scope(&waveform_file.scopes, selected_scope_id) {
                return variables;
            }
        }
    }
    Vec::new()
}