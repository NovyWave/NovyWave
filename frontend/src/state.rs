use zoon::*;
use std::collections::HashMap;
use indexmap::{IndexMap, IndexSet};
use shared::{WaveformFile, LoadingFile, FileSystemItem, TrackedFile, FileState};
// use crate::visualizer::timeline::time_types::{TimeNs, TimelineCache}; // Unused
// use crate::config::app_config; // Unused
// Using simpler queue approach with MutableVec

// ===== STABLE SIGNAL HELPERS =====






/// ✅ ACTOR+RELAY: Batch loading function using TrackedFiles domain
/// 
/// Migrated from legacy global mutables to Actor+Relay architecture.
/// All file operations now go through the TrackedFiles domain.
pub fn _batch_load_files(file_paths: Vec<String>) {
    if file_paths.is_empty() {
        return;
    }
    
    // Use TrackedFiles domain instead of legacy TRACKED_FILES
    let tracked_files_domain = crate::actors::global_domains::tracked_files_domain();
    tracked_files_domain.batch_load_files(file_paths);
}

/// Helper function to clean up file-related state during batch operations
fn _cleanup_file_related_state_for_batch(file_id: &str) {
    // Clear scope selections for this file
    crate::TREE_SELECTED_ITEMS.lock_mut().retain(|id| !id.starts_with(&format!("scope_{}_", file_id)));
    
    // Clear variables from this file using domain events
    let current_vars = crate::actors::selected_variables::current_variables();
    let vars_to_remove: Vec<String> = current_vars.iter()
        .filter(|var| var.file_path().as_ref().map(|path| path.as_str()) == Some(file_id))
        .map(|var| var.unique_id.clone())
        .collect();
    
    // Send remove events for each variable from this file
    for var_id in vars_to_remove {
        crate::actors::selected_variables::variable_removed_relay().send(var_id);
    }
    
    // Clear expanded scopes for this file using domain events
    let current_scopes = crate::actors::selected_variables::current_expanded_scopes();
    let scopes_to_collapse: Vec<String> = current_scopes.iter()
        .filter(|scope_id| scope_id.starts_with(&format!("{}_", file_id)))
        .cloned()
        .collect();
    
    // Send collapse events for each scope from this file
    for scope_id in scopes_to_collapse {
        crate::actors::selected_variables::scope_collapsed_relay().send(scope_id);
    }
}





// ===== LEGACY FILE UPDATE QUEUE SYSTEM - REPLACED BY TRACKEDFILES DOMAIN =====
// This entire section is now handled by the TrackedFiles domain and can be removed
// after migration is complete. Keeping for reference during migration.

/*
// Message queue system to prevent recursive locking
static FILE_UPDATE_QUEUE: Lazy<Mutable<Vec<FileUpdateMessage>>> = Lazy::new(|| {
    // ... (queue processing code now handled by TrackedFiles domain) ...
});
*/


// ===== MIGRATED TO ACTOR+RELAY: Panel dragging state =====
// These mutables have been migrated to visualizer/interaction/dragging.rs system
// Use functions from that module and actors/panel_layout.rs instead of these globals

// Selected Variables panel row height
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;

// ===== MIGRATED TO ACTOR+RELAY: WaveformTimeline domain =====
// These mutables have been migrated to actors/waveform_timeline.rs
// Use functions from that module instead of these globals

// MIGRATED: Timeline cursor position → use cursor_position_signal() from waveform_timeline
// pub static _TIMELINE_CURSOR_NS: Lazy<Mutable<TimeNs>> = Lazy::new(|| Mutable::new(TimeNs::ZERO));

// MIGRATED: Timeline viewport → use viewport_signal() from waveform_timeline  
// pub static _TIMELINE_VIEWPORT: Lazy<Mutable<Viewport>> = Lazy::new(|| {
//     Mutable::new(Viewport::new(
//         TimeNs::ZERO,
// TimeNs::from_external_seconds(100.0) // Default 100 second range
//     ))
// });

// MIGRATED: Timeline resolution → use ns_per_pixel_signal() from waveform_timeline
// pub static _TIMELINE_NS_PER_PIXEL: Lazy<Mutable<NsPerPixel>> = Lazy::new(|| Mutable::new(NsPerPixel::MEDIUM_ZOOM));

// MIGRATED: Timeline coordinates → use coordinates_signal() from waveform_timeline
// pub static _TIMELINE_COORDINATES: Lazy<Mutable<TimelineCoordinates>> = Lazy::new(|| {
//     Mutable::new(TimelineCoordinates::new(
//         TimeNs::ZERO,                    // cursor_ns
//         TimeNs::ZERO,                    // viewport_start_ns
//         NsPerPixel::MEDIUM_ZOOM,         // ns_per_pixel
//         800                              // canvas_width_pixels (initial value)
//     ))
// });

