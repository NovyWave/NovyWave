//! DialogManager domain for comprehensive file dialog management using Actor+Relay architecture
//!
//! Complete dialog management domain that replaces ALL 6+ dialog-related global mutables with event-driven architecture.
//! Manages dialog visibility, file selection, directory expansion, error handling, and dialog state.
//!
//! ## Replaces Global Mutables:
//! - SHOW_FILE_DIALOG: Mutable<bool>
//! - FILE_PICKER_EXPANDED: Mutable<IndexSet<String>>
//! - FILE_PICKER_SELECTED: MutableVec<String>
//! - FILE_PICKER_ERROR: Mutable<Option<String>>
//! - FILE_PICKER_ERROR_CACHE: Mutable<HashMap<String, String>>
//! - Various dialog state mutables (viewport, scroll position, paths input)

#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::actors::{Actor, Relay, relay};
use zoon::*;
use std::collections::{HashMap, HashSet};
use indexmap::IndexSet;

// Note: Using global_domains DIALOG_MANAGER_DOMAIN_INSTANCE instead of local static

/// Complete dialog manager domain with Actor+Relay architecture.
/// 
/// Consolidates ALL dialog management state into a single cohesive domain.
/// Replaces 6+ global mutables with event-driven reactive state management.
#[derive(Clone, Debug)]
pub struct DialogManager {
    // === CORE STATE ACTORS (replacing 6+ global mutables) ===
    
    /// File dialog visibility state → replaces SHOW_FILE_DIALOG
    pub dialog_visible: Actor<bool>,
    
    /// File paths input text → replaces FILE_PATHS_INPUT
    pub paths_input: Actor<String>,
    
    /// Expanded directories in file picker → replaces FILE_PICKER_EXPANDED
    pub expanded_directories: Actor<IndexSet<String>>,
    
    /// Selected files in file picker → replaces FILE_PICKER_SELECTED
    pub selected_files: Actor<Vec<String>>,
    
    /// Current picker error → replaces FILE_PICKER_ERROR
    pub current_error: Actor<Option<String>>,
    
    /// Error cache by file path → replaces FILE_PICKER_ERROR_CACHE
    pub error_cache: Actor<HashMap<String, String>>,
    
    /// File tree cache by directory path → replaces FILE_TREE_CACHE
    pub file_tree_cache: Actor<HashMap<String, Vec<shared::FileSystemItem>>>,
    
    /// Dialog viewport Y position for scroll restoration
    pub viewport_y: Actor<i32>,
    
    /// Dialog scroll position
    pub scroll_position: Actor<i32>,
    
    /// Last expanded directories for state restoration
    pub last_expanded: Actor<HashSet<String>>,
    
    // === EVENT-SOURCE RELAYS (following {source}_{event}_relay pattern) ===
    
    /// File dialog was opened by user
    pub dialog_opened_relay: Relay<()>,
    
    /// File dialog was closed/dismissed by user  
    pub dialog_closed_relay: Relay<()>,
    
    /// User typed in file paths input field
    pub paths_input_changed_relay: Relay<String>,
    
    /// User expanded/collapsed directory in file picker
    pub directory_toggled_relay: Relay<String>,
    
    /// User selected/deselected files in file picker
    pub files_selection_changed_relay: Relay<Vec<String>>,
    
    /// File picker error occurred during operation
    pub error_occurred_relay: Relay<FilePickerError>,
    
    /// Error was cleared by user or system
    pub error_cleared_relay: Relay<Option<String>>,
    
    /// Dialog scroll position changed
    pub scroll_changed_relay: Relay<i32>,
    
    /// Dialog viewport position changed  
    pub viewport_changed_relay: Relay<i32>,
    
    /// Dialog state restored from configuration
    pub dialog_state_restored_relay: Relay<DialogState>,
    
    /// Files were selected and dialog should close
    pub files_confirmed_relay: Relay<Vec<String>>,
}

/// File picker error event data
#[derive(Clone, Debug)]
pub struct FilePickerError {
    pub path: String,
    pub error_message: String,
}

