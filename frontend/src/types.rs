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



/// Get variables from a specific scope using actors (enables per-file loading)

pub fn get_variables_from_tracked_files(selected_scope_id: &str) -> Vec<VariableWithContext> {
    use shared::{find_variables_in_scope}; // Removed unused FileState
    
    // Parse scope_ format correctly - it's needed for TreeView identification
    // The scope ID format is: "scope_{file_path}|{scope_path}"
    let scope_for_lookup = if selected_scope_id.starts_with("scope_") {
        &selected_scope_id[6..] // Remove "scope_" prefix for file scope lookup
    } else {
        selected_scope_id
    };
    
    // Get tracked files from actor system
    let tracked_files = if let Some(signals) = crate::actors::global_domains::TRACKED_FILES_SIGNALS.get() {
        signals.files_mutable.lock_ref().to_vec()
    } else {
        Vec::new()
    };
    
    // Find variables in any loaded file that matches the scope
    for tracked_file in tracked_files.iter() {
        if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
            if let Some(variables) = find_variables_in_scope(&waveform_file.scopes, scope_for_lookup) {
                // Extract just the scope path from scope_for_lookup
                // scope_for_lookup format: "/path/to/file|scope_path" - we want just "scope_path"
                let scope_path = if let Some(pipe_pos) = scope_for_lookup.find('|') {
                    &scope_for_lookup[pipe_pos + 1..]
                } else {
                    scope_for_lookup
                };
                
                return variables.into_iter().map(|signal| VariableWithContext {
                    signal,
                    file_id: tracked_file.id.clone(),
                    scope_id: scope_path.to_string(),
                }).collect();
            }
        }
    }
    // No variables found in any loaded file for this scope
    Vec::new()
}