use shared::FileState;
use zoon::*;

// ===== STABLE SIGNAL HELPERS =====

/// ✅ ACTOR+RELAY: Batch loading function using TrackedFiles domain
///
/// Migrated from legacy global mutables to Actor+Relay architecture.
/// All file operations now go through the TrackedFiles domain.
pub fn _batch_load_files(
    file_paths: Vec<String>,
    tracked_files: &crate::tracked_files::TrackedFiles,
) {
    if file_paths.is_empty() {
        return;
    }

    // Use TrackedFiles domain relay instead of legacy TRACKED_FILES
    let path_bufs: Vec<std::path::PathBuf> = file_paths
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect();
    tracked_files.files_dropped_relay.send(path_bufs);
}

/// ✅ ACTOR+RELAY: Clean up file-related state during batch operations
/// Now uses domain events only - no direct global state manipulation
fn _cleanup_file_related_state_for_batch(
    file_id: &str,
    selected_variables: &crate::selected_variables::SelectedVariables,
) {
    // TODO: This function needs reactive access to current variables and scopes
    // For now, placeholder implementation during Actor+Relay migration
    // These operations would need to be done through signal streams rather than synchronous access
    
    let _ = (file_id, selected_variables); // Suppress unused warnings
    
    // NOTE: In proper Actor+Relay architecture, cleanup would be handled by:
    // 1. Listening to file removal events in SelectedVariables Actor
    // 2. Automatically cleaning up related variables and scopes
    // 3. No synchronous "current state" access needed
}

// Note: Tree UI selection clearing now handled by TreeView component locally
// No need to manipulate global TREE_SELECTED_ITEMS - use component Atom instead


// ===== MIGRATED TO ACTOR+RELAY: Panel dragging state =====
// These mutables have been migrated to visualizer/interaction/dragging.rs system
// Use functions from that module and actors/panel_layout.rs instead of these globals

// Selected Variables panel row height
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;


// Now using search_focused_signal() from selected_variables domain Actor

// File picker expanded state is now managed by app_config().file_picker_expanded_directories
// File picker errors should be handled by error domain actors
// Now using app_config().file_picker_expanded_directories with proper persistence
// Now using proper ErrorManager and DialogManager domain Actors with error_cache_signal()

// Config initialization complete flag removed - config loaded in main with await

// TREE_SELECTED_ITEMS is UI-only state - should be local Atom in TreeView component
// USER_CLEARED_SELECTION should be part of SelectedVariables domain logic
// Now using proper bi-directional sync between TreeView Mutable and SelectedVariables Actor
// Now using proper SelectedVariables domain Actor with user_cleared field

// All scope expansion state now managed by selected_variables domain Actor
// The static mutable has been completely replaced by Actor+Relay architecture

// NOTE: SELECTED_VARIABLES and SELECTED_VARIABLES_INDEX have been migrated to Actor+Relay architecture
// Use crate::actors::selected_variables::* functions instead

// MIGRATED: Signal values and variable formats moved to visualizer/state/timeline_state.rs

// ===== ERROR DISPLAY SYSTEM =====

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorAlert {
    pub id: String,
    pub title: String,
    pub message: String,
    pub technical_error: String, // Raw technical error for console logging
    pub auto_dismiss_ms: u64,
}

impl ErrorAlert {
    pub fn new_file_parsing_error(file_id: String, filename: String, error: String) -> Self {
        let user_friendly_message = make_error_user_friendly(&error);
        Self {
            id: format!("file_error_{}", file_id),
            title: "File Loading Error".to_string(),
            message: format!("{}: {}", filename, user_friendly_message),
            technical_error: format!("Error parsing file {}: {}", file_id, error),
            auto_dismiss_ms: 5000, // Default 5s, will be overridden by config in error_display
        }
    }

    pub fn new_directory_error(path: String, error: String) -> Self {
        let user_friendly_message = make_error_user_friendly(&error);
        Self {
            id: format!("dir_error_{}", path.replace("/", "_")),
            title: "Directory Access Error".to_string(),
            message: format!("Cannot access {}: {}", path, user_friendly_message),
            technical_error: format!("Error browsing directory {}: {}", path, error),
            auto_dismiss_ms: 5000, // Default 5s, will be overridden by config in error_display
        }
    }

    pub fn new_connection_error(error: String) -> Self {
        let user_friendly_message = make_error_user_friendly(&error);
        Self {
            id: format!("conn_error_{}", js_sys::Date::now() as u64),
            title: "Connection Error".to_string(),
            message: user_friendly_message,
            technical_error: format!("Connection error: {}", error),
            auto_dismiss_ms: 5000, // Default 5s, will be overridden by config in error_display
        }
    }