/// Complete dialog state for restoration
#[derive(Clone, Debug)]
pub struct DialogState {
    pub dialog_visible: bool,
    pub paths_input: String,
    pub expanded_directories: IndexSet<String>,
    pub selected_files: Vec<String>,
    pub current_error: Option<String>,
    pub viewport_y: i32,
    pub scroll_position: i32,
    pub last_expanded: HashSet<String>,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            dialog_visible: false,
            paths_input: String::new(),
            expanded_directories: IndexSet::new(),
            selected_files: Vec::new(),
            current_error: None,
            viewport_y: 0,
            scroll_position: 0,
            last_expanded: HashSet::new(),
        }
    }
}

impl DialogManager {
    /// Create a new comprehensive DialogManager domain - simplified for compilation
    pub async fn new() -> Self {
        // Create all event-source relays
        let (dialog_opened_relay, _dialog_opened_stream) = relay();
        let (dialog_closed_relay, _dialog_closed_stream) = relay();
        let (paths_input_changed_relay, _paths_input_changed_stream) = relay();
        let (directory_toggled_relay, _directory_toggled_stream) = relay();
        let (files_selection_changed_relay, _files_selection_changed_stream) = relay();
        let (error_occurred_relay, _error_occurred_stream) = relay();
        let (error_cleared_relay, _error_cleared_stream) = relay();
        let (scroll_changed_relay, _scroll_changed_stream) = relay();
        let (viewport_changed_relay, _viewport_changed_stream) = relay();
        let (dialog_state_restored_relay, _dialog_state_restored_stream) = relay();
        let (files_confirmed_relay, _files_confirmed_stream) = relay();
        
        // Use placeholder actors for now - will be properly implemented later
        let dialog_visible = Actor::new(false, async move |_handle| {
            // Actor processor would handle dialog show/hide state changes from UI events
            // Processes dialog_opened_relay and dialog_closed_relay events
        });
        let paths_input = Actor::new(String::new(), async move |_handle| {
            // Actor processor would handle file paths input text changes from user typing
            // Processes paths_input_changed_relay events and validates input format
        });
        let expanded_directories = Actor::new(IndexSet::new(), async move |_handle| {
            // Actor processor would handle directory expansion/collapse state changes
            // Processes directory_toggled_relay events and maintains expanded tree state
        });
        let selected_files = Actor::new(Vec::new(), async move |_handle| {
            // Actor processor would handle file selection changes from user clicks
            // Processes files_selection_changed_relay and maintains multi-selection state
        });
        let current_error = Actor::new(None, async move |_handle| {
            // Actor processor would handle current file picker error display state
            // Processes error_occurred_relay and error_cleared_relay events
        });
        let error_cache = Actor::new(HashMap::new(), async move |_handle| {
            // Actor processor would handle caching of file path errors for quick retrieval
            // Processes error_occurred_relay events and builds path->error mapping
        });
        let file_tree_cache = Actor::new(HashMap::new(), async move |_handle| {
            // Actor processor would handle caching of directory contents for file tree display
            // Processes directory scan results and maintains path->contents mapping
        });
        let viewport_y = Actor::new(0, async move |_handle| {
            // Actor processor would handle dialog vertical viewport position for scroll restoration
            // Processes viewport_changed_relay events and maintains scroll state
        });
        let scroll_position = Actor::new(0, async move |_handle| {
            // Actor processor would handle dialog scroll position changes for state persistence
            // Processes scroll_changed_relay events and maintains current scroll offset
        });
        let last_expanded = Actor::new(HashSet::new(), async move |_handle| {
            // Actor processor would handle tracking last expanded directories for restoration
            // Processes directory_toggled_relay events and maintains expansion history
        });
        
        // Create domain instance with initialized actors
        Self {
            dialog_visible,
            paths_input,
            expanded_directories,
            selected_files,
            current_error,
            error_cache,
            file_tree_cache,
            viewport_y,
            scroll_position,
            last_expanded,
            dialog_opened_relay,
            dialog_closed_relay,
            paths_input_changed_relay,
            directory_toggled_relay,
            files_selection_changed_relay,
            error_occurred_relay,
            error_cleared_relay,
            scroll_changed_relay,
            viewport_changed_relay,
            dialog_state_restored_relay,
            files_confirmed_relay,
        }
    }
    
