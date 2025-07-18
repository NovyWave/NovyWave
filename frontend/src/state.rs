use zoon::*;
use std::collections::{HashMap, HashSet};
use indexmap::IndexMap;
use shared::{WaveformFile, LoadingFile, FileSystemItem, TrackedFile, FileState, FileError, create_tracked_file, update_smart_labels};

// Panel resizing state
pub static FILES_PANEL_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 470.into());
pub static FILES_PANEL_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 300.into());
pub static VERTICAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
pub static HORIZONTAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Search filter for Variables panel
pub static VARIABLES_SEARCH_FILTER: Lazy<Mutable<String>> = lazy::default();

// Dock state management - DEFAULT TO DOCKED MODE  
pub static IS_DOCKED_TO_BOTTOM: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(true));

// File dialog state
pub static SHOW_FILE_DIALOG: Lazy<Mutable<bool>> = lazy::default();
pub static FILE_PATHS_INPUT: Lazy<Mutable<String>> = lazy::default();

// Dock toggle state to prevent cascading saves
pub static DOCK_TOGGLE_IN_PROGRESS: Lazy<Mutable<bool>> = lazy::default();

// File picker state for TreeView-based browser
pub static FILE_PICKER_EXPANDED: Lazy<Mutable<HashSet<String>>> = lazy::default();
pub static FILE_PICKER_SELECTED: Lazy<MutableVec<String>> = lazy::default();
pub static CURRENT_DIRECTORY: Lazy<Mutable<String>> = lazy::default();
pub static FILE_PICKER_DATA: Lazy<MutableVec<FileSystemItem>> = lazy::default();
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
pub static TREE_SELECTED_ITEMS: Lazy<Mutable<HashSet<String>>> = lazy::default(); // UI state only - not persisted
pub static USER_CLEARED_SELECTION: Lazy<Mutable<bool>> = lazy::default(); // Flag to prevent unwanted restoration

// Track expanded scopes for TreeView persistence
pub static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = lazy::default();

// ===== ERROR DISPLAY SYSTEM =====

#[derive(Debug, Clone)]
pub struct ErrorAlert {
    pub id: String,
    pub title: String,
    pub message: String,
    pub technical_error: String, // Raw technical error for console logging
    pub error_type: ErrorType,
    pub timestamp: u64,
    pub auto_dismiss_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum ErrorType {
    FileParsingError { file_id: String, filename: String },
    DirectoryAccessError { path: String },
    ConnectionError,
    ConfigError,
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
}

pub fn make_error_user_friendly(error: &str) -> String {
    let error_lower = error.to_lowercase();
    
    // Extract file path from error messages like "Failed to parse waveform file '/path/to/file': error"
    let file_path = if let Some(start) = error.find("'") {
        if let Some(end) = error[start + 1..].find("'") {
            Some(&error[start + 1..start + 1 + end])
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
        if let Some(path) = file_path {
            format!("Permission denied for '{}'. Please check file permissions and try again.", path)
        } else {
            "Permission denied. Please check file permissions and try again.".to_string()
        }
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
    zoon::println!("DEBUG: add_tracked_file() called with path: {}", file_path);
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
        let mut file = tracked_files.iter().nth(index).unwrap().clone();
        file.state = new_state;
        tracked_files.set_cloned(index, file);
    }
    drop(tracked_files); // Release lock before calling refresh_smart_labels
    
    // Refresh smart labels whenever file state changes
    refresh_smart_labels();
}

/// Remove a tracked file by ID
pub fn remove_tracked_file(file_id: &str) {
    TRACKED_FILES.lock_mut().retain(|f| f.id != file_id);
    refresh_smart_labels();
}

/// Get all tracked files in a specific state
pub fn get_tracked_files_by_state<F>(state_filter: F) -> Vec<TrackedFile>
where
    F: Fn(&FileState) -> bool,
{
    TRACKED_FILES.lock_ref()
        .iter()
        .filter(|f| state_filter(&f.state))
        .cloned()
        .collect()
}

/// Get all successfully loaded waveform files (for backward compatibility)
pub fn get_loaded_waveform_files() -> Vec<WaveformFile> {
    TRACKED_FILES.lock_ref()
        .iter()
        .filter_map(|f| match &f.state {
            FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
            _ => None,
        })
        .collect()
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
    zoon::println!("DEBUG: refresh_smart_labels() called");
    let mut tracked_files = TRACKED_FILES.lock_mut();
    let mut files_vec: Vec<TrackedFile> = tracked_files.iter().cloned().collect();
    
    zoon::println!("DEBUG: refresh_smart_labels() has {} files", files_vec.len());
    for file in &files_vec {
        zoon::println!("  - File: {}", file.path);
    }
    
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