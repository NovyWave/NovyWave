use shared::{Signal, WaveformFile, ScopeData, find_variables_in_scope};

// ===== FRONTEND-SPECIFIC TYPES =====

#[derive(Clone, Debug)]
pub enum LoadingStatus {
    Starting,
    Parsing,
    Completed,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct LoadingFile {
    pub file_id: String,
    pub filename: String,
    pub progress: f32,
    pub status: LoadingStatus,
}

// ===== FRONTEND-SPECIFIC UTILITY FUNCTIONS =====

pub fn get_variables_from_selected_scope(selected_scope_id: &str) -> Vec<Signal> {
    use crate::state::LOADED_FILES;
    
    let loaded_files = LOADED_FILES.lock_ref();
    for file in loaded_files.iter() {
        if let Some(variables) = find_variables_in_scope(&file.scopes, selected_scope_id) {
            return variables;
        }
    }
    Vec::new()
}

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