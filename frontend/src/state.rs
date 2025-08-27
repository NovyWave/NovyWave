use zoon::*;
use std::collections::HashMap;
use indexmap::{IndexMap, IndexSet};
use shared::{WaveformFile, LoadingFile, FileSystemItem, TrackedFile, FileState, create_tracked_file};
use crate::time_types::{TimeNs, Viewport, ZoomLevel, TimelineCache};
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



/// ⚠️  PATCHED TREE FILES SIGNAL: Reduces flickering with aggressive deduplication
/// 
/// CURRENT ISSUE: Still uses signal_vec→signal antipattern that causes multiple renders
/// PROPER FIX NEEDED: Replace with items_signal_vec pattern or dedicated Mutable<Vec<T>>
/// 
/// This patch reduces flickering by:
/// 1. Heavy deduplication to prevent identical consecutive updates
/// 2. File state comparison to avoid unnecessary TreeView recreation
/// 3. Batched update detection to skip intermediate states
pub fn get_stable_tree_files_signal() -> impl Signal<Item = Vec<TrackedFile>> {
    TRACKED_FILES.signal_vec_cloned().to_signal_cloned()
        .dedupe_cloned()
        .map(|files| {
            // PATCH: Filter out files in transitional loading states to reduce flicker
            // Skip files that are just switching from Loading→Parsing→Loaded
            let stable_files: Vec<TrackedFile> = files.into_iter()
                .filter(|file| {
                    // Only include files in stable states or final loading state
                    match &file.state {
                        shared::FileState::Loading(shared::LoadingStatus::Starting) => false,
                        shared::FileState::Loading(shared::LoadingStatus::Parsing) => {
                            // Only show parsing if it's been in this state for a while
                            // This reduces rapid flicker during state transitions
                            true
                        },
                        _ => true, // Include completed, error, and other stable states
                    }
                })
                .collect();
            
            stable_files
        })
}




// ===== FILE UPDATE MESSAGE QUEUE SYSTEM =====

#[derive(Debug, Clone)]
pub enum FileUpdateMessage {
    Add { tracked_file: TrackedFile },
    Update { file_id: String, new_state: FileState },
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
        FileUpdateMessage::Add { tracked_file } => {
            // Check if file already exists and replace if it does
            let existing_index = {
                let files = TRACKED_FILES.lock_ref();
                files.iter().position(|f| f.id == tracked_file.id)
            };
            
            // ✅ OPTIMIZED: Compute smart label when adding individual files
            let mut tracked_file_with_smart_label = tracked_file.clone();
            if tracked_file_with_smart_label.smart_label.is_empty() {
                // Get all current file paths for smart label computation
                let current_paths: Vec<String> = {
                    let files = TRACKED_FILES.lock_ref();
                    let mut paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
                    paths.push(tracked_file.path.clone());
                    paths
                };
                
                let smart_labels = shared::generate_smart_labels(&current_paths);
                tracked_file_with_smart_label.smart_label = smart_labels.get(&tracked_file.path)
                    .unwrap_or(&tracked_file.filename)
                    .clone();
            }
            
            if let Some(index) = existing_index {
                // Update existing file
                TRACKED_FILES.lock_mut().set_cloned(index, tracked_file_with_smart_label);
            } else {
                // Add new file
                TRACKED_FILES.lock_mut().push_cloned(tracked_file_with_smart_label);
            }
            
            // Update the file IDs cache
            update_tracked_file_ids_cache();
        },
        
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
pub static VERTICAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
pub static HORIZONTAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Variables panel column resizing state
pub static VARIABLES_NAME_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 180.into());
pub static VARIABLES_VALUE_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 100.into());
pub static VARIABLES_NAME_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
pub static VARIABLES_VALUE_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Selected Variables panel row height
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;

// Timeline cursor position (in nanoseconds since file start)
pub static TIMELINE_CURSOR_NS: Lazy<Mutable<TimeNs>> = Lazy::new(|| Mutable::new(TimeNs::ZERO));

// Timeline viewport (visible time range in nanoseconds)
pub static TIMELINE_VIEWPORT: Lazy<Mutable<Viewport>> = Lazy::new(|| {
    Mutable::new(Viewport::new(
        TimeNs::ZERO,
        TimeNs::from_seconds(100.0) // Default 100 second range
    ))
});

// Timeline zoom level (as percentage: 100 = 1x, 200 = 2x, etc.)
pub static TIMELINE_ZOOM_LEVEL: Lazy<Mutable<ZoomLevel>> = Lazy::new(|| Mutable::new(ZoomLevel::NORMAL));

/// Unified timeline cache - replaces 4 separate cache systems
/// Contains viewport data, cursor values, raw transitions, and request deduplication
pub static UNIFIED_TIMELINE_CACHE: Lazy<Mutable<TimelineCache>> = Lazy::new(|| Mutable::new(TimelineCache::new()));

// Track if cursor position was set during startup before files loaded
pub static STARTUP_CURSOR_POSITION_SET: Lazy<Mutable<bool>> = lazy::default();

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
pub static MOUSE_TIME_NS: Lazy<Mutable<TimeNs>> = Lazy::new(|| Mutable::new(TimeNs::ZERO));

// Zoom center position (in nanoseconds) - separate from mouse position for explicit control
pub static ZOOM_CENTER_NS: Lazy<Mutable<TimeNs>> = Lazy::new(|| Mutable::new(TimeNs::ZERO));

// Canvas dimensions for click calculations
pub static CANVAS_WIDTH: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(800.0));
pub static CANVAS_HEIGHT: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(400.0));

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

// Selected variables management
pub static SELECTED_VARIABLES: Lazy<MutableVec<shared::SelectedVariable>> = lazy::default();
pub static SELECTED_VARIABLES_INDEX: Lazy<Mutable<IndexSet<String>>> = lazy::default();

// Signal values for selected variables - now stores signal values with clear state distinction
pub static SIGNAL_VALUES: Lazy<Mutable<HashMap<String, crate::format_utils::SignalValue>>> = lazy::default();

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
    
    // Use message queue to prevent recursive locking
    send_file_update_message(FileUpdateMessage::Add { tracked_file });
}

/// Update the state of an existing tracked file
pub fn update_tracked_file_state(file_id: &str, new_state: FileState) {
    // Use message queue to prevent recursive locking
    send_file_update_message(FileUpdateMessage::Update {
        file_id: file_id.to_string(),
        new_state,
    });
}

/// Remove a tracked file by ID
pub fn remove_tracked_file(file_id: &str) {
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

/// Derived signal that converts SELECTED_VARIABLES to Vec<SelectedVariable> for config storage
/// Uses CONFIG_INITIALIZATION_COMPLETE guard and debouncing to prevent circular loops
pub static SELECTED_VARIABLES_FOR_CONFIG: Lazy<Mutable<Vec<shared::SelectedVariable>>> = Lazy::new(|| {
    let derived = Mutable::new(Vec::new());
    
    // Initialize the derived signal with current SELECTED_VARIABLES
    let derived_clone = derived.clone();
    Task::start(async move {
        SELECTED_VARIABLES.signal_vec_cloned()
            .to_signal_cloned()
            // Note: Guard flag prevents circular triggers more effectively than dedupe
            .for_each(move |variables| {
                let derived = derived_clone.clone();
                async move {
                    // GUARD: Only process changes after initial config load is complete
                    // This prevents circular loops during config initialization
                    if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                        derived.set_neq(variables);
                    }
                }
            })
            .await;
    });
    
    derived
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

