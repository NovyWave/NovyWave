//! TrackedFiles Actor+Relay Domain
//!
//! Proper Actor+Relay architecture for file loading and management.
//! Uses dataflow Actor pattern instead of global Mutables.

use crate::dataflow::{ActorVec, Relay, relay};
use futures::{StreamExt, select};
use shared::{FileState, LoadingStatus, TrackedFile, create_tracked_file};
use zoon::*;

/// TrackedFiles domain with proper Actor+Relay architecture
#[derive(Clone)]
pub struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,
    pub files_vec_signal: zoon::Mutable<Vec<TrackedFile>>,
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

        let files_vec_signal = zoon::Mutable::new(vec![]);
        let files = ActorVec::new(vec![], {
            let files_vec_signal_sync = files_vec_signal.clone();
            async move |files_handle| {
                let mut cached_loading_states: std::collections::HashMap<String, FileState> =
                    std::collections::HashMap::new();
                let mut all_files_loaded_signaled = false;

                loop {
                    select! {
                        config_files = config_files_loaded_stream.next() => {
                            if let Some(file_paths) = config_files {
                                let tracked_files: Vec<TrackedFile> = file_paths.into_iter()
                                    .map(|path_str| create_tracked_file(path_str, FileState::Loading(LoadingStatus::Starting)))
                                    .collect();

                                for file in &tracked_files {
                                    file_parse_requested_relay.send(file.path.clone());
                                }

                                files_handle.lock_mut().replace_cloned(tracked_files);

                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
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
                                    let existing = files_handle.lock_ref().iter().any(|f| f.id == new_file.id);
                                    if !existing {
                                        file_parse_requested_relay.send(new_file.path.clone());
                                        files_handle.lock_mut().push_cloned(new_file);
                                        
                                        let current_files = files_handle.lock_ref().to_vec();
                                        files_vec_signal_sync.set_neq(current_files);
                                    }
                                }
                            }
                        }
                        removed_file = file_removed_stream.next() => {
                            if let Some(file_id) = removed_file {
                                files_handle.lock_mut().retain(|f| f.id != file_id);
                                
                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
                            }
                        }
                        reload_requested = file_reload_requested_stream.next() => {
                            if let Some(file_id) = reload_requested {
                                let existing_file = files_handle.lock_ref().iter()
                                    .find(|f| f.id == file_id).cloned();

                                if let Some(existing_file) = existing_file {
                                    let new_file = create_tracked_file(
                                        existing_file.path.clone(),
                                        FileState::Loading(LoadingStatus::Starting)
                                    );

                                    file_parse_requested_relay.send(new_file.path.clone());

                                    let mut files = files_handle.lock_mut();
                                    files.retain(|f| f.id != file_id);
                                    files.push_cloned(new_file);
                                    
                                    let current_files = files_handle.lock_ref().to_vec();
                                    files_vec_signal_sync.set_neq(current_files);
                                }
                            }
                        }
                        completed = file_load_completed_stream.next() => {
                            if let Some((file_id, new_state)) = completed {
                                cached_loading_states.insert(file_id.clone(), new_state.clone());

                                let mut files = files_handle.lock_ref().to_vec();
                                if let Some(file) = files.iter_mut().find(|f| f.id == file_id) {
                                    file.state = new_state;
                                    files_handle.lock_mut().replace_cloned(files);

                                    let current_files = files_handle.lock_ref().to_vec();
                                    files_vec_signal_sync.set_neq(current_files);
                                    let current_files = files_handle.lock_ref();
                                    let all_done = current_files.iter().all(|f| {
                                        matches!(f.state, shared::FileState::Loaded(_) | shared::FileState::Failed(_))
                                    });

                                    if all_done && !all_files_loaded_signaled {
                                        all_files_loaded_signaled = true;

                                        zoon::Task::start({
                                            async move {
                                                crate::selected_variables::SelectedVariables::restore_scope_selections_reactive().await;
                                            }
                                        });
                                    }
                                }
                            }
                        }
                        parsing_progress = parsing_progress_stream.next() => {
                            if let Some((file_id, _progress, status)) = parsing_progress {
                                let current_files = files_handle.lock_ref().to_vec();
                                let updated_files: Vec<TrackedFile> = current_files.into_iter().map(|mut file| {
                                    if file.id == file_id {
                                        file.state = FileState::Loading(status.clone());
                                    }
                                    file
                                }).collect();

                                files_handle.lock_mut().replace_cloned(updated_files);
                                
                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
                            }
                        }
                        loading_started = loading_started_stream.next() => {
                            if let Some((file_id, filename)) = loading_started {
                                let loading_file = create_tracked_file(filename, FileState::Loading(LoadingStatus::Starting));

                                let current_files = files_handle.lock_ref().to_vec();
                                let existing_index = current_files.iter().position(|f| f.id == file_id);

                                if let Some(_index) = existing_index {
                                    let updated_files: Vec<TrackedFile> = current_files.into_iter().map(|file| {
                                        if file.id == file_id {
                                            loading_file.clone()
                                        } else {
                                            file
                                        }
                                    }).collect();
                                    files_handle.lock_mut().replace_cloned(updated_files);
                                } else {
                                    files_handle.lock_mut().push_cloned(loading_file);
                                }

                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
                            }
                        }
                        cleared = all_files_cleared_stream.next() => {
                            if cleared.is_some() {
                                files_handle.lock_mut().clear();
                                
                                let current_files = files_handle.lock_ref().to_vec();
                                files_vec_signal_sync.set_neq(current_files);
                            }
                        }
                        parse_requested = file_parse_requested_stream.next() => {
                            if let Some(file_path) = parse_requested {
                                send_parse_request_to_backend(file_path).await;
                            }
                        }
                        complete => break, // All streams ended
                    }
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

    pub fn files_signal(&self) -> impl Signal<Item = Vec<TrackedFile>> {
        self.files_vec_signal.signal_cloned()
    }

    pub fn files_signal_vec(&self) -> impl SignalVec<Item = TrackedFile> {
        self.files.signal_vec()
    }

    pub fn file_count_signal(&self) -> impl Signal<Item = usize> {
        self.files_signal().map(|files| files.len())
    }

    pub fn get_current_files(&self) -> Vec<TrackedFile> {
        self.files_vec_signal.get_cloned()
    }

    pub fn reload_file(&self, file_id: String) {
        self.file_reload_requested_relay.send(file_id);
    }

    pub fn update_file_state(&self, file_id: String, new_state: FileState) {
        self.file_load_completed_relay.send((file_id, new_state));
    }
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
