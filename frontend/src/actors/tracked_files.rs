//! TrackedFiles Actor+Relay Domain
//! 
//! Proper Actor+Relay architecture for file loading and management.
//! Uses dataflow Actor pattern instead of global Mutables.

use crate::dataflow::{ActorVec, Relay, relay};
use shared::{TrackedFile, FileState, LoadingStatus, create_tracked_file};
use futures::{StreamExt, select};
use zoon::*;

/// TrackedFiles domain with proper Actor+Relay architecture
#[derive(Clone)]
pub struct TrackedFiles {
    // Core state
    files: ActorVec<TrackedFile>,
    files_vec_signal: zoon::Mutable<Vec<TrackedFile>>,  // Dedicated signal for efficient Vec access
    
    // Event-source relays
    pub config_files_loaded_relay: Relay<Vec<String>>,
    pub files_dropped_relay: Relay<Vec<std::path::PathBuf>>,
    pub file_removed_relay: Relay<String>,
    pub file_reload_requested_relay: Relay<String>,
    pub file_load_completed_relay: Relay<(String, FileState)>,
    pub parsing_progress_relay: Relay<(String, f32, LoadingStatus)>,
    pub loading_started_relay: Relay<(String, String)>, // (file_id, filename)
    pub all_files_cleared_relay: Relay<()>,
}

