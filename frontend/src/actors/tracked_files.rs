//! TrackedFiles Actor+Relay Domain
//! 
//! Proper Actor+Relay architecture for file loading and management.
//! Uses dataflow Actor pattern instead of global Mutables.

use crate::dataflow::{Actor, ActorVec, Relay, relay};
use shared::{TrackedFile, FileState, LoadingStatus, FileFormat, create_tracked_file, generate_smart_labels};
use indexmap::IndexSet;
use futures::{StreamExt, select};
use zoon::*;

/// TrackedFiles domain with proper Actor+Relay architecture
#[derive(Clone)]
pub struct TrackedFiles {
    // Core state
    files: ActorVec<TrackedFile>,
    expanded_scopes: Actor<IndexSet<String>>,
    
    // Event-source relays
    pub config_files_loaded_relay: Relay<Vec<String>>,
    pub files_dropped_relay: Relay<Vec<std::path::PathBuf>>,
    pub file_removed_relay: Relay<String>,
    pub scope_toggled_relay: Relay<String>,
    pub file_load_completed_relay: Relay<(String, bool)>,
    pub all_files_cleared_relay: Relay<()>,
}

impl TrackedFiles {
    pub async fn new() -> Self {
        // Create relays
        let (config_files_loaded_relay, config_files_loaded_stream) = relay::<Vec<String>>();
        let (files_dropped_relay, files_dropped_stream) = relay::<Vec<std::path::PathBuf>>();
        let (file_removed_relay, file_removed_stream) = relay::<String>();
        let (scope_toggled_relay, scope_toggled_stream) = relay::<String>();
        let (file_load_completed_relay, file_load_completed_stream) = relay::<(String, bool)>();
        let (all_files_cleared_relay, all_files_cleared_stream) = relay::<()>();
        
        // Create files ActorVec with event processing
        let files = ActorVec::new(vec![], async move |files_handle| {
            let mut config_stream = config_files_loaded_stream.fuse();
            let mut dropped_stream = files_dropped_stream.fuse();
            let mut removed_stream = file_removed_stream.fuse();
            let mut completed_stream = file_load_completed_stream.fuse();
            let mut cleared_stream = all_files_cleared_stream.fuse();
            
            loop {
                select! {
                    config_files = config_stream.next() => {
                        if let Some(file_paths) = config_files {
                            zoon::println!("🔄 TrackedFiles: Config loaded {} files", file_paths.len());
                            
                            let tracked_files: Vec<TrackedFile> = file_paths.into_iter()
                                .map(|path_str| create_tracked_file(path_str, FileState::Loading(LoadingStatus::Starting)))
                                .collect();
                            
                            files_handle.lock_mut().replace_cloned(tracked_files);
                        }
                    }
                    dropped_files = dropped_stream.next() => {
                        if let Some(file_paths) = dropped_files {
                            zoon::println!("🔄 TrackedFiles: Files dropped: {:?}", file_paths);
                            
                            let new_files: Vec<TrackedFile> = file_paths.into_iter()
                                .map(|path| {
                                    let path_str = path.to_string_lossy().to_string();
                                    create_tracked_file(path_str, FileState::Loading(LoadingStatus::Starting))
                                })
                                .collect();
                            
                            for new_file in new_files {
                                // Don't add duplicates
                                let existing = files_handle.lock_ref().iter().any(|f| f.id == new_file.id);
                                if !existing {
                                    files_handle.lock_mut().push_cloned(new_file);
                                }
                            }
                        }
                    }
                    removed_file = removed_stream.next() => {
                        if let Some(file_id) = removed_file {
                            zoon::println!("🔄 TrackedFiles: File removed: {}", file_id);
                            files_handle.lock_mut().retain(|f| f.id != file_id);
                        }
                    }
                    completed = completed_stream.next() => {
                        if let Some((file_id, success)) = completed {
                            zoon::println!("🔄 TrackedFiles: File load completed: {} success={}", file_id, success);
                            
                            let new_state = if success { 
                                FileState::Loaded(shared::WaveformFile { 
                                    id: file_id.clone(), 
                                    filename: String::new(),
                                    format: FileFormat::VCD,
                                    scopes: Vec::new(),
                                    min_time_ns: None,
                                    max_time_ns: None,
                                }) 
                            } else { 
                                FileState::Loading(LoadingStatus::Error("Load failed".to_string())) 
                            };
                            
                            // Update the specific file's state by replacing the entire vector
                            let mut files = files_handle.lock_ref().to_vec();
                            if let Some(file) = files.iter_mut().find(|f| f.id == file_id) {
                                file.state = new_state;
                                files_handle.lock_mut().replace_cloned(files);
                            }
                        }
                    }
                    cleared = cleared_stream.next() => {
                        if cleared.is_some() {
                            zoon::println!("🔄 TrackedFiles: All files cleared");
                            files_handle.lock_mut().clear();
                        }
                    }
                    complete => break, // All streams ended
                }
            }
        });
        
        // Create expanded_scopes Actor
        let expanded_scopes = Actor::new(IndexSet::new(), async move |scopes_handle| {
            let mut scope_stream = scope_toggled_stream.fuse();
            
            loop {
                select! {
                    scope_id = scope_stream.next() => {
                        if let Some(scope_id) = scope_id {
                            zoon::println!("🔄 TrackedFiles: Scope toggled: {}", scope_id);
                            
                            let mut scopes = scopes_handle.lock_mut();
                            if scopes.contains(&scope_id) {
                                scopes.shift_remove(&scope_id);
                            } else {
                                scopes.insert(scope_id);
                            }
                        }
                    }
                    complete => break, // Stream ended
                }
            }
        });
        
        Self {
            files,
            expanded_scopes,
            config_files_loaded_relay,
            files_dropped_relay,
            file_removed_relay,
            scope_toggled_relay,
            file_load_completed_relay,
            all_files_cleared_relay,
        }
    }
    