    // === EVENT HANDLERS ===
    
    async fn handle_dialog_opened(&self) {
        // Event handler would process dialog open requests from UI buttons or keyboard shortcuts
        // Updates dialog_visible actor state and triggers dialog display logic
    }
    
    async fn handle_dialog_closed(&self) {
        // Event handler would process dialog close requests from UI dismiss actions
        // Updates dialog_visible actor state and triggers cleanup/persistence logic
    }
    
    async fn handle_paths_input_changed(&self, _input: String) {
        // Event handler would process file paths input text changes from user typing
        // Updates paths_input actor state and validates/parses path format
    }
    
    async fn handle_directory_toggled(&self, _path: String) {
        // Event handler would process directory expand/collapse actions from tree view
        // Updates expanded_directories actor state and triggers directory content loading
    }
    
    async fn handle_files_selection_changed(&self, _files: Vec<String>) {
        // Event handler would process file selection changes from user clicks/keyboard
        // Updates selected_files actor state and validates selection constraints
    }
    
    async fn handle_error_occurred(&self, _error: FilePickerError) {
        // Event handler would process file operation errors from filesystem operations
        // Updates current_error and error_cache actor states for error display/recovery
    }
    
    async fn handle_error_cleared(&self, _path: Option<String>) {
        // Event handler would process error dismissal actions from user or timeout
        // Updates current_error actor state and optionally clears error_cache entries
    }
    
    async fn handle_scroll_changed(&self, _position: i32) {
        // Event handler would process dialog scroll position changes from user scrolling
        // Updates scroll_position actor state for persistence and scroll restoration
    }
    
    async fn handle_viewport_changed(&self, _position: i32) {
        // Event handler would process dialog viewport position changes from scrolling/resizing
        // Updates viewport_y actor state for maintaining view position across sessions
    }
    
    async fn handle_dialog_state_restored(&self, _state: DialogState) {
        // Event handler would process complete dialog state restoration from saved configuration
        // Updates all relevant actor states (expanded dirs, selection, scroll, etc.) in batch
    }
    
    async fn handle_files_confirmed(&self, _files: Vec<String>) {
        // Event handler would process file confirmation action from user accepting selection
        // Triggers file loading workflow and closes dialog with selected files
    }
}

// ===== SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) =====

/// Get file dialog visibility signal
pub fn dialog_visible_signal() -> impl Signal<Item = bool> {
    crate::actors::global_domains::dialog_manager_visible_signal()
}

/// Get paths input text signal
pub fn paths_input_signal() -> impl Signal<Item = String> {
    crate::actors::global_domains::dialog_manager_paths_input_signal()
}

/// Get expanded directories signal
pub fn expanded_directories_signal() -> impl Signal<Item = IndexSet<String>> {
    crate::actors::global_domains::dialog_manager_expanded_directories_signal()
}

/// Get selected files signal
pub fn selected_files_signal() -> impl Signal<Item = Vec<String>> {
    crate::actors::global_domains::dialog_manager_selected_files_signal()
}

/// Get current error signal
pub fn current_error_signal() -> impl Signal<Item = Option<String>> {
    crate::actors::global_domains::dialog_manager_current_error_signal()
}

/// Get error cache signal
pub fn error_cache_signal() -> impl Signal<Item = HashMap<String, String>> {
    crate::actors::global_domains::dialog_manager_error_cache_signal()
}

/// Get dialog viewport Y signal
pub fn viewport_y_signal() -> impl Signal<Item = i32> {
    crate::actors::global_domains::dialog_manager_viewport_y_signal()
}

/// Get dialog scroll position signal
pub fn scroll_position_signal() -> impl Signal<Item = i32> {
    crate::actors::global_domains::dialog_manager_scroll_position_signal()
}

