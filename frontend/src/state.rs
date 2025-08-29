use zoon::*;
use std::collections::HashMap;
use indexmap::{IndexMap, IndexSet};
use shared::{WaveformFile, LoadingFile, FileSystemItem, TrackedFile, FileState, create_tracked_file};
use crate::time_types::{TimeNs, TimelineCache};
// Using simpler queue approach with MutableVec

// ===== STABLE SIGNAL HELPERS =====

/// ✅ EFFICIENT: Count-only signal for file counts in titles and UI
/// Tracks only the count of tracked files, not the full file list
pub fn tracked_files_count_signal() -> impl Signal<Item = usize> {
    TRACKED_FILES.signal_vec_cloned().len().dedupe()
}

/// ✅ NEW: Signal that tracks count of files that are actually LOADED with data
/// This is what Variables panel should use - only fires when files have scopes loaded
pub fn loaded_files_count_signal() -> impl Signal<Item = usize> {
    TRACKED_FILES.signal_vec_cloned().to_signal_cloned()
        .map(|files| {
            files.iter()
                .filter(|file| matches!(file.state, shared::FileState::Loaded(_)))
                .count()
        })
        .dedupe()
}




/// ✅ ENHANCED: Batch loading function with full backend integration
/// 
/// This replaces individual file operations with a single atomic update,
/// reducing renders from 6+ to 1 during batch file loading operations.
/// Includes duplicate detection, reloading logic, and backend communication.
pub fn _batch_load_files(file_paths: Vec<String>) {
    if file_paths.is_empty() {
        return;
    }
    
    // Get currently tracked file IDs for duplicate detection
    let existing_tracked_files: std::collections::HashMap<String, TrackedFile> = TRACKED_FILES.lock_ref()
        .iter()
        .map(|f| (f.id.clone(), f.clone()))
        .collect();
    
    // Process files and separate new vs reload
    let mut files_to_process = Vec::new();
    let mut files_to_reload = Vec::new();
    
    for file_path in file_paths {
        let file_id = shared::generate_file_id(&file_path);
        
        if existing_tracked_files.contains_key(&file_id) {
            files_to_reload.push((file_id, file_path));
        } else {
            files_to_process.push(file_path);
        }
    }
    
    // Get IDs of files being reloaded for filtering (before moving files_to_reload)
    let reload_ids: std::collections::HashSet<String> = files_to_reload.iter().map(|(id, _)| id.clone()).collect();
    
    // Handle reloads: clean up existing files first
    for (file_id, _) in &files_to_reload {
        _cleanup_file_related_state_for_batch(file_id);
        // Remove from legacy systems too
        crate::LOADED_FILES.lock_mut().retain(|f| f.id != *file_id);
        crate::FILE_PATHS.lock_mut().shift_remove(file_id);
    }
    
    // Collect all files to process (new + reloaded)
    let mut all_file_paths: Vec<String> = files_to_process;
    all_file_paths.extend(files_to_reload.into_iter().map(|(_, path)| path));
    
    if all_file_paths.is_empty() {
        return;
    }
    
    // Create TrackedFiles from paths with smart labels
    let smart_labels = shared::generate_smart_labels(&all_file_paths);
    let new_tracked_files: Vec<TrackedFile> = all_file_paths.iter()
        .map(|path| {
            let mut tracked_file = create_tracked_file(path.clone(), shared::FileState::Loading(shared::LoadingStatus::Starting));
            tracked_file.smart_label = smart_labels.get(path).unwrap_or(&tracked_file.filename).clone();
            tracked_file
        })
        .collect();
    
    // Combine existing files (minus reloaded ones) with new files
    let mut final_files: Vec<TrackedFile> = TRACKED_FILES.lock_ref()
        .iter()
        .filter(|f| !reload_ids.contains(&f.id))
        .cloned()
        .collect();
    
    final_files.extend(new_tracked_files.clone());
    
    // Single atomic update instead of multiple individual pushes
    TRACKED_FILES.lock_mut().replace_cloned(final_files);
    
    // Update the file IDs cache
    update_tracked_file_ids_cache();
    
    // Send backend requests for each file
    for tracked_file in new_tracked_files {
        // Add to legacy FILE_PATHS for compatibility
        crate::FILE_PATHS.lock_mut().insert(tracked_file.id.clone(), tracked_file.path.clone());
        
        // Send backend request
        Task::start({
            let file_path = tracked_file.path.clone();
            async move {
                use crate::platform::{Platform, CurrentPlatform};
                let _ = CurrentPlatform::send_message(shared::UpMsg::LoadWaveformFile(file_path)).await;
            }
        });
    }
    
    // Save config to persist the new file list
    crate::config::save_file_list();
}