    // ===== SIGNAL ACCESS =====
    
    /// Get signal for all tracked files
    pub fn files_signal(&self) -> impl Signal<Item = Vec<TrackedFile>> {
        self.files.signal_vec().to_signal_cloned()
    }
    
    /// Get signal vec for tracked files (for items_signal_vec usage)
    pub fn files_signal_vec(&self) -> impl SignalVec<Item = TrackedFile> {
        self.files.signal_vec()
    }
    
    /// Get signal for file count
    pub fn file_count_signal(&self) -> impl Signal<Item = usize> {
        self.files_signal().map(|files| {
            let count = files.len();
            zoon::println!("📊 TrackedFiles: Count signal returning: {}", count);
            count
        })
    }
    
    /// Get signal for expanded scopes
    pub fn expanded_scopes_signal(&self) -> impl Signal<Item = IndexSet<String>> {
        self.expanded_scopes.signal()
    }
    
    /// Get signal for smart labels (computed from files)
    pub fn smart_labels_signal(&self) -> impl Signal<Item = Vec<String>> {
        self.files_signal().map(|files| create_smart_labels(&files))
    }
    
    /// Get signal for whether files are loaded
    pub fn has_files_signal(&self) -> impl Signal<Item = bool> {
        self.files_signal().map(|files| !files.is_empty())
    }
    
    // ===== COMPATIBILITY METHODS =====
    
    /// Remove a file by ID (uses relay-based removal)
    pub fn remove_file(&self, file_id: String) {
        self.file_removed_relay.send(file_id);
    }
    
    // === REMOVED: get_all_file_paths() escape hatch method ===
    // This method broke Actor+Relay architecture by using .current_value()
    // 
    // Migration guide for code that used get_all_file_paths():
    // OLD: tracked_files_domain().get_all_file_paths()
    // NEW: Use tracked_files_signal() reactive pattern:
    //   tracked_files_signal().map(|files| files.iter().map(|f| f.path.clone()).collect())
    
    /// Batch load files (compatibility method - should use relays instead)
    pub fn batch_load_files(&self, file_paths: Vec<String>) {
        // Convert to PathBuf and emit through relay
        let path_bufs: Vec<std::path::PathBuf> = file_paths.into_iter()
            .map(std::path::PathBuf::from)
            .collect();
        self.files_dropped_relay.send(path_bufs);
    }
    
    /// Update file state (compatibility method - should use relays instead)
    pub fn update_file_state(&self, file_id: String, new_state: FileState) {
        // Convert FileState to success bool for the relay
        let success = matches!(new_state, FileState::Loaded(_));
        self.file_load_completed_relay.send((file_id, success));
    }
}

// ===== GLOBAL INSTANCE =====

static TRACKED_FILES_INSTANCE: std::sync::OnceLock<TrackedFiles> = std::sync::OnceLock::new();

/// Initialize TrackedFiles domain (call once at startup)
pub async fn initialize_tracked_files() -> TrackedFiles {
    let tracked_files = TrackedFiles::new().await;
    if TRACKED_FILES_INSTANCE.set(tracked_files.clone()).is_err() {
        zoon::println!("⚠️ TrackedFiles already initialized");
    }
    tracked_files
}

/// Get the global TrackedFiles instance
pub fn tracked_files_domain() -> Option<&'static TrackedFiles> {
    TRACKED_FILES_INSTANCE.get()
}

// ===== PUBLIC API FUNCTIONS =====

/// Initialize TrackedFiles domain from config data
pub fn initialize_from_config(config_file_paths: Vec<String>) {
    zoon::println!("🚀 TrackedFiles: Initializing with {} files from config", config_file_paths.len());
    
    if let Some(domain) = tracked_files_domain() {
        domain.config_files_loaded_relay.send(config_file_paths);
    } else {
        zoon::println!("⚠️ TrackedFiles domain not initialized - cannot load config files");
    }
}

/// Get signal for all tracked files (convenience function)
pub fn tracked_files_signal() -> impl Signal<Item = Vec<TrackedFile>> {
    if let Some(domain) = tracked_files_domain() {
        domain.files_signal().boxed_local()
    } else {
        zoon::always(Vec::new()).boxed_local()
    }
}

/// Get signal vec for tracked files (convenience function)
pub fn tracked_files_signal_vec() -> impl SignalVec<Item = TrackedFile> {
    if let Some(domain) = tracked_files_domain() {
        domain.files_signal_vec().boxed_local()
    } else {
        zoon::always(Vec::new()).to_signal_vec().boxed_local()
    }
}

/// Get signal for file count (convenience function)
pub fn tracked_files_count_signal() -> impl Signal<Item = usize> {
    if let Some(domain) = tracked_files_domain() {
        domain.file_count_signal().boxed_local()
    } else {
        zoon::always(0).boxed_local()
    }
}

/// Get signal for expanded scopes (convenience function)
pub fn expanded_scopes_signal() -> impl Signal<Item = IndexSet<String>> {
    if let Some(domain) = tracked_files_domain() {
        domain.expanded_scopes_signal().boxed_local()
    } else {
        zoon::always(IndexSet::new()).boxed_local()
    }
}

/// Create smart labels that disambiguate duplicate filenames
fn create_smart_labels(files: &[TrackedFile]) -> Vec<String> {
    if files.is_empty() {
        return Vec::new();
    }
    
    // Use the shared generate_smart_labels function
    let file_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    let labels_map = generate_smart_labels(&file_paths);
    
    // Return labels in the same order as files
    files.iter().map(|file| {
        labels_map.get(&file.path).cloned().unwrap_or_else(|| file.filename.clone())
    }).collect()
}