impl TrackedFiles {
    pub async fn new() -> Self {
        // Create relays
        let (config_files_loaded_relay, mut config_files_loaded_stream) = relay::<Vec<String>>();
        let (files_dropped_relay, mut files_dropped_stream) = relay::<Vec<std::path::PathBuf>>();
        let (file_removed_relay, mut file_removed_stream) = relay::<String>();
        let (file_reload_requested_relay, mut file_reload_requested_stream) = relay::<String>();
        let (file_load_completed_relay, mut file_load_completed_stream) = relay::<(String, FileState)>();
        let (parsing_progress_relay, mut parsing_progress_stream) = relay::<(String, f32, LoadingStatus)>();
        let (loading_started_relay, mut loading_started_stream) = relay::<(String, String)>();
        let (all_files_cleared_relay, mut all_files_cleared_stream) = relay::<()>();
        
        // Create dedicated Vec signal that syncs with ActorVec changes (no conversion antipattern)
        let files_vec_signal = zoon::Mutable::new(vec![]);
        
        // Create files ActorVec with event processing including business logic
        let files = ActorVec::new(vec![], {
            let files_vec_signal_sync = files_vec_signal.clone();
            async move |files_handle| {
            
            // ✅ Cache Current Values pattern - maintain state for business logic
            let mut cached_loading_states: std::collections::HashMap<String, FileState> = std::collections::HashMap::new();
            let mut all_files_loaded_signaled = false;
            
            loop {
                select! {
                    config_files = config_files_loaded_stream.next() => {
                        if let Some(file_paths) = config_files {
                            // TrackedFiles: Config loaded files
                            
                            let tracked_files: Vec<TrackedFile> = file_paths.into_iter()
                                .map(|path_str| create_tracked_file(path_str, FileState::Loading(LoadingStatus::Starting)))
                                .collect();
                            
                            // Trigger parsing for each file
                            for file in &tracked_files {
                                // TrackedFiles: Triggering parse for file
                                trigger_file_parsing(file.path.clone());
                            }
                            
                            files_handle.lock_mut().replace_cloned(tracked_files);
                            
                            // Sync dedicated Vec signal after ActorVec change
                            {
                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
                            }
                        }
                    }
                    dropped_files = files_dropped_stream.next() => {
                        if let Some(file_paths) = dropped_files {
                            
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
                                    // TrackedFiles: Triggering parse for dropped file
                                    trigger_file_parsing(new_file.path.clone());
                                    files_handle.lock_mut().push_cloned(new_file);
                                    
                                    // Sync dedicated Vec signal after ActorVec change
                                    {
                                        let current_files = files_handle.lock_ref().to_vec();
                                        files_vec_signal_sync.set_neq(current_files);
                                    }
                                }
                            }
                        }
                    }
                    removed_file = file_removed_stream.next() => {
                        if let Some(file_id) = removed_file {
                            files_handle.lock_mut().retain(|f| f.id != file_id);
                            
                            // Sync dedicated Vec signal after ActorVec change
                            {
                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
                            }
                        }
                    }
                    reload_requested = file_reload_requested_stream.next() => {
                        if let Some(file_id) = reload_requested {
                            
                            // Find the existing file and perform atomic reload operation
                            {
                                let existing_file = files_handle.lock_ref().iter()
                                    .find(|f| f.id == file_id).cloned();
                                
                                if let Some(existing_file) = existing_file {
                                    
                                    // Create new file with Starting state to trigger full parsing
                                    let new_file = create_tracked_file(
                                        existing_file.path.clone(), 
                                        FileState::Loading(LoadingStatus::Starting)
                                    );
                                    
                                    // CRITICAL: Trigger parsing BEFORE modifying collection to avoid duplicate detection
                                    trigger_file_parsing(new_file.path.clone());
                                    
                                    // ATOMIC: Single lock operation - remove old, add new  
                                    {
                                        let mut files = files_handle.lock_mut();
                                        files.retain(|f| f.id != file_id);
                                        files.push_cloned(new_file);
                                    } // Lock released here
                                    
                                    // Sync dedicated Vec signal after ActorVec change
                                    {
                                        let current_files = files_handle.lock_ref().to_vec();
                                        files_vec_signal_sync.set_neq(current_files);
                                    }
                                } else {
                                }
                            } // Read lock scope ends here
                        }
                    }
                    completed = file_load_completed_stream.next() => {
                        if let Some((file_id, new_state)) = completed {
                            // ✅ Cache Current Values: Update cached loading states for business logic
                            cached_loading_states.insert(file_id.clone(), new_state.clone());
                            
                            // Update the specific file's state by replacing the entire vector
                            let mut files = files_handle.lock_ref().to_vec();
                            // Update file state in collection
                            // Found files list (debug info omitted for performance)
                            if let Some(file) = files.iter_mut().find(|f| f.id == file_id) {
                                // File state updated successfully
                                file.state = new_state;
                                files_handle.lock_mut().replace_cloned(files);
                                
                                // Sync dedicated Vec signal after ActorVec change
                                {
                                    let current_files = files_handle.lock_ref().to_vec();
                                    files_vec_signal_sync.set_neq(current_files);
                                }
                                
                                // ✅ BUSINESS LOGIC: Check if all files are loaded using cached values
                                let current_files = files_handle.lock_ref();
                                let all_done = current_files.iter().all(|f| {
                                    matches!(f.state, shared::FileState::Loaded(_) | shared::FileState::Failed(_))
                                });
                                
                                if all_done && !all_files_loaded_signaled {
                                    all_files_loaded_signaled = true;
                                    
                                    // ✅ BUSINESS LOGIC: Trigger scope restoration and value queries
                                    zoon::Task::start({
                                        async move {
                                            // Restore scope selections using SelectedVariables Actor method
                                            crate::actors::selected_variables::SelectedVariables::restore_scope_selections_reactive().await;
                                            
                                            // Trigger signal value queries after loading completes
                                            crate::views::trigger_signal_value_queries();
                                        }
                                    });
                                }
                            } else {
                            }
                        }
                    }
                    parsing_progress = parsing_progress_stream.next() => {
                        if let Some((file_id, _progress, status)) = parsing_progress {
                            // Update existing file's loading state by recreating the files vector
                            let current_files = files_handle.lock_ref().to_vec();
                            let updated_files: Vec<TrackedFile> = current_files.into_iter().map(|mut file| {
                                if file.id == file_id {
                                    file.state = FileState::Loading(status.clone());
                                }
                                file
                            }).collect();
                            
                            files_handle.lock_mut().replace_cloned(updated_files);
                            
                            // Sync dedicated Vec signal after ActorVec change
                            let current_files = files_handle.lock_ref().to_vec();
                            files_vec_signal_sync.set_neq(current_files);
                        }
                    }
                    loading_started = loading_started_stream.next() => {
                        if let Some((file_id, filename)) = loading_started {
                            // Create new loading file
                            let loading_file = create_tracked_file(filename, FileState::Loading(LoadingStatus::Starting));
                            
                            // Check if file already exists, update or add
                            let current_files = files_handle.lock_ref().to_vec();
                            let existing_index = current_files.iter().position(|f| f.id == file_id);
                            
                            if let Some(_index) = existing_index {
                                // Update existing file
                                let updated_files: Vec<TrackedFile> = current_files.into_iter().map(|file| {
                                    if file.id == file_id {
                                        loading_file.clone()
                                    } else {
                                        file
                                    }
                                }).collect();
                                files_handle.lock_mut().replace_cloned(updated_files);
                            } else {
                                // Add new file
                                files_handle.lock_mut().push_cloned(loading_file);
                            }
                            
                            // Sync dedicated Vec signal after ActorVec change
                            let current_files = files_handle.lock_ref().to_vec();
                            files_vec_signal_sync.set_neq(current_files);
                        }
                    }
                    cleared = all_files_cleared_stream.next() => {
                        if cleared.is_some() {
                            files_handle.lock_mut().clear();
                            
                            // Sync dedicated Vec signal after ActorVec change
                            {
                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
                            }
                        }
                    }
                    complete => break, // All streams ended
                }
            }
        }});
        
        Self {
            files,
            files_vec_signal,
            config_files_loaded_relay,
            files_dropped_relay,
            file_removed_relay,
            file_reload_requested_relay,
            file_load_completed_relay,
            parsing_progress_relay,
            loading_started_relay,
            all_files_cleared_relay,
        }
    }
    
    // ===== SIGNAL ACCESS =====
    
    /// Get signal for all tracked files
    pub fn files_signal(&self) -> impl Signal<Item = Vec<TrackedFile>> {
        self.files_vec_signal.signal_cloned()
    }
    
    /// Get signal vec for tracked files (for items_signal_vec usage)
    pub fn files_signal_vec(&self) -> impl SignalVec<Item = TrackedFile> {
        self.files.signal_vec()
    }
    
    /// Get signal for file count
    pub fn file_count_signal(&self) -> impl Signal<Item = usize> {
        self.files_signal().map(|files| {
            let count = files.len();
            // Returning file count
            count
        })
    }
    
    /// Get current files (escape hatch for imperative usage during migration)
    /// NOTE: This breaks Actor+Relay principles but is needed for gradual migration
    pub fn get_current_files(&self) -> Vec<TrackedFile> {
        self.files_vec_signal.get_cloned()
    }
    
    // ===== COMPATIBILITY METHODS =====
    
    
    /// Reload a file by ID (uses relay-based reload with full parsing)
    pub fn reload_file(&self, file_id: String) {
        self.file_reload_requested_relay.send(file_id);
    }
    
    // This method broke Actor+Relay architecture by using .current_value()
    // 
    // Migration guide for code that used get_all_file_paths():
    // OLD: tracked_files_domain().get_all_file_paths()
    // NEW: Use tracked_files_signal() reactive pattern:
    //   tracked_files_signal().map(|files| files.iter().map(|f| f.path.clone()).collect())
    
    
    /// Update file state (compatibility method - should use relays instead)
    pub fn update_file_state(&self, file_id: String, new_state: FileState) {
        // Send the full FileState to preserve all parsed data
        self.file_load_completed_relay.send((file_id, new_state));
    }
    
}

// ===== GLOBAL INSTANCE =====

// ✅ ELIMINATED: TRACKED_FILES_INSTANCE duplicate - use global_domains::tracked_files_domain() instead



// ===== PUBLIC API FUNCTIONS =====



// ✅ ELIMINATED: tracked_files_signal_vec() - unused convenience function, use crate::actors::global_domains::tracked_files_signal_vec() directly

// ✅ ELIMINATED: tracked_files_count_signal() - Use crate::actors::global_domains::file_count_signal() directly



/// Create smart labels that disambiguate duplicate filenames

/// Trigger file parsing by sending LoadWaveformFile message to backend
fn trigger_file_parsing(file_path: String) {
    use crate::platform::{Platform, CurrentPlatform};
    use shared::UpMsg;
    
    zoon::Task::start(async move {
        match CurrentPlatform::send_message(UpMsg::LoadWaveformFile(file_path.clone())).await {
            Ok(()) => {
                // Parse request sent - monitoring progress through domain signals
            }
            Err(e) => {
                zoon::eprintln!("🚨 TrackedFiles: Failed to send parse request for {}: {}", file_path, e);
            }
        }
    });
}