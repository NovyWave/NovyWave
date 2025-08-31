use zoon::*;
use std::collections::HashMap;
use indexmap::{IndexMap, IndexSet};
use shared::{WaveformFile, LoadingFile, FileSystemItem, TrackedFile, FileState};
use crate::time_types::{TimeNs, TimelineCache};
use crate::config::app_config;
// Using simpler queue approach with MutableVec

// ===== STABLE SIGNAL HELPERS =====

/// ✅ MIGRATED: Count-only signal for file counts in titles and UI
pub fn tracked_files_count_signal() -> impl Signal<Item = usize> {
    crate::actors::file_count_signal()
}

/// ✅ MIGRATED: Signal that tracks count of files that are actually LOADED with data
pub fn loaded_files_count_signal() -> impl Signal<Item = usize> {
    crate::actors::loaded_files_count_signal()
}




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




// ===== FILE UPDATE MESSAGE QUEUE SYSTEM =====

#[derive(Debug, Clone)]
pub enum FileUpdateMessage {
    Update { file_id: String, new_state: FileState },
    #[allow(dead_code)]
    Remove { file_id: String },
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

/// ✅ ACTOR+RELAY: Send a message to the file update processor
/// Migrated to use TrackedFiles domain (legacy queue system removed)
pub fn send_file_update_message(_message: FileUpdateMessage) {
    // Note: File update messaging is now handled internally by the TrackedFiles domain
    // This function is kept for compatibility but does nothing - callers should use
    // domain methods directly (update_file_state, remove_file, etc.)
}

// Panel resizing state
pub static FILES_PANEL_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 470.into());
pub static FILES_PANEL_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 300.into());
#[allow(dead_code)]
pub static VERTICAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
#[allow(dead_code)]
pub static HORIZONTAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Variables panel column resizing state
pub static VARIABLES_NAME_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 180.into());
pub static VARIABLES_VALUE_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 100.into());
#[allow(dead_code)]
pub static VARIABLES_NAME_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
#[allow(dead_code)]
pub static VARIABLES_VALUE_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

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

// MIGRATED: Timeline cache → use unified_timeline_cache_signal() from waveform_timeline
pub static UNIFIED_TIMELINE_CACHE: Lazy<Mutable<TimelineCache>> = Lazy::new(|| Mutable::new(TimelineCache::new()));

// MIGRATED: Cursor initialization → use startup_cursor_position_set_signal() from waveform_timeline
pub static STARTUP_CURSOR_POSITION_SET: Lazy<Mutable<bool>> = lazy::default();

// MIGRATED: Zoom control → use is_zooming_in_signal() / is_zooming_out_signal() from waveform_timeline
pub static IS_ZOOMING_IN: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_ZOOMING_OUT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Pan control → use is_panning_left_signal() / is_panning_right_signal() from waveform_timeline
pub static IS_PANNING_LEFT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_PANNING_RIGHT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Cursor movement → use is_cursor_moving_left/right_signal() from waveform_timeline
pub static IS_CURSOR_MOVING_LEFT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_CURSOR_MOVING_RIGHT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Shift key → use is_shift_pressed_signal() from waveform_timeline
pub static IS_SHIFT_PRESSED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Mouse tracking → use mouse_x_position_signal() / mouse_time_ns_signal() from waveform_timeline
pub static MOUSE_X_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(0.0));
pub static MOUSE_TIME_NS: Lazy<Mutable<TimeNs>> = Lazy::new(|| Mutable::new(TimeNs::ZERO));

// MIGRATED: Zoom center → use zoom_center_ns_signal() from waveform_timeline
pub static ZOOM_CENTER_NS: Lazy<Mutable<TimeNs>> = Lazy::new(|| Mutable::new(TimeNs::ZERO));

// MIGRATED: Canvas dimensions → use canvas_width_signal() / canvas_height_signal() from waveform_timeline
// pub static CANVAS_WIDTH: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(800.0));
// pub static CANVAS_HEIGHT: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(400.0));

// Search filter for Variables panel
pub static VARIABLES_SEARCH_FILTER: Lazy<Mutable<String>> = lazy::default();

// Input focus tracking for keyboard control prevention
pub static VARIABLES_SEARCH_INPUT_FOCUSED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));


// Dock state management - DEFAULT TO DOCKED MODE  
pub static IS_DOCKED_TO_BOTTOM: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(true));

// File dialog state
pub static SHOW_FILE_DIALOG: Lazy<Mutable<bool>> = lazy::default();
pub static FILE_PATHS_INPUT: Lazy<Mutable<String>> = lazy::default();

// Dock toggle state to prevent cascading saves
pub static DOCK_TOGGLE_IN_PROGRESS: Lazy<Mutable<bool>> = lazy::default();

// File picker state for TreeView-based browser
pub static FILE_PICKER_EXPANDED: Lazy<Mutable<IndexSet<String>>> = lazy::default();
pub static FILE_PICKER_SELECTED: Lazy<MutableVec<String>> = lazy::default();
pub static CURRENT_DIRECTORY: Lazy<Mutable<String>> = lazy::default();
pub static FILE_PICKER_ERROR: Lazy<Mutable<Option<String>>> = lazy::default();
pub static FILE_PICKER_ERROR_CACHE: Lazy<Mutable<HashMap<String, String>>> = lazy::default();


// Test viewport scrolling for Load Files dialog  
pub static LOAD_FILES_VIEWPORT_Y: Lazy<Mutable<i32>> = lazy::default();

// Load Files dialog scroll position (persistent)
pub static LOAD_FILES_SCROLL_POSITION: Lazy<Mutable<i32>> = lazy::default();

// Config initialization complete flag removed - config loaded in main with await

