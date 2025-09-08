//! Simple file dialog state management using Atom pattern
//!
//! Replaces the enterprise DialogManager antipattern with simple Atom-based UI state.
//! Dialog visibility and file selection are local UI concerns, not complex domain logic.

use crate::dataflow::atom::Atom;
use zoon::{Signal, Lazy};
use futures::StreamExt;

/// File Dialog UI state - proper Actor+Relay for dialog management
#[derive(Clone)]
pub struct FileDialogState {
    pub visible: Atom<bool>,
    pub selected_files: Atom<Vec<String>>,
    pub dialog_opened_relay: crate::dataflow::Relay<()>,
    pub dialog_closed_relay: crate::dataflow::Relay<()>,
    pub files_selected_relay: crate::dataflow::Relay<Vec<String>>,
}

impl Default for FileDialogState {
    fn default() -> Self {
        let (dialog_opened_relay, mut dialog_opened_stream) = crate::dataflow::relay::<()>();
        let (dialog_closed_relay, mut dialog_closed_stream) = crate::dataflow::relay::<()>();
        let (files_selected_relay, mut files_selected_stream) = crate::dataflow::relay::<Vec<String>>();
        
        let visible = Atom::new(false);
        let selected_files = Atom::new(Vec::new());
        
        // Set up reactive handlers
        let visible_clone = visible.clone();
        let selected_files_clone = selected_files.clone();
        
        zoon::Task::start(async move {
            loop {
                futures::select! {
                    _ = dialog_opened_stream.next() => {
                        visible_clone.set(true);
                    }
                    _ = dialog_closed_stream.next() => {
                        visible_clone.set(false);
                    }
                    files = files_selected_stream.next() => {
                        if let Some(file_paths) = files {
                            selected_files_clone.set(file_paths);
                        }
                    }
                }
            }
        });
        
        Self {
            visible,
            selected_files,
            dialog_opened_relay,
            dialog_closed_relay,
            files_selected_relay,
        }
    }
}

// Static instance for compatibility during migration
static FILE_DIALOG_STATE_INSTANCE: Lazy<FileDialogState> = Lazy::new(|| FileDialogState::default());

/// Get the file dialog state instance
pub fn get_file_dialog_state() -> &'static FileDialogState {
    &FILE_DIALOG_STATE_INSTANCE
}

// === UI FUNCTIONS (using proper event-source relays) ===

/// Open the file dialog - triggers dialog_opened_relay
pub fn open_file_dialog() {
    get_file_dialog_state().dialog_opened_relay.send(());
}

/// Close the file dialog - triggers dialog_closed_relay
pub fn close_file_dialog() {
    get_file_dialog_state().dialog_closed_relay.send(());
}

/// Get dialog visibility signal - direct Atom access
pub fn dialog_visible_signal() -> impl Signal<Item = bool> {
    get_file_dialog_state().visible.signal()
}

/// Get file picker selection signal - direct Atom access
pub fn file_picker_selected_signal() -> impl Signal<Item = Vec<String>> {
    get_file_dialog_state().selected_files.signal()
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

/// File tree cache domain using proper Actor+Relay architecture
#[derive(Clone)]
pub struct FileTreeCache {
    pub cache: crate::dataflow::Actor<std::collections::HashMap<String, Vec<shared::FileSystemItem>>>,
    pub cache_updated_relay: crate::dataflow::Relay<(String, Vec<shared::FileSystemItem>)>,
}

impl FileTreeCache {
    pub async fn new() -> Self {
        let (cache_updated_relay, mut cache_updated_stream) = crate::dataflow::relay::<(String, Vec<shared::FileSystemItem>)>();
        
        let cache = crate::dataflow::Actor::new(std::collections::HashMap::new(), async move |state| {
            while let Some((path, items)) = cache_updated_stream.next().await {
                let mut cache_map = state.lock_mut();
                cache_map.insert(path, items);
            }
        });
        
        Self { cache, cache_updated_relay }
    }
    
    pub fn cache_signal(&self) -> impl zoon::Signal<Item = std::collections::HashMap<String, Vec<shared::FileSystemItem>>> {
        self.cache.signal()
    }
}

// Static instance for compatibility during migration
static FILE_TREE_CACHE_INSTANCE: zoon::Lazy<std::sync::Arc<std::sync::Mutex<Option<FileTreeCache>>>> = 
    zoon::Lazy::new(|| std::sync::Arc::new(std::sync::Mutex::new(None)));

/// Get or initialize the file tree cache instance
pub async fn get_file_tree_cache() -> FileTreeCache {
    let instance_arc = FILE_TREE_CACHE_INSTANCE.clone();
    let mut instance_guard = instance_arc.lock().unwrap();
    
    if instance_guard.is_none() {
        let cache = FileTreeCache::new().await;
        *instance_guard = Some(cache.clone());
        cache
    } else {
        instance_guard.as_ref().unwrap().clone()
    }
}