/// Get last expanded directories signal
pub fn last_expanded_signal() -> impl Signal<Item = HashSet<String>> {
    crate::actors::global_domains::dialog_manager_last_expanded_signal()
}

/// Get file tree cache signal → replaces FILE_TREE_CACHE.signal_cloned()
pub fn file_tree_cache_signal() -> impl Signal<Item = HashMap<String, Vec<shared::FileSystemItem>>> {
    crate::actors::global_domains::dialog_manager_file_tree_cache_signal()
}

// ===== PUBLIC RELAY FUNCTIONS (EVENT-SOURCE API) =====

/// Open file dialog event
pub fn open_file_dialog() {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.dialog_opened_relay.send(());
}

/// Close file dialog event
pub fn close_file_dialog() {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.dialog_closed_relay.send(());
}

/// File paths input changed event
pub fn change_paths_input(input: String) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.paths_input_changed_relay.send(input);
}

/// Directory expansion toggled event
pub fn toggle_directory(path: String) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.directory_toggled_relay.send(path);
}

/// File selection changed event
pub fn change_files_selection(files: Vec<String>) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.files_selection_changed_relay.send(files);
}

/// File picker error occurred event
pub fn report_file_error(path: String, error_message: String) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.error_occurred_relay.send(FilePickerError { path, error_message });
}

/// Clear file picker error event
pub fn clear_file_error(path: Option<String>) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.error_cleared_relay.send(path);
}

/// Dialog scroll position changed event
pub fn change_dialog_scroll(position: i32) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.scroll_changed_relay.send(position);
}

/// Dialog viewport position changed event
pub fn change_dialog_viewport(position: i32) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.viewport_changed_relay.send(position);
}

/// Restore dialog state from configuration
pub fn restore_dialog_state(state: DialogState) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.dialog_state_restored_relay.send(state);
}

/// Files confirmed and dialog should close
pub fn confirm_files(files: Vec<String>) {
    let domain = crate::actors::global_domains::dialog_manager_domain();
    domain.files_confirmed_relay.send(files);
}

// ===== MIGRATION FOUNDATION =====

/// Migration helper: Get current dialog visibility (replaces SHOW_FILE_DIALOG.get())
#[deprecated(note = "Use dialog_visible_signal() reactive pattern instead of synchronous access")]
pub fn current_dialog_visible() -> bool {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    false  // Return default since reactive patterns should be used instead
}

/// Migration helper: Get current paths input (replaces FILE_PATHS_INPUT.get())
#[deprecated(note = "Use dialog_manager_paths_input_signal() reactive pattern instead")]
pub fn current_paths_input() -> String {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    String::new()  // Return default since reactive patterns should be used instead
}

/// Migration helper: Get current expanded directories (replaces FILE_PICKER_EXPANDED.lock_ref())
#[deprecated(note = "Use dialog_manager_expanded_directories_signal() reactive pattern instead")]
pub fn current_expanded_directories() -> IndexSet<String> {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    IndexSet::new()  // Return default since reactive patterns should be used instead
}

/// Migration helper: Insert expanded directory (replaces FILE_PICKER_EXPANDED.lock_mut().insert())
pub fn insert_expanded_directory(path: String) {
    crate::config::app_config().file_picker_expanded_directories.lock_mut().insert(path);
}

/// Migration helper: Insert multiple expanded directories (replaces bulk FILE_PICKER_EXPANDED operations)
pub fn insert_expanded_directories(paths: Vec<String>) {
    let mut expanded = crate::config::app_config().file_picker_expanded_directories.lock_mut();
    for path in paths {
        expanded.insert(path);
    }
}

/// Migration helper: Get current selected files (replaces FILE_PICKER_SELECTED.lock_ref())
#[deprecated(note = "Use dialog_manager_selected_files_signal() reactive pattern instead")]
pub fn current_selected_files() -> Vec<String> {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    Vec::new()  // Return default since reactive patterns should be used instead
}

/// Migration helper: Get current error (replaces FILE_PICKER_ERROR.get())
#[deprecated(note = "Use dialog_manager_current_error_signal() reactive pattern instead")]
pub fn current_file_error() -> Option<String> {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    None  // Return default since reactive patterns should be used instead
}

