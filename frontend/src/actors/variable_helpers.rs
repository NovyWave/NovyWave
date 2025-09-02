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
    zoon::println!("🔍 Looking for file_id: '{}', scope_id: '{}'", file_id, scope_id);
    
    // Find context information from tracked files - USE SAME SOURCE AS Variables panel
    let tracked_files = if let Some(signals) = crate::actors::global_domains::TRACKED_FILES_SIGNALS.get() {
        signals.files_mutable.lock_ref().to_vec()
    } else {
        Vec::new()
    };
    
    zoon::println!("📂 Available files: {}", tracked_files.len());
    
    for (i, f) in tracked_files.iter().enumerate() {
        zoon::println!("  File {}: id='{}', path='{}'", i, f.id, f.path);
    }
    
    let file = tracked_files.iter().find(|f| f.id == file_id);
    if file.is_none() {
        zoon::println!("❌ File not found with id: '{}'", file_id);
        return None;
    }
    let file = file.unwrap();
    
    // Find scope full name from the file state
    let scope_full_name = if let FileState::Loaded(waveform_file) = &file.state {
        zoon::println!("✅ File is loaded, finding scope: '{}'", scope_id);
        find_scope_full_name(&waveform_file.scopes, scope_id)
            .unwrap_or_else(|| scope_id.to_string())
    } else {
        zoon::println!("⚠️ File not loaded, using scope_id as fallback");
        scope_id.to_string()
    };
    
    // Create SelectedVariable with the same logic as the legacy function
    let selected_var = SelectedVariable::new(
        variable,
        file.path.clone(),
        scope_full_name,
    );
    
    zoon::println!("✅ Created SelectedVariable: {}", selected_var.unique_id);
    Some(selected_var)
}

/// Check if a variable is already selected
/// 
/// This checks both the legacy global state and domain state during transition
pub fn _is_variable_selected(unique_id: &str) -> bool {
    // During transition, check the legacy index
    // Use domain synchronous access for variable index
    let index = crate::actors::selected_variables::current_variable_index();
    index.contains(unique_id)
}