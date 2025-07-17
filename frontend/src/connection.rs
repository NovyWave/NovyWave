use zoon::*;
use crate::{LOADING_FILES, LOADED_FILES, check_loading_complete, config};
use crate::config::CONFIG_LOADED;
use crate::error_display::add_error_alert;
use crate::state::ErrorAlert;
use shared::{UpMsg, DownMsg};
use shared::{LoadingFile, LoadingStatus};

static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    Connection::new(|down_msg, _| {
        // DownMsg logging disabled - causes CLI overflow with large files
        match down_msg {
            DownMsg::ParsingStarted { file_id, filename } => {
                // Add or update loading file
                let loading_file = LoadingFile {
                    file_id: file_id.clone(),
                    filename: filename.clone(),
                    progress: 0.0,
                    status: LoadingStatus::Starting,
                };
                
                LOADING_FILES.lock_mut().push_cloned(loading_file);
            }
            DownMsg::ParsingProgress { file_id, progress } => {
                // Update progress for the file
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.progress = progress;
                        file.status = LoadingStatus::Parsing;
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
            }
            DownMsg::FileLoaded { file_id, hierarchy } => {
                // Add loaded files to the TreeView state
                for file in hierarchy.files {
                    LOADED_FILES.lock_mut().push_cloned(file.clone());
                    
                    // Store scope selection for later restoration (don't restore immediately)
                    // This prevents multiple files from fighting over global selection during loading
                }
                
                // Mark file as completed
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.progress = 1.0;
                        file.status = LoadingStatus::Completed;
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
                
                // Check if all files are completed
                check_loading_complete();
                
                // Auto-save config with updated file list
                if CONFIG_LOADED.get() {
                    config::save_file_list();
                }
            }
            DownMsg::ParsingError { file_id, error } => {
                // Find the filename for the error alert
                let filename = {
                    let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                    current_files.iter()
                        .find(|file| file.file_id == file_id)
                        .map(|file| file.filename.clone())
                        .unwrap_or_else(|| "Unknown file".to_string())
                };
                
                // Create and display error alert
                let error_alert = ErrorAlert::new_file_parsing_error(
                    file_id.clone(),
                    filename,
                    error.clone()
                );
                add_error_alert(error_alert);
                
                // Mark file as error
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.status = LoadingStatus::Error(error.clone());
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
                
                // Check if all files are completed
                check_loading_complete();
            }
            DownMsg::DirectoryContents { path, items } => {
                // Cache directory contents
                crate::FILE_TREE_CACHE.lock_mut().insert(path.clone(), items.clone());
                
                // Auto-expand home directory path and its parent directories
                if path.contains("/home/") || path.starts_with("/Users/") {
                    let mut expanded = crate::FILE_PICKER_EXPANDED.lock_mut();
                    
                    // Expand the home directory itself
                    expanded.insert(path.clone());
                    
                    // Only expand parent directories, don't browse them automatically
                    // This prevents infinite loops
                    let mut parent_path = std::path::Path::new(&path);
                    while let Some(parent) = parent_path.parent() {
                        let parent_str = parent.to_string_lossy().to_string();
                        if parent_str == "" || parent_str == "/" {
                            break;
                        }
                        expanded.insert(parent_str);
                        parent_path = parent;
                    }
                }
                
                // Clear any previous error for this directory (fresh data overwrites cached errors)
                crate::FILE_PICKER_ERROR.set_neq(None);
                crate::FILE_PICKER_ERROR_CACHE.lock_mut().remove(&path);
            }
            DownMsg::DirectoryError { path, error } => {
                // Create and display directory error alert (auto-dismisses)
                let error_alert = ErrorAlert::new_directory_error(path.clone(), error.clone());
                add_error_alert(error_alert);
                
                // Store error for this specific directory
                crate::FILE_PICKER_ERROR_CACHE.lock_mut().insert(path.clone(), error);
                
                // Clear global error (we now use per-directory errors)
                crate::FILE_PICKER_ERROR.set_neq(None);
            }
            DownMsg::ConfigLoaded(config) => {
                crate::config::apply_config(config);
            }
            DownMsg::ConfigSaved => {
                // Config saved successfully
            }
            DownMsg::ConfigError(_error) => {
                // Config error: {}
            }
        }
    })
});

pub fn send_up_msg(up_msg: UpMsg) {
    Task::start(async move {
        let result = CONNECTION.send_up_msg(up_msg).await;
        if let Err(error) = result {
            // Create and display connection error alert
            let error_alert = ErrorAlert::new_connection_error(format!("Failed to send message: {:?}", error));
            add_error_alert(error_alert);
        }
    });
}

pub fn init_connection() {
    CONNECTION.init_lazy();
}