/// Helper function to clean up file-related state during batch operations
fn _cleanup_file_related_state_for_batch(file_id: &str) {
    // Clear scope selections for this file
    crate::TREE_SELECTED_ITEMS.lock_mut().retain(|id| !id.starts_with(&format!("scope_{}_", file_id)));
    
    // Clear variables from this file
    crate::SELECTED_VARIABLES.lock_mut().retain(|var| 
        var.file_path().as_ref().map(|path| path.as_str()) != Some(file_id)
    );
    
    // Clear expanded scopes for this file  
    crate::EXPANDED_SCOPES.lock_mut().retain(|scope_id| !scope_id.starts_with(&format!("{}_", file_id)));
}




// ===== FILE UPDATE MESSAGE QUEUE SYSTEM =====

#[derive(Debug, Clone)]
pub enum FileUpdateMessage {
    Update { file_id: String, new_state: FileState },
    #[allow(dead_code)]
    Remove { file_id: String },
}

// Message queue system to prevent recursive locking
static FILE_UPDATE_QUEUE: Lazy<Mutable<Vec<FileUpdateMessage>>> = Lazy::new(|| {
    let queue = Mutable::new(Vec::new());
    
    // Start the processing task immediately when queue is first accessed
    start_queue_processor(&queue);
    
    queue
});

static QUEUE_PROCESSOR_RUNNING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

fn start_queue_processor(queue: &Mutable<Vec<FileUpdateMessage>>) {
    let queue_clone = queue.clone();
    Task::start(async move {
        // Ensure only one processor runs
        if QUEUE_PROCESSOR_RUNNING.replace(true) {
            return; // Another processor is already running
        }
        
        loop {
            // Wait for messages
            let messages = {
                let mut queue_lock = queue_clone.lock_mut();
                if queue_lock.is_empty() {
                    drop(queue_lock);
                    Timer::sleep(10).await; // Small delay to prevent busy waiting
                    continue;
                }
                
                // Take all messages and clear the queue
                std::mem::take(&mut *queue_lock)
            };
            
            // Process each message sequentially with proper event loop yielding
            for message in messages {
                // CRITICAL: Yield to the event loop between messages to ensure:
                // 1. Previous locks are fully dropped
                // 2. Signal handlers complete execution  
                // 3. DOM updates are processed
                // This prevents recursive mutex locks by allowing the JavaScript event loop
                // to run between operations, ensuring signals fire after locks are released
                Task::next_macro_tick().await;
                
                // Process the message sequentially (not concurrently!)
                process_file_update_message_sync(message).await;
            }
        }
    });
}

/// Process file update messages synchronously - the ONLY place that locks TRACKED_FILES for writing
async fn process_file_update_message_sync(message: FileUpdateMessage) {
    match message {
        
        FileUpdateMessage::Update { file_id, new_state } => {
            // Find and update the file state
            let file_index = {
                let tracked_files = TRACKED_FILES.lock_ref();
                let index = tracked_files.iter().position(|f| f.id == file_id);
                if index.is_none() {
                }
                index
            };
            
            if let Some(index) = file_index {
                let mut tracked_files = TRACKED_FILES.lock_mut();
                if let Some(mut file) = tracked_files.get(index).cloned() {
                    file.state = new_state;
                    tracked_files.set_cloned(index, file);
                }
                // Drop the lock before triggering signal
            }
            
            // Note: TreeView updates automatically via treeview_tracked_files_signal() when needed
            
            // Update the file IDs cache
            update_tracked_file_ids_cache();
        },
        
        FileUpdateMessage::Remove { file_id } => {
            TRACKED_FILES.lock_mut().retain(|f| f.id != file_id);
            
            // Update the file IDs cache
            update_tracked_file_ids_cache();
        },
    }
}

