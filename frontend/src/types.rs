use shared::Signal;

#[derive(Debug, Clone)]
pub struct VariableWithContext {
    pub signal: Signal,
    pub file_id: String,
    pub scope_id: String,
}

pub fn filter_variables_with_context(variables: &[VariableWithContext], search_filter: &str) -> Vec<VariableWithContext> {
    if search_filter.is_empty() {
        variables.to_vec()
    } else {
        let filter_lower = search_filter.to_lowercase();
        variables.iter()
            .filter(|var| var.signal.name.to_lowercase().contains(&filter_lower))
            .cloned()
            .collect()
    }
}

// ===== FRONTEND-SPECIFIC UTILITY FUNCTIONS =====



/// Get variables from a specific scope using TRACKED_FILES (enables per-file loading)
pub fn get_variables_from_tracked_files(selected_scope_id: &str) -> Vec<VariableWithContext> {
    use crate::state::TRACKED_FILES;
    use shared::{FileState, find_variables_in_scope};
    
    let tracked_files = TRACKED_FILES.lock_ref();
    
    // Search through all loaded files in TRACKED_FILES
    for tracked_file in tracked_files.iter() {
        if let FileState::Loaded(waveform_file) = &tracked_file.state {
            if let Some(variables) = find_variables_in_scope(&waveform_file.scopes, selected_scope_id) {
                return variables.into_iter().map(|signal| VariableWithContext {
                    signal,
                    file_id: tracked_file.id.clone(),
                    scope_id: selected_scope_id.to_string(),
                }).collect();
            }
        }
    }
    Vec::new()
}