// ===== MIGRATED TO ACTOR+RELAY: WaveformTimeline domain (15 more mutables) =====

// ===== MIGRATED TO VISUALIZER/STATE/TIMELINE_STATE.RS =====
// All timeline-related globals have been moved to visualizer/state/timeline_state.rs
// Use imports from there instead:

// MIGRATED: Canvas dimensions → use canvas_width_signal() / canvas_height_signal() from waveform_timeline
// pub static CANVAS_WIDTH: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(800.0));
// pub static CANVAS_HEIGHT: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(400.0));


// ❌ ANTIPATTERN: Global mutable state - TODO: Move to DialogManager Actor
// Input focus tracking for keyboard control prevention
#[deprecated(note = "Use DialogManager Actor with Atom for local UI state instead of global mutables")]
pub static VARIABLES_SEARCH_INPUT_FOCUSED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));





// ❌ ANTIPATTERN: Global mutable state - TODO: Move to DialogManager Actor
// File picker state for TreeView-based browser
#[deprecated(note = "Use DialogManager Actor for file picker state instead of global mutables")]
pub static FILE_PICKER_EXPANDED: Lazy<Mutable<IndexSet<String>>> = lazy::default();
#[deprecated(note = "Use DialogManager Actor for error handling instead of global mutables")]
pub static FILE_PICKER_ERROR_CACHE: Lazy<Mutable<HashMap<String, String>>> = lazy::default();




// Config initialization complete flag removed - config loaded in main with await

// ❌ ANTIPATTERN: Global mutable state - TODO: Complete migration to DialogManager Actor
// Hierarchical file tree storage - maps directory path to its contents
#[deprecated(note = "Use DialogManager.file_tree_cache Actor instead of global mutables")]
pub static FILE_TREE_CACHE: Lazy<Mutable<HashMap<String, Vec<FileSystemItem>>>> = lazy::default();

// ❌ ANTIPATTERN: Global mutable state - TODO: Complete migration to TrackedFiles domain
// Enhanced file tracking system - replaces LOADED_FILES, LOADING_FILES, and FILE_PATHS  
#[deprecated(note = "Use tracked_files_domain() signals instead of global mutables")]
pub static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
#[deprecated(note = "Use tracked_files_domain().is_loading_signal() instead of global mutables")]
pub static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();



// ❌ ANTIPATTERN: Legacy support global mutables - TODO: Complete removal after TrackedFiles migration
// Legacy support during transition - will be removed later
#[deprecated(note = "Use tracked_files_domain().loading_files_signal() instead of global mutables")]
pub static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
#[deprecated(note = "Use tracked_files_domain().loaded_files_signal() instead of global mutables")]
pub static LOADED_FILES: Lazy<MutableVec<WaveformFile>> = lazy::default();
#[deprecated(note = "Use tracked_files_domain().file_paths_signal() instead of global mutables")]
pub static FILE_PATHS: Lazy<Mutable<IndexMap<String, String>>> = lazy::default();

// ❌ ANTIPATTERN: Global mutable state - TODO: Move to SelectedVariables Actor
#[deprecated(note = "Use selected_variables_domain().selected_scope_signal() instead of global mutables")]
pub static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = Lazy::new(|| {
    let mutable = Mutable::new(None);
    mutable
});
pub static TREE_SELECTED_ITEMS: Lazy<Mutable<IndexSet<String>>> = lazy::default(); // UI state only - not persisted
pub static USER_CLEARED_SELECTION: Lazy<Mutable<bool>> = lazy::default(); // Flag to prevent unwanted restoration

// ❌ ANTIPATTERN: Global mutable state - TODO: Move to SelectedVariables Actor
// Track expanded scopes for TreeView persistence
#[deprecated(note = "Use selected_variables_domain().expanded_scopes_signal() instead of global mutables")]
pub static EXPANDED_SCOPES: Lazy<Mutable<IndexSet<String>>> = Lazy::new(|| {
    let expanded = Mutable::new(IndexSet::new());
    
    // State changes monitored by ConfigSaver automatically
    
    expanded
});

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
    #[allow(dead_code)]
    pub error_type: ErrorType,
    #[allow(dead_code)]
    pub timestamp: u64,
    pub auto_dismiss_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    FileParsingError { 
        #[allow(dead_code)]
        file_id: String, 
        #[allow(dead_code)]
        filename: String 
    },
    DirectoryAccessError { 
        #[allow(dead_code)]
        path: String 
    },
    ConnectionError,
    #[allow(dead_code)]
    ConfigError,
    ClipboardError,
}