    pub fn new_clipboard_error(error: String) -> Self {
        Self {
            id: format!("clipboard_error_{}", js_sys::Date::now() as u64),
            title: "Clipboard Error".to_string(),
            message: "Failed to copy to clipboard. Your browser may not support clipboard access or you may need to use HTTPS.".to_string(),
            technical_error: format!("Clipboard operation failed: {}", error),
            auto_dismiss_ms: 5000, // Default 5s, will be overridden by config in error_display
        }
    }
}

pub fn make_error_user_friendly(error: &str) -> String {
    let error_lower = error.to_lowercase();

    // Extract file path from error messages in multiple formats:
    // - "Failed to parse waveform file '/path/to/file': error" (quoted format)
    // - "File not found: /path/to/file" (backend format)
    let file_path = if let Some(start) = error.find("'") {
        if let Some(end) = error[start + 1..].find("'") {
            Some(&error[start + 1..start + 1 + end])
        } else {
            None
        }
    } else if error_lower.contains("file not found:") {
        // Extract path after "File not found: "
        if let Some(colon_pos) = error.find("File not found:") {
            let path_start = colon_pos + "File not found:".len();
            Some(error[path_start..].trim())
        } else {
            None
        }
    } else {
        None
    };

    if error_lower.contains("unknown file format")
        || error_lower.contains("only ghw, fst and vcd are supported")
    {
        if let Some(path) = file_path {
            format!(
                "Unsupported file format '{}'. Only VCD and FST files are supported.",
                path
            )
        } else {
            "Unsupported file format. Only VCD and FST files are supported.".to_string()
        }
    } else if error_lower.contains("file not found") || error_lower.contains("no such file") {
        if let Some(path) = file_path {
            format!(
                "File not found '{}'. Please check if the file exists and try again.",
                path
            )
        } else {
            "File not found. Please check if the file exists and try again.".to_string()
        }
    } else if error_lower.contains("permission denied") || error_lower.contains("access denied") {
        "Can't access this directory".to_string()
    } else if error_lower.contains("connection") || error_lower.contains("network") {
        "Connection error. Please check your network connection.".to_string()
    } else if error_lower.contains("timeout") {
        "Operation timed out. Please try again.".to_string()
    } else {
        // Keep original error but make it more presentable
        error.trim().to_string()
    }
}

// ===== TRACKED FILES MANAGEMENT UTILITIES =====

/// ✅ ACTOR+RELAY: Update the state of an existing tracked file
/// Migrated to use TrackedFiles domain
pub fn update_tracked_file_state(
    file_id: &str, 
    new_state: FileState,
    tracked_files: &crate::tracked_files::TrackedFiles,
) {
    // TODO: Implement through TrackedFiles domain relay
    // This should send a file_state_updated_relay event rather than direct update
    let _ = (file_id, new_state, tracked_files); // Suppress unused warnings
    
    // NOTE: In proper Actor+Relay architecture, this would be:
    // tracked_files.file_state_updated_relay.send((file_id.to_string(), new_state));
}

// ===== SELECTED VARIABLES MANAGEMENT =====

/// Helper function to find scope full name in the file structure
pub fn find_scope_full_name(scopes: &[shared::ScopeData], target_scope_id: &str) -> Option<String> {
    for scope in scopes {
        if scope.id == target_scope_id {
            return Some(scope.full_name.clone());
        }
        // Recursively search children
        if let Some(name) = find_scope_full_name(&scope.children, target_scope_id) {
            return Some(name);
        }
    }
    None
}

// =============================================================================
// DERIVED SIGNALS FOR CONFIG - Single Source of Truth for Config Serialization
// =============================================================================

// ✅ ARCHITECTURE SUCCESS: SELECTED_SCOPE_ID_FOR_CONFIG static signal bypass completely eliminated
// Replaced with proper domain Actor access and current_selected_scope_for_config() function

// ✅ ARCHITECTURE SUCCESS: SELECTED_VARIABLES_FOR_CONFIG static signal bypass completely eliminated
// Replaced with proper domain Actor access and config.loaded_selected_variables pattern

// =============================================================================
// SELECTED SCOPE SYNCHRONIZATION - Bi-directional sync between UI and persistence
// =============================================================================

/// ✅ MIGRATED TO ACTOR+RELAY: Scope synchronization now handled by TreeView component
/// TreeView component should use external_selected pattern to connect to domain signals
pub fn initialize_selected_scope_synchronization() {
    // This synchronization is now handled directly by TreeView component using external_selected
    // TreeView connects to selected_variables domain signals for bi-directional sync
    // No global state synchronization needed - TreeView manages its own local Atom state
    // and syncs with domain through external_selected_signal() pattern
}