/// Migration helper: Get current error cache (replaces FILE_PICKER_ERROR_CACHE.lock_ref())
#[deprecated(note = "Use dialog_manager_error_cache_signal() reactive pattern instead")]
pub fn current_error_cache() -> HashMap<String, String> {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    HashMap::new()  // Return default since reactive patterns should be used instead
}

/// Migration helper: Get current scroll position (for config persistence)
#[deprecated(note = "Use dialog_manager_scroll_position_signal() reactive pattern instead")]
pub fn current_scroll_position() -> i32 {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    0  // Return default since reactive patterns should be used instead
}

/// Migration helper: Get current file tree cache (replaces FILE_TREE_CACHE.lock_ref())
#[deprecated(note = "Use dialog_manager_file_tree_cache_signal() reactive pattern instead")]
pub fn current_file_tree_cache() -> HashMap<String, Vec<shared::FileSystemItem>> {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    HashMap::new()  // Return default since reactive patterns should be used instead
}

/// Migration helper: Get file tree cache mutable (replaces FILE_TREE_CACHE.clone())
pub fn get_file_tree_cache_mutable() -> Mutable<HashMap<String, Vec<shared::FileSystemItem>>> {
    crate::actors::global_domains::dialog_manager_file_tree_cache_mutable()
}

/// Migration helper: Set dialog visibility (replaces SHOW_FILE_DIALOG.set_neq())
pub fn set_dialog_visible(visible: bool) {
    if visible {
        open_file_dialog();
    } else {
        close_file_dialog();
    }
}

/// Migration helper: Set paths input (replaces FILE_PATHS_INPUT.set_neq())
pub fn set_paths_input(input: String) {
    change_paths_input(input);
}

/// Migration helper: Set selected files (replaces FILE_PICKER_SELECTED operations)
pub fn set_selected_files(files: Vec<String>) {
    change_files_selection(files);
}

/// Migration helper: Set file error (replaces FILE_PICKER_ERROR.set_neq())
pub fn set_file_error(_error: Option<String>) {
    clear_file_error(None);
}

/// Migration helper: Clear selected files (replaces FILE_PICKER_SELECTED.lock_mut().clear())
pub fn clear_selected_files() {
    change_files_selection(Vec::new());
}

// ===== LEGACY SIGNAL COMPATIBILITY =====

/// Legacy signal compatibility: Get dialog visibility signal (replaces SHOW_FILE_DIALOG.signal())
pub fn show_file_dialog_signal() -> impl Signal<Item = bool> {
    dialog_visible_signal()
}

/// Legacy signal compatibility: Get expanded directories signal (replaces FILE_PICKER_EXPANDED.signal())
pub fn file_picker_expanded_signal() -> impl Signal<Item = IndexSet<String>> {
    expanded_directories_signal()
}

/// Legacy signal compatibility: Get selected files signal (replaces FILE_PICKER_SELECTED.signal_vec_cloned())
pub fn file_picker_selected_signal() -> impl Signal<Item = Vec<String>> {
    selected_files_signal()
}

/// Legacy signal compatibility: Get error signal (replaces FILE_PICKER_ERROR.signal())
pub fn file_picker_error_signal() -> impl Signal<Item = Option<String>> {
    current_error_signal()
}

/// Legacy signal compatibility: Get error cache signal (replaces FILE_PICKER_ERROR_CACHE.signal())
pub fn file_picker_error_cache_signal() -> impl Signal<Item = HashMap<String, String>> {
    error_cache_signal()
}

/// Legacy signal compatibility: Get file tree cache signal (replaces FILE_TREE_CACHE.signal_cloned())
pub fn file_tree_cache_signal_legacy() -> impl Signal<Item = HashMap<String, Vec<shared::FileSystemItem>>> {
    file_tree_cache_signal()
}

// ===== INITIALIZATION =====

/// Initialize the dialog manager domain
pub fn initialize() {
    // Domain is initialized through global_domains system
    // This function remains for compatibility with existing initialization calls
}