//! TrackedFiles domain - file loading and management
//!
//! Uses MutableVec with direct methods for state management.

use futures::StreamExt;
use shared::{CanonicalPathPayload, FileState, LoadingStatus, TrackedFile, create_tracked_file};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use zoon::*;

#[derive(Clone)]
pub struct TrackedFiles {
    pub files: MutableVec<TrackedFile>,
    pub files_vec_signal: Mutable<Vec<TrackedFile>>,
    loading_start_times: Mutable<HashMap<String, f64>>,
    file_reload_completed: Mutable<Option<String>>,
    file_reload_started: Mutable<Option<String>>,
    parse_request_sender: futures::channel::mpsc::UnboundedSender<(String, String)>,
    _parse_task: Arc<TaskHandle>,
    _global_watchdog_task: Arc<TaskHandle>,
}

impl TrackedFiles {
    pub fn new() -> Self {
        let (parse_request_sender, mut parse_request_receiver) =
            futures::channel::mpsc::unbounded::<(String, String)>();

        let files_vec_signal: Mutable<Vec<TrackedFile>> = Mutable::new(Vec::new());
        let files = MutableVec::new();
        let loading_start_times: Mutable<HashMap<String, f64>> = Mutable::new(HashMap::new());

        let files_vec_for_task = files_vec_signal.clone();
        let files_for_task = files.clone();
        let loading_start_times_for_task = loading_start_times.clone();

        let _parse_task = Arc::new(Task::start_droppable(async move {
            while let Some((file_id, file_path)) = parse_request_receiver.next().await {
                if let Err(error) = send_parse_request_to_backend(file_path.clone()).await {
                    let mut current = files_vec_for_task.get_cloned();
                    if let Some(f) = current
                        .iter_mut()
                        .find(|f| f.id == file_id || f.path == file_path || f.canonical_path == file_path)
                    {
                        f.state = FileState::Failed(shared::FileError::IoError {
                            path: file_path,
                            error,
                        });
                    }
                    files_vec_for_task.set_neq(current.clone());
                    {
                        let mut vec = files_for_task.lock_mut();
                        vec.clear();
                        vec.extend(current);
                    }
                } else {
                    // Record loading start time - global watchdog will check for timeouts
                    loading_start_times_for_task.lock_mut().insert(file_id, js_sys::Date::now());
                }
            }
        }));

        // Global watchdog: checks all loading files every 10 seconds
        let files_vec_for_watchdog = files_vec_signal.clone();
        let files_for_watchdog = files.clone();
        let loading_start_times_for_watchdog = loading_start_times.clone();
        let _global_watchdog_task = Arc::new(Task::start_droppable(async move {
            const TIMEOUT_MS: f64 = 60_000.0;
            const CHECK_INTERVAL_MS: u32 = 10_000;

            loop {
                Timer::sleep(CHECK_INTERVAL_MS).await;
                let now = js_sys::Date::now();
                let start_times = loading_start_times_for_watchdog.get_cloned();
                let mut timed_out_files: Vec<String> = Vec::new();

                // Check for timeouts
                for (file_id, start_time) in &start_times {
                    if now - start_time > TIMEOUT_MS {
                        timed_out_files.push(file_id.clone());
                    }
                }

                // Mark timed out files as failed
                if !timed_out_files.is_empty() {
                    let mut current = files_vec_for_watchdog.get_cloned();
                    let mut changed = false;

                    for file_id in &timed_out_files {
                        if let Some(file) = current.iter_mut().find(|f| f.id == *file_id) {
                            if matches!(file.state, FileState::Loading(_)) {
                                zoon::println!("File loading timeout for: {file_id}");
                                file.state = FileState::Failed(shared::FileError::Timeout {
                                    path: file_id.clone(),
                                    timeout_seconds: 60,
                                });
                                changed = true;
                            }
                        }
                        loading_start_times_for_watchdog.lock_mut().remove(file_id);
                    }

                    if changed {
                        files_vec_for_watchdog.set_neq(current.clone());
                        let mut vec = files_for_watchdog.lock_mut();
                        vec.clear();
                        vec.extend(current);
                    }
                }
            }
        }));

        Self {
            files,
            files_vec_signal,
            loading_start_times,
            file_reload_completed: Mutable::new(None),
            file_reload_started: Mutable::new(None),
            parse_request_sender,
            _parse_task,
            _global_watchdog_task,
        }
    }

    pub fn load_config_files(&self, file_payloads: Vec<CanonicalPathPayload>) {
        zoon::println!("[TRACKED_FILES] config_files_loaded: {} files", file_payloads.len());

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

        {
            let mut vec = self.files.lock_mut();
            vec.clear();
            vec.extend(tracked_files.clone());
        }
        self.files_vec_signal.set_neq(tracked_files.clone());

        zoon::println!("[TRACKED_FILES] sending parse requests for {} files", tracked_files.len());
        for file in &tracked_files {
            let load_path = if file.path.is_empty() {
                file.canonical_path.clone()
            } else {
                file.path.clone()
            };
            zoon::println!("[TRACKED_FILES] sending parse request: {}", load_path);
            self.send_parse_request(file.id.clone(), load_path);
        }
    }

