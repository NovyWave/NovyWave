use zoon::*;
use shared::{Signal, WaveformFile, ScopeData, file_contains_scope, collect_variables_from_scopes};
use crate::types::{LoadingFile, LoadingStatus};
use crate::state::{LOADING_FILES, IS_LOADING, LOADED_FILES, SELECTED_SCOPE_ID, TREE_SELECTED_ITEMS, EXPANDED_SCOPES};

pub fn check_loading_complete() {
    let loading_files = LOADING_FILES.lock_ref();
    let all_done = loading_files.iter().all(|f| {
        matches!(f.status, LoadingStatus::Completed | LoadingStatus::Error(_))
    });
    
    if all_done {
        IS_LOADING.set(false);
        
        // Restore scope selections - defer signal updates to prevent deadlock
        restore_scope_selections_deferred();
        
        // Clear completed files after a delay to show final state
        Task::start(async {
            Timer::sleep(2000).await;
            LOADING_FILES.lock_mut().clear();
        });
    }
}

pub fn restore_scope_selections_deferred() {
    // Check if there's a saved selected_scope_id to restore
    let scope_to_restore = SELECTED_SCOPE_ID.get_cloned();
    
    if let Some(scope_id) = scope_to_restore {
        // Validate that the scope still exists in loaded files
        let loaded_files = LOADED_FILES.lock_ref();
        let is_valid = loaded_files.iter().any(|file| file_contains_scope(&file.scopes, &scope_id));
        
        if is_valid {
            Task::start(async move {
                Timer::sleep(100).await; // Small delay to ensure UI updates complete
                
                // Restore TreeView selection to match the persisted scope
                zoon::println!("Restoring scope selection: {}", scope_id);
                TREE_SELECTED_ITEMS.lock_mut().insert(scope_id.clone());
                
                // Re-trigger SELECTED_SCOPE_ID signal to update variables panel
                SELECTED_SCOPE_ID.set(Some(scope_id.clone()));
                
                // Also expand parent scopes
                let loaded_files = LOADED_FILES.lock_ref();
                for file in loaded_files.iter() {
                    expand_parent_scopes(&file.scopes, &scope_id);
                }
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

pub fn init_scope_selection() {
    let files = LOADED_FILES.lock_ref();
    if files.is_empty() {
        return;
    }
    
    // Find the first scope with variables (depth-first search)
    if let Some(first_scope_with_vars) = find_first_scope_with_variables(&files) {
        zoon::println!("Auto-selecting first scope with variables: {}", first_scope_with_vars);
        SELECTED_SCOPE_ID.set_neq(Some(first_scope_with_vars.clone()));
        TREE_SELECTED_ITEMS.lock_mut().insert(first_scope_with_vars.clone());
        
        
        // Expand parent scopes
        for file in files.iter() {
            expand_parent_scopes(&file.scopes, &first_scope_with_vars);
        }
    }
}

fn find_first_scope_with_variables(files: &[WaveformFile]) -> Option<String> {
    for file in files {
        if let Some(scope_id) = find_scope_with_variables_recursive(&file.scopes) {
            return Some(scope_id);
        }
    }
    None
}

fn find_scope_with_variables_recursive(scopes: &[ScopeData]) -> Option<String> {
    for scope in scopes {
        if !scope.variables.is_empty() {
            return Some(scope.id.clone());
        }
        
        if let Some(child_scope_id) = find_scope_with_variables_recursive(&scope.children) {
            return Some(child_scope_id);
        }
    }
    None
}

pub fn get_all_variables_from_files() -> Vec<Signal> {
    let mut variables = Vec::new();
    for file in LOADED_FILES.lock_ref().iter() {
        collect_variables_from_scopes(&file.scopes, &mut variables);
    }
    variables
}

pub fn get_variables_from_selected_scope(selected_scope_id: &str) -> Vec<Signal> {
    let mut variables = Vec::new();
    for file in LOADED_FILES.lock_ref().iter() {
        if let Some(scope_vars) = shared::find_variables_in_scope(&file.scopes, selected_scope_id) {
            variables.extend(scope_vars);
        }
    }
    variables
}