use zoon::*;
use std::collections::HashMap;
use indexmap::{IndexMap, IndexSet};
use shared::{WaveformFile, LoadingFile, FileSystemItem, TrackedFile, FileState, create_tracked_file, update_smart_labels};

// Panel resizing state
pub static FILES_PANEL_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 470.into());
pub static FILES_PANEL_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 300.into());
pub static VERTICAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
pub static HORIZONTAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Variables panel column resizing state
pub static VARIABLES_NAME_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 180.into());
pub static VARIABLES_VALUE_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 100.into());
pub static VARIABLES_NAME_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
pub static VARIABLES_VALUE_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Selected Variables panel row height
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;

// Timeline cursor position (in seconds)
pub static TIMELINE_CURSOR_POSITION: Lazy<Mutable<f64>> = Lazy::new(|| Mutable::new(10.0));

// Timeline zoom state
pub static TIMELINE_ZOOM_LEVEL: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(1.0)); // 1.0 = normal, 1B max for extreme zoom
pub static TIMELINE_VISIBLE_RANGE_START: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(0.0)); // Visible time window start
pub static TIMELINE_VISIBLE_RANGE_END: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(100.0)); // Visible time window end

// Smooth zoom control
pub static IS_ZOOMING_IN: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_ZOOMING_OUT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Smooth pan control
pub static IS_PANNING_LEFT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_PANNING_RIGHT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Smooth cursor movement control
pub static IS_CURSOR_MOVING_LEFT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_CURSOR_MOVING_RIGHT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Shift key state tracking for modifier combinations
pub static IS_SHIFT_PRESSED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Mouse position tracking for zoom center
pub static MOUSE_X_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(0.0));
pub static MOUSE_TIME_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(10.0));

// Zoom center position (in seconds) - separate from mouse position for explicit control
pub static ZOOM_CENTER_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(0.0));

// Canvas dimensions for click calculations
pub static CANVAS_WIDTH: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(800.0));
pub static CANVAS_HEIGHT: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(400.0));

// Search filter for Variables panel
pub static VARIABLES_SEARCH_FILTER: Lazy<Mutable<String>> = lazy::default();

// Input focus tracking for keyboard control prevention
pub static VARIABLES_SEARCH_INPUT_FOCUSED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// File loading trigger signal for reactive type updates
pub static FILE_LOADING_TRIGGER: Lazy<Mutable<u32>> = lazy::default();

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

// Prevent config saves during initialization to avoid race conditions
pub static CONFIG_INITIALIZATION_COMPLETE: Lazy<Mutable<bool>> = lazy::default();

// Hierarchical file tree storage - maps directory path to its contents
pub static FILE_TREE_CACHE: Lazy<Mutable<HashMap<String, Vec<FileSystemItem>>>> = lazy::default();

// Enhanced file tracking system - replaces LOADED_FILES, LOADING_FILES, and FILE_PATHS
pub static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
pub static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();

// Legacy support during transition - will be removed later
pub static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
pub static LOADED_FILES: Lazy<MutableVec<WaveformFile>> = lazy::default();
pub static FILE_PATHS: Lazy<Mutable<IndexMap<String, String>>> = lazy::default();

pub static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = lazy::default();
pub static TREE_SELECTED_ITEMS: Lazy<Mutable<IndexSet<String>>> = lazy::default(); // UI state only - not persisted
pub static USER_CLEARED_SELECTION: Lazy<Mutable<bool>> = lazy::default(); // Flag to prevent unwanted restoration

// Track expanded scopes for TreeView persistence
pub static EXPANDED_SCOPES: Lazy<Mutable<IndexSet<String>>> = lazy::default();

// Selected variables management
pub static SELECTED_VARIABLES: Lazy<MutableVec<shared::SelectedVariable>> = lazy::default();
pub static SELECTED_VARIABLES_INDEX: Lazy<Mutable<IndexSet<String>>> = lazy::default();

// Signal values for selected variables - now stores multi-format cached values
pub static SIGNAL_VALUES: Lazy<Mutable<HashMap<String, crate::format_utils::MultiFormatValue>>> = lazy::default();

// Format selections for selected variables (unique_id -> VarFormat)
pub static SELECTED_VARIABLE_FORMATS: Lazy<Mutable<HashMap<String, shared::VarFormat>>> = lazy::default();

