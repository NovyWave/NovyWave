use zoon::*;
use shared::{ScopeData, file_contains_scope};
use shared::LoadingStatus;
use crate::state::{LOADING_FILES, IS_LOADING, LOADED_FILES, SELECTED_SCOPE_ID, TREE_SELECTED_ITEMS, EXPANDED_SCOPES, USER_CLEARED_SELECTION};

// Signal for completion state changes - triggers clearing of completed files
static LOADING_COMPLETION_TRIGGER: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

// Signal for UI update sequencing - ensures proper signal ordering
static UI_UPDATE_SEQUENCE: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

pub fn check_loading_complete() {
    let loading_files = LOADING_FILES.lock_ref();
    let all_done = loading_files.iter().all(|f| {
        matches!(f.status, LoadingStatus::Completed | LoadingStatus::Error(_))
    });
    
    if all_done {
        IS_LOADING.set(false);
        
        // Restore scope selections using proper signal sequencing
        restore_scope_selections_sequenced();
        
        // Trigger file clearing through signal chain instead of timer
        LOADING_COMPLETION_TRIGGER.update(|count| count + 1);
    }
}

pub fn restore_scope_selections_sequenced() {
    // Check if user has explicitly cleared selection - if so, don't restore
    if USER_CLEARED_SELECTION.get() {
        // Skip scope restoration - user cleared selection
        return;
    }
    
    // Check if there's a saved selected_scope_id to restore
    let scope_to_restore = SELECTED_SCOPE_ID.get_cloned();
    
    if let Some(scope_id) = scope_to_restore {
        // Validate that the scope still exists in loaded files
        let loaded_files = LOADED_FILES.lock_ref();
        let is_valid = loaded_files.iter().any(|file| file_contains_scope(&file.scopes, &scope_id));
        
        if is_valid {
            // Use signal sequencing instead of timer for proper coordination
            let scope_id_clone = scope_id.clone();
            
            // Schedule scope restoration to occur after UI updates complete
            Task::start(async move {
                // Wait for next UI update cycle using signal coordination
                let current_sequence = UI_UPDATE_SEQUENCE.get();
                UI_UPDATE_SEQUENCE.set(current_sequence + 1);
                
                // Restore TreeView selection to match the persisted scope
                // Convert scope ID to TreeView format (with "scope_" prefix)
                let tree_id = format!("scope_{}", scope_id_clone);
                TREE_SELECTED_ITEMS.lock_mut().insert(tree_id);
                
                // Re-trigger SELECTED_SCOPE_ID signal to update variables panel
                SELECTED_SCOPE_ID.set(Some(scope_id_clone.clone()));
                
                // Clear the user cleared flag since we successfully restored
                USER_CLEARED_SELECTION.set(false);
                
                // Also expand parent scopes
                let loaded_files = LOADED_FILES.lock_ref();
                for file in loaded_files.iter() {
                    expand_parent_scopes(&file.scopes, &scope_id_clone);
                }
            });
        }
    }
}

/// Restore scope selection immediately for a specific file (per-file loading)
pub fn restore_scope_selection_for_file(loaded_file: &shared::WaveformFile) {
    // Check if user has explicitly cleared selection - if so, don't restore
    if USER_CLEARED_SELECTION.get() {
        return;
    }
    
    // Check if there's a saved selected_scope_id to restore
    let scope_to_restore = SELECTED_SCOPE_ID.get_cloned();
    
    if let Some(scope_id) = scope_to_restore {
        // Check if this specific file contains the scope we want to restore
        if file_contains_scope(&loaded_file.scopes, &scope_id) {
            // Use signal sequencing for proper coordination
            let scope_id_clone = scope_id.clone();
            let scopes_clone = loaded_file.scopes.clone();
            
            // Schedule scope restoration to occur after UI updates complete
            Task::start(async move {
                // Wait for next UI update cycle using signal coordination
                let current_sequence = UI_UPDATE_SEQUENCE.get();
                UI_UPDATE_SEQUENCE.set(current_sequence + 1);
                
                // Restore TreeView selection to match the persisted scope
                // Convert scope ID to TreeView format (with "scope_" prefix)
                let tree_id = format!("scope_{}", scope_id_clone);
                TREE_SELECTED_ITEMS.lock_mut().insert(tree_id);
                
                // Re-trigger SELECTED_SCOPE_ID signal to update variables panel
                SELECTED_SCOPE_ID.set(Some(scope_id_clone.clone()));
                
                // Clear the user cleared flag since we successfully restored
                USER_CLEARED_SELECTION.set(false);
                
                // Also expand parent scopes for this specific file
                expand_parent_scopes(&scopes_clone, &scope_id_clone);
            });
        }
    }
}


fn expand_parent_scopes(scopes: &[ScopeData], target_scope_id: &str) {
    for scope in scopes {
        if scope.id == target_scope_id {
            // Found the target scope, expand all parent scopes in the path
            return;
        }
        
        if scope_contains_target(&scope.children, target_scope_id) {
            // This scope is a parent of the target, expand it
            EXPANDED_SCOPES.lock_mut().insert(scope.id.clone());
            // Recursively expand children
            expand_parent_scopes(&scope.children, target_scope_id);
        }
    }
}

fn scope_contains_target(scopes: &[ScopeData], target_scope_id: &str) -> bool {
    for scope in scopes {
        if scope.id == target_scope_id {
            return true;
        }
        if scope_contains_target(&scope.children, target_scope_id) {
            return true;
        }
    }
    false
}




// Initialize signal-based file clearing on loading completion
pub fn init_signal_chains() {
    // Set up signal chain to clear completed files after completion is shown
    Task::start(async {
        LOADING_COMPLETION_TRIGGER.signal().for_each_sync(move |_| {
            // Clear files after completion state is visually confirmed
            // Use signal coordination instead of arbitrary timer
            Task::start(async {
                // Wait for completion state to be visible (deterministic signal-based)
                IS_LOADING.signal().wait_for(false).await;
                
                // Clear the completed loading files
                LOADING_FILES.lock_mut().clear();
            });
        }).await;
    });
}


