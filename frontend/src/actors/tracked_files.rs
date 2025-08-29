//! TrackedFiles domain for file management using Actor+Relay architecture
//!
//! Consolidated file management domain to replace global mutables with event-driven architecture.

use crate::actors::{Actor, ActorVec, Relay, relay};
use shared::{TrackedFile, FileState, LoadingStatus, FileError};
use zoon::{Task, SignalVecExt};
use indexmap::IndexSet;
use futures::{select, StreamExt};
use std::path::PathBuf;

/// Domain-driven file management with Actor+Relay architecture.
/// 
/// Replaces global mutables with cohesive event-driven state management.
#[derive(Clone, Debug)]
pub struct TrackedFiles {
    /// Core tracked files collection
    files: ActorVec<TrackedFile>,
    
    /// Loading state
    is_loading: Actor<bool>,
    
    /// Selected scope (integrated with files)
    selected_scope_id: Actor<Option<String>>,
    
    /// Expanded scopes
    expanded_scopes: Actor<IndexSet<String>>,
    
    // === USER FILE INTERACTION EVENTS ===
    /// User dropped files onto the application
    pub files_dropped_relay: Relay<Vec<PathBuf>>,
    
    /// User clicked a specific file in the file list
    pub file_selected_relay: Relay<String>,
    
    /// User removed a specific file from tracking
    pub file_removed_relay: Relay<String>,
    
    /// User cleared all tracked files
    pub all_files_cleared_relay: Relay<()>,
    
    // === SYSTEM FILE PROCESSING EVENTS ===
    /// File parsing started by the system
    pub parse_started_relay: Relay<(String, String)>,
    
    // === SCOPE MANAGEMENT EVENTS ===
    /// User expanded a scope in the file tree
    pub scope_expanded_relay: Relay<String>,
    
    /// User collapsed a scope in the file tree
    pub scope_collapsed_relay: Relay<String>,
    
    /// User selected a scope
    pub scope_selected_relay: Relay<String>,
    
    /// User cleared scope selection
    pub scope_selection_cleared_relay: Relay<()>,
}

impl TrackedFiles {
    /// Create a new TrackedFiles domain with event processors
    pub async fn new() -> Self {
        // Create relays for file operations
        let (files_dropped_relay, files_dropped_stream) = relay();
        let (file_selected_relay, _file_selected_stream) = relay();
        let (file_removed_relay, file_removed_stream) = relay();
        let (all_files_cleared_relay, all_files_cleared_stream) = relay();
        let (parse_started_relay, _parse_started_stream) = relay();
        
        // Create relays for scope operations
        let (scope_expanded_relay, scope_expanded_stream) = relay();
        let (scope_collapsed_relay, scope_collapsed_stream) = relay();
        let (scope_selected_relay, scope_selected_stream) = relay();
        let (scope_selection_cleared_relay, scope_selection_cleared_stream) = relay();
        
        // Create files actor with comprehensive event handling
        let files = ActorVec::new(vec![], {
            async move |files_mutable| {
                use futures::StreamExt;
                let mut files_dropped = files_dropped_stream.fuse();
                let mut file_removed = file_removed_stream.fuse();
                let mut all_files_cleared = all_files_cleared_stream.fuse();
                
                loop {
                    futures::select! {
                        paths_opt = files_dropped.next() => {
                            match paths_opt {
                                Some(paths) => Self::handle_files_dropped(&files_mutable, paths),
                                None => break,
                            }
                        },
                        file_id_opt = file_removed.next() => {
                            match file_id_opt {
                                Some(file_id) => Self::handle_file_removed(&files_handle, file_id),
                                None => break,
                            }
                        },
                        cleared_opt = all_files_cleared.next() => {
                            match cleared_opt {
                                Some(()) => Self::handle_all_files_cleared(&files_handle),
                                None => break,
                            }
                        },
                    }
                }
            }
        });
        
        // Create simple loading state actor
        let is_loading = Actor::new(false, async move |_loading_handle| {
            // Simple loading state - will be enhanced later
            loop {
                futures::future::pending::<()>().await;
            }
        });
        
        // Create scope selection actor
        let selected_scope_id = Actor::new(None, async move |scope_handle| {
            let mut scope_selected = scope_selected_stream;
            
            while let Some(scope_id) = scope_selected.next().await {
                scope_handle.set(Some(scope_id));
            }
        });
        
        // Create expanded scopes actor
        let expanded_scopes = Actor::new(IndexSet::new(), async move |scopes_handle| {
            let mut scope_expanded = scope_expanded_stream;
            
            while let Some(scope_id) = scope_expanded.next().await {
                scopes_handle.update(|scopes| { scopes.insert(scope_id); });
            }
        });
        
        Self {
            files,
            is_loading,
            selected_scope_id,
            expanded_scopes,
            
            files_dropped_relay,
            file_selected_relay,
            file_removed_relay,
            all_files_cleared_relay,
            parse_started_relay,
            
            scope_expanded_relay,
            scope_collapsed_relay,
            scope_selected_relay,
            scope_selection_cleared_relay,
        }
    }
    
