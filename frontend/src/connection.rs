use zoon::*;
use crate::{LOADING_FILES, LOADED_FILES, check_loading_complete};
use crate::error_display::{add_error_alert, log_error_console_only};
use crate::state::ErrorAlert;
use crate::utils::restore_scope_selection_for_file;
use crate::views::is_cursor_within_variable_time_range;
use crate::visualizer::timeline::timeline_actor::current_cursor_position_seconds;
use crate::actors::dialog_manager::{set_file_error};
use shared::{UpMsg, DownMsg};
use shared::{LoadingFile, LoadingStatus};
use wasm_bindgen::JsValue;
use js_sys::Reflect;

// Tauri environment detection
#[allow(dead_code)]
fn is_tauri_environment() -> bool {
    if let Ok(global) = js_sys::global().dyn_into::<web_sys::Window>() {
        // Check if __TAURI__ exists in the global scope
        Reflect::has(&global, &JsValue::from_str("__TAURI__"))
            .unwrap_or(false)
    } else {
        false
    }
}






pub(crate) static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    // TEMPORARY: Both web and Tauri use port 8080 for easier testing
    // TODO: Implement proper dynamic port detection for Tauri
    
    
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
                    
                    // ✅ FIX: Send timeline bounds to WaveformTimeline Actor when file loads
                    // Use get_maximum_timeline_range() to get actual file data, not current viewport
                    if let Some((min_time, max_time)) = crate::visualizer::canvas::waveform_canvas::get_maximum_timeline_range() {
                        crate::visualizer::timeline::timeline_actor::timeline_bounds_calculated_relay()
                            .send((min_time, max_time));
                    } else {
                        // Force timeline bounds calculation even when no variables selected
                        let (file_min, file_max) = crate::visualizer::canvas::waveform_canvas::get_full_file_range();
                        if file_min < file_max && file_min.is_finite() && file_max.is_finite() {
                            crate::visualizer::timeline::timeline_actor::timeline_bounds_calculated_relay()
                                .send((file_min, file_max));
                        } else {
                        }
                    }
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
                
                // Config automatically saved by ConfigSaver watching domain signals
            }
            DownMsg::ParsingError { file_id, error } => {
                
                // Update TRACKED_FILES with error state
                let file_error = shared::FileError::ParseError { 
                    source: error.clone(),
                    context: format!("Parsing file with ID: {}", file_id),
                };
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
                    // TODO: Add domain function for bulk directory expansion
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
                set_file_error(None);
                // TODO: Add domain function for error cache manipulation
                crate::FILE_PICKER_ERROR_CACHE.lock_mut().remove(&path);
            }
            DownMsg::DirectoryError { path, error } => {
                // Log to console for debugging but don't show toast (UX redundancy)
                let error_alert = ErrorAlert::new_directory_error(path.clone(), error.clone());
                log_error_console_only(error_alert);
                
                // Store error for UI display in dialog tree
                // TODO: Add domain function for error cache manipulation
                crate::FILE_PICKER_ERROR_CACHE.lock_mut().insert(path.clone(), error);
                
                // Clear global error (we now use per-directory errors)
                set_file_error(None);
            }
            DownMsg::ConfigLoaded(_config) => {
                // Config response now handled directly by exchange_msgs in load_config_from_backend
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
                            // TODO: Add domain function for error cache manipulation
                            crate::FILE_PICKER_ERROR_CACHE.lock_mut().remove(&path);
                        }
                        Err(error) => {
                            // Log to console for debugging but don't show toast (UX redundancy)
                            let error_alert = crate::state::ErrorAlert::new_directory_error(path.clone(), error.clone());
                            log_error_console_only(error_alert);
                            
                            // Store directory scan error for UI display 
                            // TODO: Add domain function for error cache manipulation
                            crate::FILE_PICKER_ERROR_CACHE.lock_mut().insert(path.clone(), error);
                        }
                    }
                }
                
                // Clear global error (batch operations successful)
                set_file_error(None);
            }
            DownMsg::SignalTransitions { file_path, results } => {
                
                // Process signal transitions from backend - UPDATE CACHE
                for result in results {
                    let cache_key = format!("{}|{}|{}", file_path, result.scope_path, result.variable_name);
                    
                    
                    // Store backend data in unified cache
                    crate::visualizer::timeline::timeline_service::UnifiedTimelineService::insert_raw_transitions(
                        cache_key.clone(), 
                        result.transitions
                    );
                    
                    // Don't clear processed cache - data hasn't changed, just updated
                    // Processed cache will remain valid for existing time ranges
                }
                
                // Trigger canvas redraw when data arrives
                crate::visualizer::canvas::waveform_canvas::trigger_canvas_redraw_global();
            }
            DownMsg::SignalTransitionsError { file_path: _, error: _ } => {
                // TODO: Handle signal transitions error for future use
                // Currently using static data in canvas, will integrate later
            }
            DownMsg::BatchSignalValues { batch_id: _, file_results } => {
                // Process batch signal values from backend (first handler)
                for file_result in file_results {
                    let mut signal_values = crate::visualizer::state::timeline_state::SIGNAL_VALUES.lock_mut();
                    
                    for result in file_result.results {
                        let unique_id = format!("{}|{}|{}", 
                            file_result.file_path,
                            result.scope_path,
                            result.variable_name
                        );
                        
                        // Check if cursor time is within this variable's file time range
                        let cursor_time = current_cursor_position_seconds();
                        let within_time_range = is_cursor_within_variable_time_range(&unique_id, cursor_time.unwrap_or(0.0));
                        
                        let signal_value = if within_time_range {
                            if let Some(raw_binary) = result.raw_value {
                                crate::visualizer::formatting::signal_values::SignalValue::from_data(raw_binary)
                            } else {
                                crate::visualizer::formatting::signal_values::SignalValue::missing()
                            }
                        } else {
                            crate::visualizer::formatting::signal_values::SignalValue::missing()  // Beyond time range
                        };
                        signal_values.insert(unique_id, signal_value);
                    }
                }
            }
            DownMsg::UnifiedSignalResponse { request_id, signal_data, cursor_values, statistics, cached_time_range_ns: _ } => {
                
                // Log cursor values for debugging
                for (_signal_id, value) in &cursor_values {
                    match value {
                        shared::SignalValue::Present(_data) => {
                        },
                        shared::SignalValue::Missing => {
                        }
                    }
                }
                
                // Handle unified signal response through the unified timeline service
                crate::visualizer::timeline::timeline_service::UnifiedTimelineService::handle_unified_response(request_id, signal_data, cursor_values, statistics);
            }
            DownMsg::UnifiedSignalError { request_id, error } => {
                // Handle unified signal error through the unified timeline service
                crate::visualizer::timeline::timeline_service::UnifiedTimelineService::handle_unified_error(request_id, error);
            }
            
            DownMsg::SignalValues { .. } => {
                // Handle legacy SignalValues message (deprecated in favor of UnifiedSignalResponse)
            }
            
            DownMsg::SignalValuesError { .. } => {
                // Handle legacy SignalValuesError message (deprecated in favor of UnifiedSignalError)
            }
        }
    })
});

pub fn send_up_msg(up_msg: UpMsg) {
    Task::start(async move {
        
        // Use the raw MoonZoon connection directly to avoid infinite recursion
        match CONNECTION.send_up_msg(up_msg).await {
            Ok(_) => {
            }
            Err(error) => {
                // Create and display connection error alert
                let error_alert = ErrorAlert::new_connection_error(format!("Connection failed: {}", error));
                add_error_alert(error_alert);
            }
        }
    });
}


pub fn init_connection() {
    
    // Initialize platform-specific connection handling
    // Platform-specific initialization would go here if needed
    #[cfg(NOVYWAVE_PLATFORM = "TAURI")]
    {
    }
    
    // For web mode, CONNECTION static already handles messages - no additional setup needed
    #[cfg(NOVYWAVE_PLATFORM = "WEB")]
    {
        // CONNECTION is automatically initialized when first accessed
    }
}


