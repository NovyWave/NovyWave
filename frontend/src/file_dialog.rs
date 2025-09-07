//! Simple file dialog state management using Atom pattern
//!
//! Replaces the enterprise DialogManager antipattern with simple Atom-based UI state.
//! Dialog visibility and file selection are local UI concerns, not complex domain logic.

use crate::dataflow::atom::Atom;
use zoon::{Signal, Lazy};

/// Simple dialog visibility state - replaces entire DialogManager enterprise antipattern
static FILE_DIALOG_VISIBLE: Lazy<Atom<bool>> = Lazy::new(|| Atom::new(false));

/// Simple file selection events - replaces complex manager relay system
static FILE_PICKER_SELECTED: Lazy<Atom<Vec<String>>> = Lazy::new(|| Atom::new(Vec::new()));

// File picker expanded directories and scroll position are handled by AppConfig
// See: config.file_picker_expanded_directories and file_picker_scroll_position

// === SIMPLE UI FUNCTIONS (replacing manager methods) ===

/// Open the file dialog - simple UI action
pub fn open_file_dialog() {
    FILE_DIALOG_VISIBLE.set(true);
}

/// Close the file dialog - simple UI action  
pub fn close_file_dialog() {
    FILE_DIALOG_VISIBLE.set(false);
}

/// Get dialog visibility signal - direct Atom access
pub fn dialog_visible_signal() -> impl Signal<Item = bool> {
    FILE_DIALOG_VISIBLE.signal()
}

/// Get file picker selection signal - direct Atom access
pub fn file_picker_selected_signal() -> impl Signal<Item = Vec<String>> {
    FILE_PICKER_SELECTED.signal()
}


/// Show file paths dialog with smart cache refresh - consolidated from file_utils.rs
pub fn show_file_paths_dialog() {
    // Open the dialog using simple UI state
    open_file_dialog();
    
    // SMART CACHE REFRESH - Request fresh data without clearing cache
    // This ensures users see newly added files without "Loading..." flicker
    // Fresh data will overwrite cached data when it arrives
    zoon::Task::start(async {
        use crate::platform::{Platform, CurrentPlatform};
        use shared::UpMsg;
        let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectory("/".to_string())).await;
        let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectory("~".to_string())).await;
    });
    
    zoon::Task::start(async {
        // Wait for dialog visible signal to be true (reactive coordination)  
        // Use signal-based coordination instead of arbitrary Timer::sleep()
        use futures::StreamExt;
        use zoon::SignalExt;
        let mut dialog_stream = dialog_visible_signal().to_stream();
        while let Some(is_visible) = dialog_stream.next().await {
            if is_visible {
                break; // Dialog is now visible, proceed with any additional setup
            }
        }
        
        // Dialog setup completed with reactive coordination
        // No complex manager needed for simple UI state
    });
}

// === MIGRATION COMPATIBILITY ===
// ✅ CLEANED UP: All legacy dialog_manager compatibility functions removed as they were unused

/// Simple file tree cache for dialog - replaces complex manager pattern
pub fn file_tree_cache_mutable() -> zoon::Mutable<std::collections::HashMap<String, Vec<shared::FileSystemItem>>> {
        // Simple UI state, not complex domain logic
    static FILE_TREE_CACHE: zoon::Lazy<zoon::Mutable<std::collections::HashMap<String, Vec<shared::FileSystemItem>>>> = 
        zoon::Lazy::new(|| zoon::Mutable::new(std::collections::HashMap::new()));
    FILE_TREE_CACHE.clone()
}