    // === REACTIVE SIGNAL ACCESS ===
    
    /// Get reactive signal for all tracked files
    pub fn files_signal(&self) -> impl zoon::Signal<Item = Vec<TrackedFile>> {
        self.files.signal_vec().to_signal_cloned()
    }
    
    /// Get reactive signal for files as signal vec (VecDiff updates)  
    pub fn files_signal_vec(&self) -> impl zoon::SignalVec<Item = TrackedFile> {
        self.files.signal_vec()
    }
    
    /// Get reactive signal for loading state
    pub fn is_loading_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.is_loading.signal()
    }
    
    /// Get reactive signal for selected scope ID
    pub fn selected_scope_signal(&self) -> impl zoon::Signal<Item = Option<String>> {
        self.selected_scope_id.signal()
    }
    
    /// Get reactive signal for expanded scopes
    pub fn expanded_scopes_signal(&self) -> impl zoon::Signal<Item = IndexSet<String>> {
        self.expanded_scopes.signal()
    }
    
    /// Get file count signal
    pub fn file_count_signal(&self) -> impl zoon::Signal<Item = usize> {
        self.files.signal_ref(|files| files.len())
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

impl TrackedFiles {
    /// Handle files dropped by user onto application
    fn handle_files_dropped(
        files: &zoon::MutableVec<TrackedFile>,
        paths: Vec<PathBuf>
    ) {
        for path in paths {
            // Convert path to string for ID generation
            let path_str = path.to_string_lossy().to_string();
            
            // Generate deterministic file ID based on full path
            let file_id = shared::generate_file_id(&path_str);
            
            let filename = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            // Create tracked file with basic structure
            let tracked_file = TrackedFile {
                id: file_id,
                path: path_str,
                filename: filename.clone(),
                state: FileState::Loading(LoadingStatus::Starting),
                smart_label: filename,
            };
            
            // Add to collection
            files.lock_mut().push_cloned(tracked_file);
        }
    }
    
    /// Handle removing specific file
    fn handle_file_removed(
        files_handle: &ActorVecHandle<TrackedFile>,
        file_id: String
    ) {
        files_handle.retain(|file| file.id != file_id);
    }
    
    /// Handle clearing all files
    fn handle_all_files_cleared(
        files_handle: &ActorVecHandle<TrackedFile>
    ) {
        files_handle.clear();
    }
}

// === CONVENIENCE FUNCTIONS FOR UI INTEGRATION ===

/// Global TrackedFiles instance
static TRACKED_FILES_INSTANCE: std::sync::OnceLock<TrackedFiles> = std::sync::OnceLock::new();

/// Initialize the TrackedFiles domain (call once on app startup)
pub async fn initialize_tracked_files() -> TrackedFiles {
    let tracked_files = TrackedFiles::new().await;
    TRACKED_FILES_INSTANCE.set(tracked_files.clone())
        .expect("TrackedFiles already initialized - initialize_tracked_files() should only be called once");
    tracked_files
}

/// Get the global TrackedFiles instance
pub fn get_tracked_files() -> TrackedFiles {
    TRACKED_FILES_INSTANCE.get()
        .expect("TrackedFiles not initialized - call initialize_tracked_files() first")
        .clone()
}