use zoon::*;
use shared::{ScopeData, file_contains_scope};
use shared::LoadingStatus;
use crate::state::{LOADING_FILES, IS_LOADING, LOADED_FILES, TREE_SELECTED_ITEMS, EXPANDED_SCOPES, USER_CLEARED_SELECTION, STARTUP_CURSOR_POSITION_SET};
use std::collections::HashSet;

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
        IS_LOADING.set_neq(false);
        
        // Restore scope selections using proper signal sequencing
        restore_scope_selections_sequenced();
        
        // Check if cursor position was set during startup - re-trigger value queries if so
        if STARTUP_CURSOR_POSITION_SET.get() {
            // Use unified caching logic with built-in range checking
            crate::views::trigger_signal_value_queries();
            STARTUP_CURSOR_POSITION_SET.set_neq(false); // Reset flag
        }
        
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
    
    // MIGRATION NOTE: This function should be moved into an Actor event loop
    // that caches current values and responds to file loading events reactively
    
    // Temporary: Get current scope from signal for migration compatibility
    let _domain = crate::actors::global_domains::selected_variables_domain();
    let scope_to_restore: Option<String> = None; // TODO: Replace with proper reactive pattern in Actor event loop
    
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
                
                let mut items = TREE_SELECTED_ITEMS.lock_mut();
                if !items.contains(&tree_id) {
                    items.clear(); // Single selection mode
                    items.insert(tree_id);
                }
                drop(items);
                
                // Clear the user cleared flag since we successfully restored
                USER_CLEARED_SELECTION.set_neq(false);
                
                // Also expand parent scopes (batched to avoid duplicates)
                let loaded_files = LOADED_FILES.lock_ref();
                let files_copy = loaded_files.to_vec();
                drop(loaded_files); // Drop lock early
                
                // Use single batch expansion to prevent duplicate calls
                for file in files_copy.iter() {
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
    
    // MIGRATION NOTE: This function should be moved into an Actor event loop
    // Check if there's a saved selected_scope_id to restore
    let _domain = crate::actors::global_domains::selected_variables_domain();
    let scope_to_restore: Option<String> = None; // TODO: Replace with proper reactive pattern in Actor event loop
    
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
                let mut items = TREE_SELECTED_ITEMS.lock_mut();
                if !items.contains(&tree_id) {
                    items.clear(); // Single selection mode
                    items.insert(tree_id);
                }
                drop(items);
                
                // Clear the user cleared flag since we successfully restored
                USER_CLEARED_SELECTION.set_neq(false);
                
                // Also expand parent scopes for this specific file
                expand_parent_scopes(&scopes_clone, &scope_id_clone);
            });
        }
    }
}


fn expand_parent_scopes(scopes: &[ScopeData], target_scope_id: &str) {
    static EXPANSION_IN_PROGRESS: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));
    
    // Prevent recursive expansion calls for same scope
    if EXPANSION_IN_PROGRESS.lock_ref().contains(target_scope_id) {
        // Already expanding, skip to prevent infinite recursion
        return;
    }
    
    EXPANSION_IN_PROGRESS.lock_mut().insert(target_scope_id.to_string());
    
    // Collect all parent scopes that need expansion (batch operation)
    let mut scopes_to_expand = Vec::new();
    collect_parent_scopes_recursive(scopes, target_scope_id, &mut scopes_to_expand);
    
    // Batch update: Add all parent scopes in single operation to minimize signal firing
    if !scopes_to_expand.is_empty() {
        let mut expanded = EXPANDED_SCOPES.lock_mut();
        let mut added_count = 0;
        for scope_id in scopes_to_expand {
            if expanded.insert(scope_id) {
                added_count += 1;
            }
        }
        drop(expanded); // Trigger signals only once after batch operation
        
        if added_count > 0 {
        }
    }
    
    EXPANSION_IN_PROGRESS.lock_mut().remove(target_scope_id);
}

/// Recursively collect all parent scope IDs that contain the target scope
fn collect_parent_scopes_recursive(scopes: &[ScopeData], target_scope_id: &str, result: &mut Vec<String>) {
    for scope in scopes {
        if scope.id == target_scope_id {
            // Found the target scope, don't need to go deeper
            return;
        }
        
        if scope_contains_target(&scope.children, target_scope_id) {
            // This scope is a parent of the target, add it to expansion list
            result.push(scope.id.clone());
            // Recursively collect from children
            collect_parent_scopes_recursive(&scope.children, target_scope_id, result);
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