/// Update the cached file IDs to prevent recursive locking in signal handlers
fn update_tracked_file_ids_cache() {
    let tracked_files = TRACKED_FILES.lock_ref();
    let file_ids: IndexSet<String> = tracked_files.iter().map(|f| f.id.clone()).collect();
    TRACKED_FILE_IDS.set_neq(file_ids);
    
    // Removed broken trigger system that was causing over-rendering
}

/// Queue a message for processing (non-recursive)
fn queue_file_update_message(message: FileUpdateMessage) {
    FILE_UPDATE_QUEUE.lock_mut().push(message);
}

/// Send a message to the file update processor
pub fn send_file_update_message(message: FileUpdateMessage) {
    queue_file_update_message(message);
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

// Prevent config saves during initialization to avoid race conditions
pub static CONFIG_INITIALIZATION_COMPLETE: Lazy<Mutable<bool>> = lazy::default();

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

pub static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = lazy::default();
pub static TREE_SELECTED_ITEMS: Lazy<Mutable<IndexSet<String>>> = lazy::default(); // UI state only - not persisted
pub static USER_CLEARED_SELECTION: Lazy<Mutable<bool>> = lazy::default(); // Flag to prevent unwanted restoration

// Track expanded scopes for TreeView persistence
pub static EXPANDED_SCOPES: Lazy<Mutable<IndexSet<String>>> = lazy::default();

// NOTE: SELECTED_VARIABLES and SELECTED_VARIABLES_INDEX have been migrated to Actor+Relay architecture
// Use crate::actors::selected_variables::* functions instead
// TEMPORARY: Re-adding for compilation - will be removed once migration is complete
pub static SELECTED_VARIABLES: Lazy<MutableVec<shared::SelectedVariable>> = Lazy::new(|| MutableVec::new());
pub static SELECTED_VARIABLES_INDEX: Lazy<Mutable<indexmap::IndexSet<String>>> = Lazy::new(|| Mutable::new(indexmap::IndexSet::new()));

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


/// Update the state of an existing tracked file
pub fn update_tracked_file_state(file_id: &str, new_state: FileState) {
    // Use message queue to prevent recursive locking
    send_file_update_message(FileUpdateMessage::Update {
        file_id: file_id.to_string(),
        new_state,
    });
}

/// Remove a tracked file by ID
pub fn _remove_tracked_file(file_id: &str) {
    // Use message queue to prevent recursive locking
    send_file_update_message(FileUpdateMessage::Remove {
        file_id: file_id.to_string(),
    });
}



/// Get all file paths currently being tracked
pub fn get_all_tracked_file_paths() -> Vec<String> {
    TRACKED_FILES.lock_ref()
        .iter()
        .map(|f| f.path.clone())
        .collect()
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
    if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
        
        // First sync current selected variables to config store
        let current_vars = SELECTED_VARIABLES.lock_ref().to_vec();
        let mut workspace = crate::config::config_store().workspace.current_value();
        workspace.selected_variables = current_vars;
        crate::config::config_store().workspace.set(workspace);
        
        // Then save config to backend
        crate::config::save_config_to_backend();
    }
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
/// Uses CONFIG_INITIALIZATION_COMPLETE guard and deduplication to prevent circular loops and flickering
pub static EXPANDED_SCOPES_FOR_CONFIG: Lazy<Mutable<Vec<String>>> = Lazy::new(|| {
    let derived = Mutable::new(Vec::new());
    
    // Initialize the derived signal with current EXPANDED_SCOPES
    let derived_clone = derived.clone();
    Task::start(async move {
        EXPANDED_SCOPES.signal_ref(|expanded_set| expanded_set.clone())
            .for_each(move |expanded_set| {
                let derived = derived_clone.clone();
                async move {
                    // GUARD: Only process changes after initial config load is complete
                    // This prevents circular loops and flickering during config initialization
                    if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
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
                        
                        derived.set_neq(expanded_vec);
                    }
                }
            })
            .await;
    });
    
    derived
});