// ===== ERROR DISPLAY SYSTEM =====

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
            auto_dismiss_ms: Some(crate::config::current_toast_dismiss_ms()), // Use configured dismiss time
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
            auto_dismiss_ms: Some(crate::config::current_toast_dismiss_ms()), // Use configured dismiss time
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
            auto_dismiss_ms: Some(crate::config::current_toast_dismiss_ms()), // Use configured dismiss time
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
            auto_dismiss_ms: Some(crate::config::current_toast_dismiss_ms()),
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

// Global error alert management
pub static ERROR_ALERTS: Lazy<MutableVec<ErrorAlert>> = lazy::default();

// Toast notification system state
pub static TOAST_NOTIFICATIONS: Lazy<MutableVec<ErrorAlert>> = lazy::default();

// ===== TRACKED FILES MANAGEMENT UTILITIES =====

/// Add a new file to tracking with initial state
pub fn add_tracked_file(file_path: String, initial_state: FileState) {
    let tracked_file = create_tracked_file(file_path, initial_state);
    
    // Check if file already exists and replace if it does
    let existing_index = TRACKED_FILES.lock_ref()
        .iter()
        .position(|f| f.id == tracked_file.id);
    
    if let Some(index) = existing_index {
        TRACKED_FILES.lock_mut().set_cloned(index, tracked_file);
    } else {
        TRACKED_FILES.lock_mut().push_cloned(tracked_file);
    }
    
    // Update smart labels for all files
    refresh_smart_labels();
}

/// Update the state of an existing tracked file
pub fn update_tracked_file_state(file_id: &str, new_state: FileState) {
    let mut tracked_files = TRACKED_FILES.lock_mut();
    
    // Find the index and update the file state
    if let Some(index) = tracked_files.iter().position(|f| f.id == file_id) {
        if let Some(file_ref) = tracked_files.iter().nth(index) {
            let mut file = file_ref.clone();
            file.state = new_state;
            tracked_files.set_cloned(index, file);
        }
    } else {
        // File ID not found in tracked files - may have been removed
    }
    drop(tracked_files); // Release lock before calling refresh_smart_labels
    
    // Refresh smart labels whenever file state changes
    refresh_smart_labels();
    
    // Trigger reactive type updates when files are loaded
    FILE_LOADING_TRIGGER.update(|count| count + 1);
}

/// Remove a tracked file by ID
pub fn remove_tracked_file(file_id: &str) {
    TRACKED_FILES.lock_mut().retain(|f| f.id != file_id);
    refresh_smart_labels();
}



/// Get all file paths currently being tracked
pub fn get_all_tracked_file_paths() -> Vec<String> {
    TRACKED_FILES.lock_ref()
        .iter()
        .map(|f| f.path.clone())
        .collect()
}

/// Refresh smart labels for all tracked files
pub fn refresh_smart_labels() {
    let mut tracked_files = TRACKED_FILES.lock_mut();
    let mut files_vec: Vec<TrackedFile> = tracked_files.iter().cloned().collect();
    
    // Generate smart labels using the shared algorithm
    update_smart_labels(&mut files_vec);
    
    // Update the MutableVec with the new smart labels
    for (index, updated_file) in files_vec.iter().enumerate() {
        if index < tracked_files.len() {
            tracked_files.set_cloned(index, updated_file.clone());
        }
    }
}

/// Initialize tracked files from config file paths (for session restoration)
pub fn init_tracked_files_from_config(file_paths: Vec<String>) {
    TRACKED_FILES.lock_mut().clear();
    
    for path in file_paths {
        add_tracked_file(path, FileState::Loading(shared::LoadingStatus::Starting));
    }
}

// ===== SELECTED VARIABLES MANAGEMENT =====

/// Add a variable to the selected list
pub fn add_selected_variable(variable: shared::Signal, file_id: &str, scope_id: &str) -> bool {
    
    // Find context information
    let tracked_files = TRACKED_FILES.lock_ref();
    let file = tracked_files.iter().find(|f| f.id == file_id);
    
    if let Some(file) = file {
        let _file_name = file.filename.clone();
        
        // Find scope full name from the file state
        let scope_full_name = if let FileState::Loaded(waveform_file) = &file.state {
            find_scope_full_name(&waveform_file.scopes, scope_id)
                .unwrap_or_else(|| scope_id.to_string())
        } else {
            scope_id.to_string()
        };
        
        let selected_var = shared::SelectedVariable::new(
            variable,
            file.path.clone(),
            scope_full_name,
        );
        
        // Check for duplicates using index
        let mut index = SELECTED_VARIABLES_INDEX.lock_mut();
        if index.contains(&selected_var.unique_id) {
            return false; // Already selected
        }
        
        // Add to both storage and index
        index.insert(selected_var.unique_id.clone());
        SELECTED_VARIABLES.lock_mut().push_cloned(selected_var.clone());
        
        // Trigger signal value queries for the newly added variable
        crate::views::trigger_signal_value_queries();
        
        // Trigger config save
        save_selected_variables();
        true
    } else {
        false // File not found
    }
}