impl ErrorAlert {
    pub fn new_file_parsing_error(file_id: String, filename: String, error: String) -> Self {
        let user_friendly_message = make_error_user_friendly(&error);
        Self {
            id: format!("file_error_{}", file_id),
            title: "File Loading Error".to_string(),
            message: format!("{}: {}", filename, user_friendly_message),
            technical_error: format!("Error parsing file {}: {}", file_id, error),
            error_type: ErrorType::FileParsingError { file_id, filename },
            timestamp: js_sys::Date::now() as u64,
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
            error_type: ErrorType::DirectoryAccessError { path },
            timestamp: js_sys::Date::now() as u64,
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
            error_type: ErrorType::ConnectionError,
            timestamp: js_sys::Date::now() as u64,
            auto_dismiss_ms: 5000, // Default 5s, will be overridden by config in error_display
        }
    }
    
    pub fn new_clipboard_error(error: String) -> Self {
        Self {
            id: format!("clipboard_error_{}", js_sys::Date::now() as u64),
            title: "Clipboard Error".to_string(),
            message: "Failed to copy to clipboard. Your browser may not support clipboard access or you may need to use HTTPS.".to_string(),
            technical_error: format!("Clipboard operation failed: {}", error),
            error_type: ErrorType::ClipboardError,
            timestamp: js_sys::Date::now() as u64,
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
    
    if error_lower.contains("unknown file format") || error_lower.contains("only ghw, fst and vcd are supported") {
        if let Some(path) = file_path {
            format!("Unsupported file format '{}'. Only VCD and FST files are supported.", path)
        } else {
            "Unsupported file format. Only VCD and FST files are supported.".to_string()
        }
    } else if error_lower.contains("file not found") || error_lower.contains("no such file") {
        if let Some(path) = file_path {
            format!("File not found '{}'. Please check if the file exists and try again.", path)
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

// ❌ LEGACY: Replaced by ErrorManager domain (actors/error_manager.rs)
// pub static ERROR_ALERTS: Lazy<MutableVec<ErrorAlert>> = lazy::default();
// pub static TOAST_NOTIFICATIONS: Lazy<MutableVec<ErrorAlert>> = lazy::default();

// ===== TRACKED FILES MANAGEMENT UTILITIES =====


/// ✅ ACTOR+RELAY: Update the state of an existing tracked file
/// Migrated to use TrackedFiles domain
pub fn update_tracked_file_state(file_id: &str, new_state: FileState) {
    let tracked_files_domain = crate::actors::global_domains::tracked_files_domain();
    tracked_files_domain.update_file_state(file_id.to_string(), new_state);
}

/// ✅ ACTOR+RELAY: Remove a tracked file by ID
/// Migrated to use TrackedFiles domain
pub fn _remove_tracked_file(file_id: &str) {
    let tracked_files_domain = crate::actors::global_domains::tracked_files_domain();
    tracked_files_domain.remove_file(file_id.to_string());
}






// ===== SELECTED VARIABLES MANAGEMENT =====


/// Remove a variable from the selected list - DEPRECATED: Use Actor+Relay domain instead
pub fn _remove_selected_variable(_unique_id: &str) {
    // DEPRECATED: This function has been replaced by Actor+Relay architecture
    // Use crate::actors::selected_variables::variable_removed_relay().send(unique_id) instead
    panic!("remove_selected_variable is deprecated - use Actor+Relay domain");
}

/// Clear all selected variables - DEPRECATED: Use Actor+Relay domain instead
pub fn _clear_selected_variables() {
    // DEPRECATED: This function has been replaced by Actor+Relay architecture
    // Use crate::actors::selected_variables::selection_cleared_relay().send(()) instead
    panic!("clear_selected_variables is deprecated - use Actor+Relay domain");
}


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

/// Save selected variables to config
#[allow(dead_code)]
pub fn save_selected_variables() {
    // Config is now automatically saved through ConfigSaver actor
}




// =============================================================================
// DERIVED SIGNALS FOR CONFIG - Single Source of Truth for Config Serialization
// =============================================================================


/// Derived signal that converts EXPANDED_SCOPES (IndexSet) to Vec<String> for config storage
/// Converts EXPANDED_SCOPES (IndexSet) to Vec<String> for config storage
pub static EXPANDED_SCOPES_FOR_CONFIG: Lazy<Mutable<Vec<String>>> = Lazy::new(|| {
    let derived = Mutable::new(Vec::new());
    
    // Initialize the derived signal with current EXPANDED_SCOPES
    let derived_clone = derived.clone();
    Task::start(async move {
        EXPANDED_SCOPES.signal_ref(|expanded_set| expanded_set.clone())
            .for_each(move |expanded_set| {
                let derived = derived_clone.clone();
                async move {
                    // Process scope changes for config storage
                    // Strip TreeView "scope_" prefixes before storing to config
                    let expanded_vec: Vec<String> = expanded_set.into_iter()
                            .map(|scope_id| {
                                if scope_id.starts_with("scope_") {
                                    scope_id.strip_prefix("scope_").unwrap_or(&scope_id).to_string()
                                } else {
                                    scope_id
                                }
                            })
                            .collect();
                    
                    // Debug logging removed - ConfigSaver handles monitoring
                    derived.set_neq(expanded_vec);
                }
            })
            .await;
    });
    
    derived
});


/// Derived signal that converts SELECTED_SCOPE_ID (Option<String>) to Option<String> for config storage  
/// Strips TreeView "scope_" prefixes before storing to config
pub static SELECTED_SCOPE_ID_FOR_CONFIG: Lazy<Mutable<Option<String>>> = Lazy::new(|| {
    let derived = Mutable::new(None);
    
    // Initialize the derived signal with current SELECTED_SCOPE_ID
    let derived_clone = derived.clone();
    Task::start(async move {
        SELECTED_SCOPE_ID.signal_ref(|selected_scope| selected_scope.clone())
            .for_each(move |selected_scope| {
                let derived = derived_clone.clone();
                async move {
                    // Process selected scope for config storage
                    // Strip TreeView "scope_" prefix before storing to config
                    let config_scope = selected_scope.as_ref().map(|scope_id| {
                        if scope_id.starts_with("scope_") {
                            scope_id.strip_prefix("scope_").unwrap_or(scope_id).to_string()
                        } else {
                            scope_id.clone()
                        }
                    });
                    
                    derived.set_neq(config_scope);
                }
            })
            .await;
    });
    
    derived
});

/// Derived signal that converts global selected variables to Vec<SelectedVariable> for config storage
pub static SELECTED_VARIABLES_FOR_CONFIG: Lazy<Mutable<Vec<shared::SelectedVariable>>> = Lazy::new(|| {
    let derived = Mutable::new(Vec::new());
    
    // ✅ FIXED: Connect to Actor's state directly (single source of truth)
    let derived_clone = derived.clone();
    Task::start(async move {
        // Watch the Actor's state directly through global domain access
        crate::actors::global_domains::selected_variables_signal()
            .for_each(move |variables| {
                let derived = derived_clone.clone();
                async move {
                    derived.set_neq(variables);
                }
            })
            .await;
    });
    
    derived
});

// =============================================================================
// SELECTED SCOPE SYNCHRONIZATION - Bi-directional sync between UI and persistence
// =============================================================================

/// Initialize synchronization between SELECTED_SCOPE_ID (persisted) and TREE_SELECTED_ITEMS (UI state)
pub fn initialize_selected_scope_synchronization() {
    // 1. SELECTED_SCOPE_ID → TREE_SELECTED_ITEMS (config load to UI)
    Task::start(async move {
        SELECTED_SCOPE_ID.signal_ref(|selected_scope| selected_scope.clone())
            .for_each(|selected_scope| async move {
                if let Some(scope_id) = selected_scope {
                    let mut tree_selected = TREE_SELECTED_ITEMS.lock_mut();
                    tree_selected.clear();
                    tree_selected.insert(scope_id);
                } else {
                    TREE_SELECTED_ITEMS.lock_mut().clear();
                }
            })
            .await;
    });
    
    // 2. TREE_SELECTED_ITEMS → SELECTED_SCOPE_ID (user selection to persistence)
    Task::start(async move {
        TREE_SELECTED_ITEMS.signal_ref(|tree_selected| tree_selected.clone())
            .for_each(|tree_selected| async move {
                let selected_scope = if tree_selected.is_empty() {
                    None
                } else if tree_selected.len() == 1 {
                    tree_selected.iter().next().cloned()
                } else {
                    // Multiple selections - take the first one (single_scope_selection should prevent this)
                    tree_selected.iter().next().cloned()
                };
                
                SELECTED_SCOPE_ID.set_neq(selected_scope);
            })
            .await;
    });
}





