//! TrackedFiles Actor+Relay Domain
//!
//! Proper Actor+Relay architecture for file loading and management.
//! Uses dataflow Actor pattern instead of global Mutables.

use crate::dataflow::{Actor, ActorVec, Relay, relay};
use futures::{StreamExt, select};
use shared::{FileState, LoadingStatus, TrackedFile, create_tracked_file};
use zoon::*;

/// TrackedFiles domain with proper Actor+Relay architecture
#[derive(Clone)]
pub struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,
    pub files_vec_signal: zoon::Mutable<Vec<TrackedFile>>, // Dedicated Vec signal for sync
    pub config_files_loaded_relay: Relay<Vec<String>>,
    pub files_dropped_relay: Relay<Vec<std::path::PathBuf>>,
    pub file_removed_relay: Relay<String>,
    pub file_reload_requested_relay: Relay<String>,
    pub file_load_completed_relay: Relay<(String, FileState)>,
    pub parsing_progress_relay: Relay<(String, f32, LoadingStatus)>,
    pub loading_started_relay: Relay<(String, String)>,
    pub all_files_cleared_relay: Relay<()>,
    pub file_parse_requested_relay: Relay<String>,
}

impl TrackedFiles {
    pub async fn new() -> Self {
        let (config_files_loaded_relay, mut config_files_loaded_stream) = relay::<Vec<String>>();
        let (files_dropped_relay, mut files_dropped_stream) = relay::<Vec<std::path::PathBuf>>();
        let (file_removed_relay, mut file_removed_stream) = relay::<String>();
        let (file_reload_requested_relay, mut file_reload_requested_stream) = relay::<String>();
        let (file_load_completed_relay, mut file_load_completed_stream) =
            relay::<(String, FileState)>();
        let (parsing_progress_relay, mut parsing_progress_stream) =
            relay::<(String, f32, LoadingStatus)>();
        let (loading_started_relay, mut loading_started_stream) = relay::<(String, String)>();
        let (all_files_cleared_relay, mut all_files_cleared_stream) = relay::<()>();
        let (file_parse_requested_relay, mut file_parse_requested_stream) = relay::<String>();

        // Create dedicated vector signal to avoid SignalVec → Signal conversion antipattern
        let files_vec_signal = zoon::Mutable::new(Vec::<TrackedFile>::new());
        let files_vec_signal_for_actor = files_vec_signal.clone();

        // ActorVec handles all event processing within its processor - proper Actor+Relay architecture
        let files = ActorVec::new(vec![], async move |files_vec| {
            // ✅ Cache Current Values pattern - maintain local state within Actor loop
            let mut cached_files: Vec<TrackedFile> = vec![];
            let mut cached_loading_states: std::collections::HashMap<String, FileState> =
                std::collections::HashMap::new();
            let mut all_files_loaded_signaled = false;

            // Process all streams directly in ActorVec processor - proper pattern
            loop {
                select! {
                    file_paths = config_files_loaded_stream.next() => {
                        if let Some(file_paths) = file_paths {
                            let tracked_files: Vec<TrackedFile> = file_paths.into_iter()
                                .map(|path_str| {
                                    let tracked_file = create_tracked_file(path_str.clone(), FileState::Loading(LoadingStatus::Starting));
                                    tracked_file
                                })
                                .collect();
                            cached_files = tracked_files.clone();
                            {
                                let mut vec = files_vec.lock_mut();
                                vec.clear();
                                vec.extend(tracked_files.clone());
                            }
                            // Update dedicated Vec signal
                            files_vec_signal_for_actor.set_neq(cached_files.clone());

                            // Send parse requests for config-loaded files
                            for file in tracked_files {
                                send_parse_request_to_backend(file.path.clone()).await;
                            }
                        }
                    }
                    file_paths = files_dropped_stream.next() => {
                        if let Some(file_paths) = file_paths {
                            let new_files: Vec<TrackedFile> = file_paths.into_iter()
                                .map(|path| {
                                    let path_str = path.to_string_lossy().to_string();
                                    create_tracked_file(path_str, FileState::Loading(LoadingStatus::Starting))
                                })
                                .collect();

                            for new_file in new_files {
                                let existing = cached_files.iter().any(|f| f.id == new_file.id);
                                if !existing {
                                    cached_files.push(new_file.clone());
                                    files_vec.lock_mut().push_cloned(new_file.clone());
                                    // Update dedicated Vec signal
                                    let current_value_before = files_vec_signal_for_actor.get_cloned();
                                    let new_files_vec = cached_files.clone();
                                    files_vec_signal_for_actor.set_neq(new_files_vec.clone());
                                    let current_value_after = files_vec_signal_for_actor.get_cloned();

                                    // Send parse request to backend for the new file
                                    send_parse_request_to_backend(new_file.path.clone()).await;
                                }
                            }
                        }
                    }
                    file_id = file_removed_stream.next() => {
                        if let Some(file_id) = file_id {
                            cached_files.retain(|f| f.id != file_id);
                            files_vec.lock_mut().retain(|f| f.id != file_id);
                            // Update dedicated Vec signal
                            files_vec_signal_for_actor.set_neq(cached_files.clone());
                        }
                    }
                    file_id = file_reload_requested_stream.next() => {
                        if let Some(file_id) = file_id {
                            if let Some(existing_file) = cached_files.iter().find(|f| f.id == file_id).cloned() {
                                let new_file = create_tracked_file(
                                    existing_file.path.clone(),
                                    FileState::Loading(LoadingStatus::Starting)
                                );

                                // Update cache
                                cached_files.retain(|f| f.id != file_id);
                                cached_files.push(new_file.clone());

                                // Update files_vec properly
                                let mut files = files_vec.lock_mut();
                                files.retain(|f| f.id != file_id);
                                files.push_cloned(new_file.clone());

                                // Send parse request for reloaded file
                                send_parse_request_to_backend(new_file.path.clone()).await;
                            }
                        }
                    }
                    load_result = file_load_completed_stream.next() => {
                        if let Some((file_id, new_state)) = load_result {
                            cached_loading_states.insert(file_id.clone(), new_state.clone());

                            // Update cached state
                            if let Some(file) = cached_files.iter_mut().find(|f| f.id == file_id) {
                                file.state = new_state;
                                {
                                    let mut vec = files_vec.lock_mut();
                                    vec.clear();
                                    vec.extend(cached_files.clone());
                                }
                                // Update dedicated Vec signal
                                files_vec_signal_for_actor.set_neq(cached_files.clone());

                                let all_done = cached_files.iter().all(|f| {
                                    matches!(f.state, shared::FileState::Loaded(_) | shared::FileState::Failed(_))
                                });

                                if all_done && !all_files_loaded_signaled {
                                    all_files_loaded_signaled = true;
                                }
                            } else {
                                // No matching file found; ignore
                            }
                        }
                    }
                    progress_result = parsing_progress_stream.next() => {
                        if let Some((file_id, _progress, status)) = progress_result {
                            // Update cached files
                            for file in &mut cached_files {
                                if file.id == file_id {
                                    file.state = FileState::Loading(status.clone());
                                }
                            }
                            {
                                let mut vec = files_vec.lock_mut();
                                vec.clear();
                                vec.extend(cached_files.clone());
                            }
                            // Update dedicated Vec signal
                            files_vec_signal_for_actor.set_neq(cached_files.clone());
                        }
                    }
                    loading_result = loading_started_stream.next() => {
                        if let Some((file_id, filename)) = loading_result {
                            let loading_file = create_tracked_file(filename, FileState::Loading(LoadingStatus::Starting));
                            let existing_index = cached_files.iter().position(|f| f.id == file_id);

                            if let Some(_index) = existing_index {
                                // Update cached files
                                for file in &mut cached_files {
                                    if file.id == file_id {
                                        *file = loading_file.clone();
                                    }
                                }
                                {
                                    let mut vec = files_vec.lock_mut();
                                    vec.clear();
                                    vec.extend(cached_files.clone());
                                }
                                // Update dedicated Vec signal
                                files_vec_signal_for_actor.set_neq(cached_files.clone());
                            } else {
                                cached_files.push(loading_file.clone());
                                files_vec.lock_mut().push_cloned(loading_file);
                                // Update dedicated Vec signal
                                files_vec_signal_for_actor.set_neq(cached_files.clone());
                            }
                        }
                    }
                    clear_result = all_files_cleared_stream.next() => {
                        if let Some(()) = clear_result {
                            cached_files.clear();
                            files_vec.lock_mut().clear();
                            // Update dedicated Vec signal
                            files_vec_signal_for_actor.set_neq(Vec::new());
                        }
                    }
                    file_path = file_parse_requested_stream.next() => {
                        if let Some(file_path) = file_path {
                            // ✅ CORRECT: Direct async call within Actor - NO zoon::Task needed!
                            send_parse_request_to_backend(file_path).await;
                        }
                    }
                    complete => break,
                }
            }
        });

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
            file_parse_requested_relay,
        }
    }

    pub fn reload_file(&self, file_id: String) {
        self.file_reload_requested_relay.send(file_id);
    }

    /// Get signal for tracked files list
    pub fn files_signal(&self) -> impl zoon::Signal<Item = Vec<TrackedFile>> {
        self.files.signal_vec().to_signal_cloned()
    }

    pub fn get_current_files(&self) -> Vec<TrackedFile> {
        // TODO: This should use signals for proper Actor+Relay architecture
        // For now, return empty vec until callers are updated to use signals
        Vec::new()
    }

    pub fn update_file_state(&self, file_id: String, new_state: FileState) {
        self.file_load_completed_relay.send((file_id, new_state));
    }
}

/// Update the state of an existing tracked file
/// Utility function for compatibility with existing code
pub fn update_tracked_file_state(
    file_id: &str,
    new_state: FileState,
    tracked_files: &TrackedFiles,
) {
    tracked_files
        .file_load_completed_relay
        .send((file_id.to_string(), new_state));
}

async fn send_parse_request_to_backend(file_path: String) {
    use crate::platform::{CurrentPlatform, Platform};
    use shared::UpMsg;

    match CurrentPlatform::send_message(UpMsg::LoadWaveformFile(file_path.clone())).await {
        Ok(()) => {}
        Err(e) => {
            zoon::eprintln!(
                "🚨 TrackedFiles: Failed to send parse request for {}: {}",
                file_path,
                e
            );
        }
    }
}
