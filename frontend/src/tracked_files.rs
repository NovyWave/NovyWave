//! TrackedFiles domain - file loading and management
//!
//! Uses MutableVec with direct methods for state management.

use shared::{CanonicalPathPayload, FileState, LoadingStatus, TrackedFile, create_tracked_file};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use zoon::*;

#[derive(Clone)]
pub struct TrackedFiles {
    pub files: MutableVec<TrackedFile>,
    pub files_vec_signal: Mutable<Vec<TrackedFile>>,
    watchdog_handles: Mutable<HashMap<String, Arc<TaskHandle>>>,
    file_reload_completed: Mutable<Option<String>>,
    file_reload_started: Mutable<Option<String>>,
}

impl TrackedFiles {
    pub fn new() -> Self {
        Self {
            files: MutableVec::new(),
            files_vec_signal: Mutable::new(Vec::new()),
            watchdog_handles: Mutable::new(HashMap::new()),
            file_reload_completed: Mutable::new(None),
            file_reload_started: Mutable::new(None),
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
        self.watchdog_handles.lock_mut().clear();
    }

    pub fn request_file_parse(&self, file_path: String) {
        let this = self.clone();
        let _ = Task::start_droppable(async move {
            if let Err(error) = send_parse_request_to_backend(file_path.clone()).await {
                let mut current = this.files_vec_signal.get_cloned();
                if let Some(f) = current.iter_mut().find(|f| f.path == file_path || f.canonical_path == file_path) {
                    f.state = FileState::Failed(shared::FileError::IoError {
                        path: file_path,
                        error,
                    });
                    this.files_vec_signal.set_neq(current.clone());
                    let mut vec = this.files.lock_mut();
                    vec.clear();
                    vec.extend(current);
                }
            }
        });
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
        let this = self.clone();
        let file_id_for_watchdog = file_id.clone();
        let _ = Task::start_droppable(async move {
            if let Err(error) = send_parse_request_to_backend(file_path.clone()).await {
                let mut current = this.files_vec_signal.get_cloned();
                if let Some(f) = current.iter_mut().find(|f| f.id == file_id) {
                    f.state = FileState::Failed(shared::FileError::IoError {
                        path: file_path,
                        error,
                    });
                }
                this.files_vec_signal.set_neq(current.clone());
                {
                    let mut vec = this.files.lock_mut();
                    vec.clear();
                    vec.extend(current);
                }
            } else {
                this.start_watchdog(file_id_for_watchdog);
            }
        });
    }

    fn start_watchdog(&self, file_id: String) {
        let this = self.clone();
        let file_id_for_timeout = file_id.clone();
        let handle = Arc::new(Task::start_droppable(async move {
            Timer::sleep(60_000).await;
            let mut current = this.files_vec_signal.get_cloned();
            if let Some(file) = current.iter_mut().find(|f| f.id == file_id_for_timeout) {
                if matches!(file.state, FileState::Loading(_)) {
                    zoon::println!("File loading timeout for: {file_id_for_timeout}");
                    file.state = FileState::Failed(shared::FileError::Timeout {
                        path: file_id_for_timeout.clone(),
                        timeout_seconds: 60,
                    });
                    this.files_vec_signal.set_neq(current.clone());
                    let mut vec = this.files.lock_mut();
                    vec.clear();
                    vec.extend(current);
                }
            }
            this.watchdog_handles.lock_mut().remove(&file_id_for_timeout);
        }));
        self.watchdog_handles.lock_mut().insert(file_id, handle);
    }

    fn cancel_watchdog(&self, file_id: &str) {
        self.watchdog_handles.lock_mut().remove(file_id);
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