// Hierarchical file tree storage - maps directory path to its contents
pub static FILE_TREE_CACHE: Lazy<Mutable<HashMap<String, Vec<FileSystemItem>>>> = lazy::default();

// Enhanced file tracking system - replaces LOADED_FILES, LOADING_FILES, and FILE_PATHS  
pub static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
pub static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();

// Cache of tracked file IDs - prevents recursive locking in signal handlers
pub static TRACKED_FILE_IDS: Lazy<Mutable<IndexSet<String>>> = lazy::default();


// Legacy support during transition - will be removed later
pub static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
pub static LOADED_FILES: Lazy<MutableVec<WaveformFile>> = lazy::default();
pub static FILE_PATHS: Lazy<Mutable<IndexMap<String, String>>> = lazy::default();

pub static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = Lazy::new(|| {
    let mutable = Mutable::new(None);
    // Debug logging to track scope selection changes
    Task::start(mutable.signal_cloned().for_each(|scope_id| async move {
        zoon::println!("🎯 SELECTED_SCOPE_ID changed to: {:?}", scope_id);
    }));
    mutable
});
pub static TREE_SELECTED_ITEMS: Lazy<Mutable<IndexSet<String>>> = lazy::default(); // UI state only - not persisted
pub static USER_CLEARED_SELECTION: Lazy<Mutable<bool>> = lazy::default(); // Flag to prevent unwanted restoration

// Track expanded scopes for TreeView persistence
pub static EXPANDED_SCOPES: Lazy<Mutable<IndexSet<String>>> = Lazy::new(|| {
    let expanded = Mutable::new(IndexSet::new());
    
    // State changes monitored by ConfigSaver automatically
    
    expanded
});

// NOTE: SELECTED_VARIABLES and SELECTED_VARIABLES_INDEX have been migrated to Actor+Relay architecture
// Use crate::actors::selected_variables::* functions instead

// MIGRATED: Signal values → use signal_values_signal() from waveform_timeline
pub static SIGNAL_VALUES: Lazy<Mutable<HashMap<String, crate::format_utils::SignalValue>>> = lazy::default();

// MIGRATED: Variable formats → use selected_variable_formats_signal() from waveform_timeline
pub static SELECTED_VARIABLE_FORMATS: Lazy<Mutable<HashMap<String, shared::VarFormat>>> = lazy::default();

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
    pub auto_dismiss_ms: Option<u64>,
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
            auto_dismiss_ms: Some(5000), // Default 5 second dismiss timeout for errors
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
            auto_dismiss_ms: Some(5000), // Default 5 second dismiss timeout for errors
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
            auto_dismiss_ms: Some(5000), // Default 5 second dismiss timeout for errors
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
            auto_dismiss_ms: Some(5000), // Default 5 second dismiss timeout
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



/// ✅ ACTOR+RELAY: Get all file paths currently being tracked  
/// MIGRATION: This function should be converted to reactive pattern
/// TODO: Replace synchronous access with tracked_files_signal()
pub fn get_all_tracked_file_paths() -> Vec<String> {
    // MIGRATION: Temporary fallback - should use reactive signals instead
    Vec::new() // Default empty list during migration
}



// ===== SELECTED VARIABLES MANAGEMENT =====

/// Add a variable to the selected list - DEPRECATED: Use Actor+Relay domain instead
pub fn add_selected_variable(_variable: shared::Signal, _file_id: &str, _scope_id: &str) -> bool {
    // DEPRECATED: This function has been replaced by Actor+Relay architecture
    // Use crate::actors::selected_variables::variable_clicked_relay().send(unique_id) instead
    // The new domain handles variable creation from file data internally
    panic!("add_selected_variable is deprecated - use Actor+Relay domain");
}

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

/// Check if a variable is already selected - DEPRECATED: Use Actor+Relay domain instead
pub fn is_variable_selected(_file_path: &str, _scope_path: &str, _variable_name: &str) -> bool {
    // DEPRECATED: This function has been replaced by Actor+Relay architecture
    // Use crate::actors::selected_variables::variable_index_signal() instead
    panic!("is_variable_selected is deprecated - use Actor+Relay domain");
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

/// Derived signal that automatically converts TRACKED_FILES to Vec<String> for config storage
pub static OPENED_FILES_FOR_CONFIG: Lazy<Mutable<Vec<String>>> = Lazy::new(|| {
    let derived = Mutable::new(Vec::new());
    
    // Initialize the derived signal with current TRACKED_FILES
    let derived_clone = derived.clone();
    Task::start(async move {
        TRACKED_FILES.signal_vec_cloned()
            .len()
            .dedupe()
            .for_each(move |_| {
                let derived = derived_clone.clone();
                async move {
                    let files: Vec<TrackedFile> = TRACKED_FILES.lock_ref().to_vec();
                    let file_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
                    derived.set_neq(file_paths);
                }
            })
            .await;
    });
    
    derived
});

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

/// Derived signal that converts FILE_PICKER_EXPANDED (IndexSet) to Vec<String> for config storage
/// Converts FILE_PICKER_EXPANDED (IndexSet) to Vec<String> for config storage
pub static LOAD_FILES_EXPANDED_DIRECTORIES_FOR_CONFIG: Lazy<Mutable<Vec<String>>> = Lazy::new(|| {
    let derived = Mutable::new(Vec::new());
    
    // Initialize the derived signal with current FILE_PICKER_EXPANDED
    let derived_clone = derived.clone();
    Task::start(async move {
        FILE_PICKER_EXPANDED.signal_ref(|expanded_set| expanded_set.clone())
            // Note: Guard flag prevents circular triggers more effectively than dedupe
            .for_each(move |expanded_set| {
                let derived = derived_clone.clone();
                async move {
                    // Process file picker directory changes for config storage
                    let expanded_vec: Vec<String> = expanded_set.into_iter().collect();
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