/// Remove a variable from the selected list
pub fn remove_selected_variable(unique_id: &str) {
    // Remove from both storage and index, releasing locks immediately
    SELECTED_VARIABLES.lock_mut().retain(|var| var.unique_id != unique_id);
    SELECTED_VARIABLES_INDEX.lock_mut().shift_remove(unique_id);
    
    // Clean up transition tracking for removed variable (prevents memory leaks)
    crate::waveform_canvas::clear_transition_tracking_for_variable(unique_id);
    
    // Now safe to call save_selected_variables() with no locks held
    save_selected_variables();
}

/// Clear all selected variables
pub fn clear_selected_variables() {
    SELECTED_VARIABLES.lock_mut().clear();
    SELECTED_VARIABLES_INDEX.lock_mut().clear();
    
    // Clear all transition tracking when clearing variables (prevents memory leaks)
    crate::waveform_canvas::clear_all_transition_tracking();
    
    save_selected_variables();
}

/// Check if a variable is already selected
pub fn is_variable_selected(file_path: &str, scope_path: &str, variable_name: &str) -> bool {
    let unique_id = format!("{}|{}|{}", file_path, scope_path, variable_name);
    SELECTED_VARIABLES_INDEX.lock_ref().contains(&unique_id)
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
pub fn save_selected_variables() {
    if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
        
        // First sync current selected variables to config store
        let current_vars = SELECTED_VARIABLES.lock_ref().to_vec();
        crate::config::config_store().workspace.lock_ref().selected_variables.lock_mut().replace_cloned(current_vars);
        
        // Then save config to backend
        crate::config::save_config_to_backend();
    }
}

/// Initialize selected variables from config
pub fn init_selected_variables_from_config(selected_vars: Vec<shared::SelectedVariable>) {
    
    // Validate that referenced files/scopes still exist
    let valid_vars: Vec<shared::SelectedVariable> = selected_vars.into_iter()
        .filter(|var| {
            let is_valid = validate_selected_variable_context(var);
            if !is_valid {
            }
            is_valid
        })
        .collect();
    
    
    // Update global state
    SELECTED_VARIABLES.lock_mut().replace_cloned(valid_vars.clone());
    
    // Update index
    let index: IndexSet<String> = valid_vars.iter()
        .map(|var| var.unique_id.clone())
        .collect();
    *SELECTED_VARIABLES_INDEX.lock_mut() = index;
    
    // Trigger signal value queries if variables were restored
    if !valid_vars.is_empty() {
        crate::views::trigger_signal_value_queries();
        
        // Use optimized batch operation for config restore - prevents O(N²) during restore
        let current_range = crate::waveform_canvas::get_current_timeline_range();
        crate::waveform_canvas::batch_request_transitions_for_variables(&valid_vars, current_range);
        
        // CRITICAL: Trigger canvas redraw to ensure transitions are visible
        crate::waveform_canvas::trigger_canvas_redraw();
    }
}

/// Validate that a selected variable's context still exists
fn validate_selected_variable_context(var: &shared::SelectedVariable) -> bool {
    let tracked_files = TRACKED_FILES.lock_ref();
    
    // Check if file still exists
    if let Some(file) = tracked_files.iter().find(|f| f.filename == var.file_name().unwrap_or_default()) {
        
        match &file.state {
            // Accept variables from files that are currently loading or successfully loaded
            FileState::Loading(_) => {
                true
            },
            FileState::Loaded(waveform_file) => {
                let scope_exists = scope_exists_in_file(&waveform_file.scopes, &var.scope_path().unwrap_or_default());
                scope_exists
            },
            // Reject variables from failed, missing, or unsupported files
            FileState::Failed(_) | FileState::Missing(_) | FileState::Unsupported(_) => {
                false
            }
        }
    } else {
        false
    }
}

/// Helper to check if scope exists in file
fn scope_exists_in_file(scopes: &[shared::ScopeData], target_scope_id: &str) -> bool {
    for scope in scopes {
        if scope.id == target_scope_id {
            return true;
        }
        if scope_exists_in_file(&scope.children, target_scope_id) {
            return true;
        }
    }
    false
}