//! TrackedFiles Actor+Relay Domain
//!
//! Proper Actor+Relay architecture for file loading and management.
//! Uses dataflow Actor pattern instead of global Mutables.

use crate::dataflow::{ActorVec, Relay, relay};
use futures::{StreamExt, select};
use shared::{CanonicalPathPayload, FileState, LoadingStatus, TrackedFile, create_tracked_file};
use std::collections::HashSet;
use std::path::PathBuf;
use zoon::*;

/// TrackedFiles domain with proper Actor+Relay architecture
#[derive(Clone)]
pub struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,
    pub files_vec_signal: zoon::Mutable<Vec<TrackedFile>>, // Dedicated Vec signal for sync
    pub config_files_loaded_relay: Relay<Vec<CanonicalPathPayload>>,
    pub files_dropped_relay: Relay<Vec<PathBuf>>,
    pub file_removed_relay: Relay<String>,
    pub file_reload_requested_relay: Relay<CanonicalPathPayload>,
    pub file_load_completed_relay: Relay<(String, FileState)>,
    pub parsing_progress_relay: Relay<(String, f32, LoadingStatus)>,
    pub loading_started_relay: Relay<(String, String)>,
    pub all_files_cleared_relay: Relay<()>,
    pub file_parse_requested_relay: Relay<String>,
    pub file_reload_completed_relay: Relay<String>,
    pub file_loading_timeout_relay: Relay<String>,  // file_id when timeout fires
}