    pub fn add_dropped_files(&self, file_paths: Vec<PathBuf>) {
        let current_files = self.files_vec_signal.get_cloned();

        for path in file_paths {
            let path_str = path.to_string_lossy().to_string();
            let payload = payload_from_string(path_str.clone());
            let new_file = create_tracked_file(
                payload,
                FileState::Loading(LoadingStatus::Starting),
            );

            let existing = current_files.iter().any(|f| f.id == new_file.id);
            if !existing {
                self.files.lock_mut().push_cloned(new_file.clone());
                let mut current = self.files_vec_signal.get_cloned();
                current.push(new_file.clone());
                self.files_vec_signal.set_neq(current);

                self.send_parse_request(new_file.id.clone(), new_file.path.clone());
            }
        }
    }

    pub fn remove_file(&self, file_id: String) {
        self.files.lock_mut().retain(|f| f.id != file_id);
        let mut current = self.files_vec_signal.get_cloned();
        current.retain(|f| f.id != file_id);
        self.files_vec_signal.set_neq(current);
        self.cancel_watchdog(&file_id);
    }

    pub fn reload_file(&self, payload: CanonicalPathPayload) {
        let canonical_key = payload.canonical.clone();
        if canonical_key.is_empty() {
            zoon::println!("⚠️ TrackedFiles reload received empty canonical path");
            return;
        }

        let new_file = create_tracked_file(
            payload.clone(),
            FileState::Loading(LoadingStatus::Starting),
        );

        let mut current = self.files_vec_signal.get_cloned();
        if let Some(index) = current.iter().position(|f| f.canonical_path == canonical_key) {
            current[index] = new_file.clone();
        } else {
            zoon::println!(
                "⚠️ TrackedFiles reload miss for {} (cached {})",
                canonical_key,
                current.len()
            );
            current.push(new_file.clone());
        }

        {
            let mut vec = self.files.lock_mut();
            vec.clear();
            vec.extend(current.clone());
        }
        self.files_vec_signal.set_neq(current);

        self.file_reload_started.set(Some(canonical_key.clone()));
        self.send_parse_request(new_file.id.clone(), canonical_key);
    }

    pub fn update_file_state(&self, file_id: String, new_state: FileState) {
        let mut current = self.files_vec_signal.get_cloned();

        if let Some(index) = current.iter().position(|tracked| {
            tracked.canonical_path == file_id || tracked.path == file_id
        }) {
            {
                let file = &mut current[index];
                file.id = file_id.clone();
                file.canonical_path = file_id.clone();
                file.state = new_state.clone();
            }
            {
                let mut vec = self.files.lock_mut();
                vec.clear();
                vec.extend(current.clone());
            }
            self.files_vec_signal.set_neq(current);

            if matches!(
                new_state,
                FileState::Loaded(_)
                    | FileState::Failed(_)
                    | FileState::Missing(_)
                    | FileState::Unsupported(_)
            ) {
                self.file_reload_completed.set(Some(file_id.clone()));
                self.cancel_watchdog(&file_id);
            }
        }
    }

    pub fn update_parsing_progress(&self, file_id: String, _progress: f32, status: LoadingStatus) {
        let mut current = self.files_vec_signal.get_cloned();
        for file in &mut current {
            if file.id == file_id {
                file.state = FileState::Loading(status.clone());
            }
        }
        {
            let mut vec = self.files.lock_mut();
            vec.clear();
            vec.extend(current.clone());
        }
        self.files_vec_signal.set_neq(current);
    }

    pub fn notify_loading_started(&self, file_id: String, filename: String) {
        let mut loading_file = create_tracked_file(
            payload_from_string(file_id.clone()),
            FileState::Loading(LoadingStatus::Starting),
        );
        loading_file.filename = filename.clone();

        let mut current = self.files_vec_signal.get_cloned();
        if let Some(index) = current.iter().position(|f| f.id == file_id) {
            current[index] = loading_file.clone();
        } else {
            current.push(loading_file.clone());
        }

        {
            let mut vec = self.files.lock_mut();
            vec.clear();
            vec.extend(current.clone());
        }
        self.files_vec_signal.set_neq(current);
    }

    pub fn clear_all_files(&self) {
        self.files.lock_mut().clear();
        self.files_vec_signal.set_neq(Vec::new());
        self.loading_start_times.lock_mut().clear();
    }

    pub fn request_file_parse(&self, file_path: String) {
        let _ = self
            .parse_request_sender
            .unbounded_send((file_path.clone(), file_path));
    }

    pub fn reload_existing_paths(&self, files: Vec<CanonicalPathPayload>) {
        for payload in files {
            self.reload_file(payload);
        }
    }

    pub fn load_new_paths(&self, files: Vec<CanonicalPathPayload>) {
        for payload in files {
            self.reload_file(payload);
        }
    }

    pub fn files_signal(&self) -> impl zoon::Signal<Item = Vec<TrackedFile>> {
        self.files.signal_vec_cloned().to_signal_cloned()
    }

    pub fn get_current_files(&self) -> Vec<TrackedFile> {
        self.files_vec_signal.get_cloned()
    }

    pub fn file_reload_completed_signal(&self) -> impl Signal<Item = Option<String>> {
        self.file_reload_completed.signal_cloned()
    }

    pub fn file_reload_started_signal(&self) -> impl Signal<Item = Option<String>> {
        self.file_reload_started.signal_cloned()
    }

    fn send_parse_request(&self, file_id: String, file_path: String) {
        let _ = self.parse_request_sender.unbounded_send((file_id, file_path));
    }

    fn cancel_watchdog(&self, file_id: &str) {
        self.loading_start_times.lock_mut().remove(file_id);
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