/// Derived signal that converts FILE_PICKER_EXPANDED (IndexSet) to Vec<String> for config storage
/// Uses CONFIG_INITIALIZATION_COMPLETE guard and debouncing to prevent circular loops
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
                    // GUARD: Only process changes after initial config load is complete
                    // This prevents circular loops during config initialization
                    if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                        let expanded_vec: Vec<String> = expanded_set.into_iter().collect();
                        derived.set_neq(expanded_vec);
                    }
                }
            })
            .await;
    });
    
    derived
});

/// DEPRECATED: SELECTED_VARIABLES_FOR_CONFIG replaced by Actor+Relay domain
/// Use crate::actors::selected_variables::variables_signal() for reactive access
pub static SELECTED_VARIABLES_FOR_CONFIG: Lazy<Mutable<Vec<shared::SelectedVariable>>> = Lazy::new(|| {
    let deprecated = Mutable::new(Vec::new());
    
    // TODO: Remove this deprecated signal once all references are migrated
    // Use crate::actors::selected_variables::variables_signal() instead
    deprecated
});

/// Derived signal for dock mode based on IS_DOCKED_TO_BOTTOM
pub static DOCK_MODE_FOR_CONFIG: Lazy<Mutable<shared::DockMode>> = Lazy::new(|| {
    let derived = Mutable::new(shared::DockMode::Bottom);
    
    let derived_clone = derived.clone();
    Task::start(async move {
        IS_DOCKED_TO_BOTTOM.signal()
            .for_each(move |is_docked_to_bottom| {
                let derived = derived_clone.clone();
                async move {
                    let dock_mode = if is_docked_to_bottom {
                        shared::DockMode::Bottom
                    } else {
                        shared::DockMode::Right
                    };
                    derived.set_neq(dock_mode);
                }
            })
            .await;
    });
    
    derived
});

/// Deduplicated EXPANDED_SCOPES signal specifically for TreeView to prevent unnecessary re-renders
pub static EXPANDED_SCOPES_FOR_TREEVIEW: Lazy<Mutable<IndexSet<String>>> = Lazy::new(|| {
    let derived = Mutable::new(IndexSet::new());
    
    // CRITICAL FIX: Immediate sync on initialization - get current value first
    // This ensures TreeView gets config-loaded expansion state immediately
    let current_expanded = EXPANDED_SCOPES.get_cloned();
    derived.set_neq(current_expanded);
    
    // Also track future changes with deduplication
    let derived_clone = derived.clone();
    Task::start(async move {
        let _ = EXPANDED_SCOPES.signal_ref(|expanded_set| expanded_set.clone())
            .for_each_sync(move |expanded_set| {
                // set_neq provides automatic deduplication - only signals if value actually changed
                derived_clone.set_neq(expanded_set);
            });
    });
    
    derived
});

/// Initialize all derived signals for config system
pub fn init_config_derived_signals() {
    // Force initialization of all derived signals
    let _ = &*OPENED_FILES_FOR_CONFIG;
    let _ = &*EXPANDED_SCOPES_FOR_CONFIG;
    let _ = &*EXPANDED_SCOPES_FOR_TREEVIEW;
    let _ = &*LOAD_FILES_EXPANDED_DIRECTORIES_FOR_CONFIG;
    let _ = &*SELECTED_VARIABLES_FOR_CONFIG;
    let _ = &*DOCK_MODE_FOR_CONFIG;
}