impl TrackedFiles {
    pub async fn new() -> Self {
        let (config_files_loaded_relay, mut config_files_loaded_stream) =
            relay::<Vec<CanonicalPathPayload>>();
        let (files_dropped_relay, mut files_dropped_stream) = relay::<Vec<PathBuf>>();
        let (file_removed_relay, mut file_removed_stream) = relay::<String>();
        let (file_reload_requested_relay, mut file_reload_requested_stream) =
            relay::<CanonicalPathPayload>();
        let (file_load_completed_relay, mut file_load_completed_stream) =
            relay::<(String, FileState)>();
        let (parsing_progress_relay, mut parsing_progress_stream) =
            relay::<(String, f32, LoadingStatus)>();
        let (loading_started_relay, mut loading_started_stream) = relay::<(String, String)>();
        let (all_files_cleared_relay, mut all_files_cleared_stream) = relay::<()>();
        let (file_parse_requested_relay, mut file_parse_requested_stream) = relay::<String>();
        let (file_reload_completed_relay, _) = relay::<String>();
        let (file_loading_timeout_relay, mut file_loading_timeout_stream) = relay::<String>();

        // Create dedicated vector signal to avoid SignalVec → Signal conversion antipattern
        let files_vec_signal = zoon::Mutable::new(Vec::<TrackedFile>::new());
        let files_vec_signal_for_actor = files_vec_signal.clone();
        let file_reload_completed_relay_for_actor = file_reload_completed_relay.clone();
        let file_loading_timeout_relay_for_actor = file_loading_timeout_relay.clone();

        // ActorVec handles all event processing within its processor - proper Actor+Relay architecture
        let files = ActorVec::new(vec![], async move |files_vec| {
            // ✅ Cache Current Values pattern - maintain local state within Actor loop
            let mut cached_files: Vec<TrackedFile> = vec![];
            let mut cached_loading_states: std::collections::HashMap<String, FileState> =
                std::collections::HashMap::new();
            let mut all_files_loaded_signaled = false;
            // Store watchdog handles to keep them alive across loop iterations
            let mut watchdog_handles: Vec<zoon::TaskHandle> = vec![];

            // Process all streams directly in ActorVec processor - proper pattern
            loop {
                select! {
                    file_paths = config_files_loaded_stream.next() => {
                        if let Some(file_payloads) = file_paths {
                            zoon::println!("[TRACKED_FILES] config_files_loaded received: {} files", file_payloads.len());
                            let mut seen = HashSet::new();
                            let tracked_files: Vec<TrackedFile> = file_payloads
                                .into_iter()
                                .filter_map(|payload| {
                                    let canonical = payload.canonical.clone();
                                    if canonical.is_empty() {
                                        return None;
                                    }
                                    if seen.insert(canonical) {
                                        Some(create_tracked_file(
                                            payload,
                                            FileState::Loading(LoadingStatus::Starting),
                                        ))
                                    } else {
                                        None
                                    }
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
                            zoon::println!("[TRACKED_FILES] sending parse requests for {} files", tracked_files.len());
                            for file in &tracked_files {
                                let load_path = if file.path.is_empty() {
                                    file.canonical_path.clone()
                                } else {
                                    file.path.clone()
                                };
                                zoon::println!("[TRACKED_FILES] sending parse request: {}", load_path);
                                if let Err(error) = send_parse_request_to_backend(load_path.clone()).await {
                                    // Update file state to Failed immediately on error
                                    if let Some(f) = cached_files.iter_mut().find(|f| f.id == file.id) {
                                        f.state = FileState::Failed(shared::FileError::IoError {
                                            path: load_path,
                                            error,
                                        });
                                    }
                                    files_vec_signal_for_actor.set_neq(cached_files.clone());
                                    {
                                        let mut vec = files_vec.lock_mut();
                                        vec.clear();
                                        vec.extend(cached_files.clone());
                                    }
                                } else {
                                    // Start timeout watchdog only on success
                                    let timeout_relay = file_loading_timeout_relay_for_actor.clone();
                                    let file_id = file.id.clone();
                                    let handle = Task::start_droppable(async move {
                                        Timer::sleep(60_000).await;
                                        timeout_relay.send(file_id);
                                    });
                                    watchdog_handles.push(handle);
                                }
                            }
                        }
                    }
                    file_paths = files_dropped_stream.next() => {
                        if let Some(file_paths) = file_paths {
                            let new_files: Vec<TrackedFile> = file_paths
                                .into_iter()
                                .map(|path| {
                                    let path_str = path.to_string_lossy().to_string();
                                    let payload = payload_from_string(path_str);
                                    create_tracked_file(
                                        payload,
                                        FileState::Loading(LoadingStatus::Starting),
                                    )
                                })
                                .collect();

                            for new_file in new_files {
                                let existing = cached_files.iter().any(|f| f.id == new_file.id);
                                if !existing {
                                    cached_files.push(new_file.clone());
                                    files_vec.lock_mut().push_cloned(new_file.clone());
                                    // Update dedicated Vec signal
                                    files_vec_signal_for_actor.set_neq(cached_files.clone());

                                    // Send parse request to backend for the new file
                                    if let Err(error) = send_parse_request_to_backend(new_file.path.clone()).await {
                                        // Update file state to Failed immediately on error
                                        if let Some(f) = cached_files.iter_mut().find(|f| f.id == new_file.id) {
                                            f.state = FileState::Failed(shared::FileError::IoError {
                                                path: new_file.path.clone(),
                                                error,
                                            });
                                        }
                                        files_vec_signal_for_actor.set_neq(cached_files.clone());
                                        {
                                            let mut vec = files_vec.lock_mut();
                                            vec.clear();
                                            vec.extend(cached_files.clone());
                                        }
                                    } else {
                                        // Start timeout watchdog only on success
                                        let timeout_relay = file_loading_timeout_relay_for_actor.clone();
                                        let file_id = new_file.id.clone();
                                        let handle = Task::start_droppable(async move {
                                            Timer::sleep(60_000).await;
                                            timeout_relay.send(file_id);
                                        });
                                        watchdog_handles.push(handle);
                                    }
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
                    payload = file_reload_requested_stream.next() => {
                        if let Some(payload) = payload {
                            let canonical_key = payload.canonical.clone();
                            if canonical_key.is_empty() {
                                zoon::println!("⚠️ TrackedFiles reload received empty canonical path");
                                continue;
                            }

                            let new_file = create_tracked_file(
                                payload.clone(),
                                FileState::Loading(LoadingStatus::Starting),
                            );

                            if let Some(index) = cached_files
                                .iter()
                                .position(|f| f.canonical_path == canonical_key)
                            {
                                cached_files[index] = new_file.clone();
                            } else {
                                zoon::println!(
                                    "⚠️ TrackedFiles reload miss for {} (cached {})",
                                    canonical_key,
                                    cached_files.len()
                                );
                                cached_files.push(new_file.clone());
                            }

                            {
                                let mut vec = files_vec.lock_mut();
                                vec.clear();
                                vec.extend(cached_files.clone());
                            }
                            files_vec_signal_for_actor.set_neq(cached_files.clone());

                            if let Err(error) = send_parse_request_to_backend(canonical_key.clone()).await {
                                // Update file state to Failed immediately on error
                                if let Some(f) = cached_files.iter_mut().find(|f| f.id == new_file.id) {
                                    f.state = FileState::Failed(shared::FileError::IoError {
                                        path: canonical_key.clone(),
                                        error,
                                    });
                                }
                                files_vec_signal_for_actor.set_neq(cached_files.clone());
                                {
                                    let mut vec = files_vec.lock_mut();
                                    vec.clear();
                                    vec.extend(cached_files.clone());
                                }
                            } else {
                                // Start timeout watchdog only on success
                                let timeout_relay = file_loading_timeout_relay_for_actor.clone();
                                let file_id = new_file.id.clone();
                                let handle = Task::start_droppable(async move {
                                    Timer::sleep(60_000).await;
                                    timeout_relay.send(file_id);
                                });
                                watchdog_handles.push(handle);
                            }
                        }
                    }
                    load_result = file_load_completed_stream.next() => {
                        if let Some((file_id, new_state)) = load_result {
                            cached_loading_states.insert(file_id.clone(), new_state.clone());

                            if let Some(index) = cached_files.iter().position(|tracked| {
                                tracked.canonical_path == file_id || tracked.path == file_id
                            }) {
                                let new_state_clone = new_state.clone();
                                {
                                    let file = &mut cached_files[index];
                                    file.id = file_id.clone();
                                    file.canonical_path = file_id.clone();
                                    file.state = new_state;
                                }
                                {
                                    let mut vec = files_vec.lock_mut();
                                    vec.clear();
                                    vec.extend(cached_files.clone());
                                }
                                files_vec_signal_for_actor.set_neq(cached_files.clone());

                                let all_done = cached_files.iter().all(|f| {
                                    matches!(f.state, shared::FileState::Loaded(_) | shared::FileState::Failed(_))
                                });

                                if all_done && !all_files_loaded_signaled {
                                    all_files_loaded_signaled = true;
                                }

                                if matches!(
                                    new_state_clone,
                                    FileState::Loaded(_)
                                        | FileState::Failed(_)
                                        | FileState::Missing(_)
                                        | FileState::Unsupported(_)
                                ) {
                                    file_reload_completed_relay_for_actor.send(file_id.clone());
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
                            let mut loading_file = create_tracked_file(
                                payload_from_string(file_id.clone()),
                                FileState::Loading(LoadingStatus::Starting),
                            );
                            loading_file.filename = filename.clone();
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
                            if let Err(error) = send_parse_request_to_backend(file_path.clone()).await {
                                // Try to find file and update state to Failed
                                if let Some(f) = cached_files.iter_mut().find(|f| f.path == file_path || f.canonical_path == file_path) {
                                    f.state = FileState::Failed(shared::FileError::IoError {
                                        path: file_path,
                                        error,
                                    });
                                    files_vec_signal_for_actor.set_neq(cached_files.clone());
                                    let mut vec = files_vec.lock_mut();
                                    vec.clear();
                                    vec.extend(cached_files.clone());
                                }
                            }
                        }
                    }
                    file_id = file_loading_timeout_stream.next() => {
                        if let Some(file_id) = file_id {
                            // Check if file is still in Loading state
                            if let Some(file) = cached_files.iter_mut().find(|f| f.id == file_id) {
                                if matches!(file.state, FileState::Loading(_)) {
                                    zoon::println!("File loading timeout for: {file_id}");
                                    file.state = FileState::Failed(shared::FileError::Timeout {
                                        path: file_id.clone(),
                                        timeout_seconds: 60,
                                    });
                                    // Update the files_vec_signal
                                    files_vec_signal_for_actor.set_neq(cached_files.clone());
                                    // Also update the ActorVec
                                    let mut vec = files_vec.lock_mut();
                                    if let Some(idx) = vec.iter().position(|f| f.id == file_id) {
                                        if let Some(file_in_vec) = cached_files.iter().find(|f| f.id == file_id) {
                                            vec.set_cloned(idx, file_in_vec.clone());
                                        }
                                    }
                                }
                            }
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
            file_reload_completed_relay,
            file_loading_timeout_relay,
        }
    }

    pub fn reload_existing_paths(&self, files: Vec<CanonicalPathPayload>) {
        for payload in files {
            self.file_reload_requested_relay.send(payload);
        }
    }

    pub fn load_new_paths(&self, files: Vec<CanonicalPathPayload>) {
        for payload in files {
            self.file_reload_requested_relay.send(payload);
        }
    }

    /// Get signal for tracked files list
    pub fn files_signal(&self) -> impl zoon::Signal<Item = Vec<TrackedFile>> {
        self.files.signal_vec().to_signal_cloned()
    }

    pub fn get_current_files(&self) -> Vec<TrackedFile> {
        self.files_vec_signal.get_cloned()
    }

    pub fn update_file_state(&self, file_id: String, new_state: FileState) {
        self.file_load_completed_relay.send((file_id, new_state));
    }
}

async fn send_parse_request_to_backend(file_path: String) -> Result<(), String> {
    use crate::platform::{CurrentPlatform, Platform};
    use shared::UpMsg;

    match CurrentPlatform::send_message(UpMsg::LoadWaveformFile(file_path.clone())).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let error_msg = format!("Failed to send parse request: {e}");
            zoon::eprintln!("🚨 TrackedFiles: {error_msg} for {file_path}");
            Err(error_msg)
        }
    }
}

fn payload_from_string(path: String) -> CanonicalPathPayload {
    CanonicalPathPayload::new(path)
}
