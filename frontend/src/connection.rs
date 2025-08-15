use zoon::*;
use crate::{LOADING_FILES, LOADED_FILES, check_loading_complete, config};
use crate::config::CONFIG_LOADED;
use crate::error_display::add_error_alert;
use crate::state::ErrorAlert;
use crate::utils::restore_scope_selection_for_file;
use shared::{UpMsg, DownMsg};
use shared::{LoadingFile, LoadingStatus};

static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    Connection::new(|down_msg, _| {
        // DownMsg logging disabled - causes CLI overflow with large files
        match down_msg {
            DownMsg::ParsingStarted { file_id, filename } => {
                // Update TRACKED_FILES with parsing started status
                crate::state::update_tracked_file_state(&file_id, shared::FileState::Loading(shared::LoadingStatus::Parsing));
                
                // Also maintain legacy LOADING_FILES for backward compatibility during transition
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
                // Update TRACKED_FILES with loaded waveform file
                if let Some(loaded_file) = hierarchy.files.first() {
                    crate::state::update_tracked_file_state(&file_id, shared::FileState::Loaded(loaded_file.clone()));
                    
                    // NEW: Immediately attempt per-file scope restoration
                    // This enables variable display as soon as each individual file loads
                    restore_scope_selection_for_file(loaded_file);
                }
                
                // Also maintain legacy LOADED_FILES for backward compatibility during transition
                for file in hierarchy.files {
                    LOADED_FILES.lock_mut().push_cloned(file.clone());
                    
                    // Store scope selection for later restoration (don't restore immediately)
                    // This prevents multiple files from fighting over global selection during loading
                }
                
                // Mark file as completed in legacy loading system
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
                
                // Update TRACKED_FILES with error state
                let file_error = shared::FileError::ParseError(error.clone());
                crate::state::update_tracked_file_state(&file_id, shared::FileState::Failed(file_error));
                
                // Find the filename for the error alert from TRACKED_FILES (more reliable for non-existent files)
                let filename = {
                    let tracked_files = crate::state::TRACKED_FILES.lock_ref();
                    tracked_files.iter()
                        .find(|file| file.id == file_id)
                        .map(|file| file.filename.clone())
                        .unwrap_or_else(|| {
                            // Fallback to legacy system if not found in TRACKED_FILES
                            let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                            current_files.iter()
                                .find(|file| file.file_id == file_id)
                                .map(|file| file.filename.clone())
                                .unwrap_or_else(|| "Unknown file".to_string())
                        })
                };
                
                // Create and display error alert
                let error_alert = ErrorAlert::new_file_parsing_error(
                    file_id.clone(),
                    filename,
                    error.clone()
                );
                add_error_alert(error_alert);
                
                // Also mark file as error in legacy loading system
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
            DownMsg::BatchDirectoryContents { results } => {
                // Handle batch directory results by updating cache for each directory
                for (path, result) in results {
                    match result {
                        Ok(items) => {
                            // Update cache with successful directory scan
                            crate::FILE_TREE_CACHE.lock_mut().insert(path.clone(), items);
                            
                            // Clear any previous error for this directory
                            crate::FILE_PICKER_ERROR_CACHE.lock_mut().remove(&path);
                        }
                        Err(error) => {
                            // Handle directory scan error
                            let error_alert = crate::state::ErrorAlert::new_directory_error(path.clone(), error.clone());
                            crate::error_display::add_error_alert(error_alert);
                            crate::FILE_PICKER_ERROR_CACHE.lock_mut().insert(path.clone(), error);
                        }
                    }
                }
                
                // Clear global error (batch operations successful)
                crate::FILE_PICKER_ERROR.set_neq(None);
            }
            DownMsg::SignalValues { file_path, results } => {
                // Process signal values from backend
                // Update signal values in the global state
                let mut signal_values = crate::state::SIGNAL_VALUES.lock_mut();
                
                for result in results {
                    // Create unique_id for signal value storage
                    // Create unique_id in the same format as SelectedVariable: file_path|scope_path|variable_name
                    let unique_id = format!("{}|{}|{}", 
                        file_path,
                        result.scope_path,
                        result.variable_name
                    );
                    
                    // Create MultiFormatValue from raw binary value
                    let raw_binary = result.raw_value
                        .unwrap_or_else(|| "Loading...".to_string());
                    
                    let multi_format_value = crate::format_utils::MultiFormatValue::new(raw_binary);
                    
                    // Store multi-format signal value with unique identifier
                    signal_values.insert(unique_id, multi_format_value);
                }
            }
            DownMsg::SignalValuesError { file_path: _, error: _ } => {
                // Show error alert for signal value query failure  
                // Signal value query error logged to console
            }
            DownMsg::SignalTransitions { file_path, results } => {
                // Process real signal transitions from backend - UPDATE CACHE
                for result in results {
                    let cache_key = format!("{}|{}|{}", file_path, result.scope_path, result.variable_name);
                    
                    
                    // Store real backend data in canvas cache
                    crate::waveform_canvas::SIGNAL_TRANSITIONS_CACHE.lock_mut()
                        .insert(cache_key, result.transitions);
                }
                
                // Trigger canvas redraw to show real data
                crate::waveform_canvas::trigger_canvas_redraw();
            }
            DownMsg::SignalTransitionsError { file_path: _, error: _ } => {
                // TODO: Handle signal transitions error for future use
                // Currently using static data in canvas, will integrate later
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