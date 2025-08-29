//! Variable Selection Helper Functions
//!
//! Utility functions for converting raw variable data to SelectedVariable objects
//! for use with the Actor+Relay architecture.

use shared::{SelectedVariable, Signal, FileState};
use crate::state::{TRACKED_FILES, find_scope_full_name};

/// Create a SelectedVariable from raw variable data and context
/// 
/// This replicates the logic from the legacy add_selected_variable function,
/// but returns the SelectedVariable instead of directly modifying global state.
pub fn create_selected_variable(
    variable: Signal, 
    file_id: &str, 
    scope_id: &str
) -> Option<SelectedVariable> {
    // Find context information from tracked files
    let tracked_files = TRACKED_FILES.lock_ref();
    let file = tracked_files.iter().find(|f| f.id == file_id)?;
    
    // Find scope full name from the file state
    let scope_full_name = if let FileState::Loaded(waveform_file) = &file.state {
        find_scope_full_name(&waveform_file.scopes, scope_id)
            .unwrap_or_else(|| scope_id.to_string())
    } else {
        scope_id.to_string()
    };
    
    // Create SelectedVariable with the same logic as the legacy function
    let selected_var = SelectedVariable::new(
        variable,
        file.path.clone(),
        scope_full_name,
    );
    
    Some(selected_var)
}

/// Check if a variable is already selected
/// 
/// This checks both the legacy global state and domain state during transition
pub fn _is_variable_selected(unique_id: &str) -> bool {
    // During transition, check the legacy index
    // This will be updated once SelectedVariables domain bridge is implemented
    use crate::state::SELECTED_VARIABLES_INDEX;
    let index = SELECTED_VARIABLES_INDEX.lock_ref();
    index.contains(unique_id)
}