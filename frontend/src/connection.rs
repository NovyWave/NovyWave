use zoon::*;
use crate::{LOADING_FILES, LOADED_FILES, check_loading_complete, config};
use crate::config::CONFIG_LOADED;
use crate::error_display::add_error_alert;
use crate::state::ErrorAlert;
use crate::utils::restore_scope_selection_for_file;
use crate::views::is_cursor_within_variable_time_range;
use crate::state::TIMELINE_CURSOR_NS;
use shared::{UpMsg, DownMsg};
use shared::{LoadingFile, LoadingStatus};
use wasm_bindgen::JsValue;
use js_sys::Reflect;

// Tauri environment detection
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
    crate::debug_utils::debug_conditional("CONNECTION: Initializing with standard port 8080");
    
    // DEBUG: Log environment and connection details
    if is_tauri_environment() {
        crate::debug_utils::debug_conditional("CONNECTION: Running in Tauri environment - SSE may fail due to protocol mismatch");
        crate::debug_utils::debug_conditional("CONNECTION: Tauri origin is likely 'tauri://localhost', backend is 'http://localhost:8080'");
    } else {
        crate::debug_utils::debug_conditional("CONNECTION: Running in web environment - standard SSE should work");
    }
    
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
                crate::debug_utils::debug_conditional("RECEIVED ConfigLoaded message");
                crate::config::apply_config(config);
                crate::debug_utils::debug_conditional("Applied config successfully");
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
                // DEPRECATED: Legacy signal values handler - should be migrated to UnifiedSignalResponse
                // This handler exists for backward compatibility during the migration to SignalDataService
                
                // Convert old format to cursor values for SignalDataService compatibility
                let mut cursor_values = std::collections::BTreeMap::new();
                
                for result in results {
                    let unique_id = format!("{}|{}|{}", file_path, result.scope_path, result.variable_name);
                    
                    // Check if cursor time is within this variable's file time range
                    let cursor_time = TIMELINE_CURSOR_NS.get().to_seconds();
                    let within_time_range = is_cursor_within_variable_time_range(&unique_id, cursor_time);
                    
                    // Convert to SignalDataService format (shared::SignalValue, not format_utils::SignalValue)
                    let signal_value = if within_time_range {
                        if let Some(raw_binary) = result.raw_value {
                            shared::SignalValue::Present(raw_binary)
                        } else {
                            shared::SignalValue::Missing
                        }
                    } else {
                        shared::SignalValue::Missing
                    };
                    
                    cursor_values.insert(unique_id, signal_value);
                }
                
                // Delegate to SignalDataService for consistent handling
                // TODO: Remove this legacy handler once all queries use UnifiedSignalRequest
                if !cursor_values.is_empty() {
                    // Simulate unified response format for SignalDataService
                    let fake_request_id = format!("legacy-{}-{}", file_path, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                    let statistics = shared::SignalStatistics {
                        total_signals: cursor_values.len(),
                        cached_signals: 0, // Legacy queries are not cached
                        query_time_ms: 0,
                        cache_hit_ratio: 0.0, // No cache hits for legacy queries
                    };
                    crate::signal_data_service::SignalDataService::handle_unified_response(
                        fake_request_id.to_string(), 
                        Vec::new(), // No signal data in legacy format
                        cursor_values, 
                        Some(statistics)
                    );
                }
            }
            DownMsg::SignalValuesError { file_path, error } => {
                // DEPRECATED: Legacy signal values error handler - should be migrated to UnifiedSignalError
                
                // Delegate to SignalDataService for consistent error handling
                let fake_request_id = format!("legacy-error-{}-{}", file_path, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                crate::signal_data_service::SignalDataService::handle_unified_error(fake_request_id.to_string(), error);
            }
            DownMsg::SignalTransitions { file_path, results } => {
                crate::debug_utils::debug_signal_transitions(&format!("Received {} transitions for {}", results.len(), file_path));
                
                // Process signal transitions from backend - UPDATE CACHE
                for result in results {
                    let cache_key = format!("{}|{}|{}", file_path, result.scope_path, result.variable_name);
                    
                    // zoon::println!("=== INSERTING TO CACHE: {} with {} transitions ===", cache_key, result.transitions.len());
                    
                    // Store backend data in cache
                    crate::waveform_canvas::SIGNAL_TRANSITIONS_CACHE.lock_mut()
                        .insert(cache_key.clone(), result.transitions);
                    
                    // Don't clear processed cache - data hasn't changed, just updated
                    // Processed cache will remain valid for existing time ranges
                }
                
                // Trigger canvas redraw when data arrives
                crate::waveform_canvas::trigger_canvas_redraw();
            }
            DownMsg::SignalTransitionsError { file_path: _, error: _ } => {
                // TODO: Handle signal transitions error for future use
                // Currently using static data in canvas, will integrate later
            }
            DownMsg::BatchSignalValues { batch_id: _, file_results } => {
                // Process batch signal values from backend (first handler)
                for file_result in file_results {
                    let mut signal_values = crate::state::SIGNAL_VALUES.lock_mut();
                    
                    for result in file_result.results {
                        let unique_id = format!("{}|{}|{}", 
                            file_result.file_path,
                            result.scope_path,
                            result.variable_name
                        );
                        
                        // Check if cursor time is within this variable's file time range
                        let cursor_time = TIMELINE_CURSOR_NS.get().to_seconds();
                        let within_time_range = is_cursor_within_variable_time_range(&unique_id, cursor_time);
                        
                        let signal_value = if within_time_range {
                            if let Some(raw_binary) = result.raw_value {
                                crate::format_utils::SignalValue::from_data(raw_binary)
                            } else {
                                crate::format_utils::SignalValue::missing()
                            }
                        } else {
                            crate::format_utils::SignalValue::missing()  // Beyond time range
                        };
                        signal_values.insert(unique_id, signal_value);
                    }
                }
            }
            DownMsg::UnifiedSignalResponse { request_id, signal_data, cursor_values, statistics, cached_time_range: _ } => {
                // Clone data for dual handling during transition period
                let request_id_legacy = request_id.clone();
                let signal_data_legacy = signal_data.clone();
                let cursor_values_legacy = cursor_values.clone();
                let statistics_legacy = statistics.clone();
                
                // Handle unified signal response through the NEW unified timeline service
                crate::unified_timeline_service::UnifiedTimelineService::handle_unified_response(request_id, signal_data, cursor_values, statistics);
                
                // LEGACY: Also handle through old signal data service for compatibility during transition
                crate::signal_data_service::SignalDataService::handle_unified_response(request_id_legacy, signal_data_legacy, cursor_values_legacy, statistics_legacy);
            }
            DownMsg::UnifiedSignalError { request_id, error } => {
                // Handle unified signal error through the NEW unified timeline service
                crate::unified_timeline_service::UnifiedTimelineService::handle_unified_error(request_id.clone(), error.clone());
                
                // LEGACY: Also handle through old signal data service for compatibility
                crate::signal_data_service::SignalDataService::handle_unified_error(request_id, error);
            }
        }
    })
});

pub fn send_up_msg(up_msg: UpMsg) {
    Task::start(async move {
        // DEBUG: Log message sending attempt
        // DEBUG: Log message sending attempt (commented out to reduce noise)
        // zoon::println!("=== SEND_UP_MSG: Attempting to send {:?} ===", std::mem::discriminant(&up_msg));
        
        // Use the raw MoonZoon connection directly to avoid infinite recursion
        match CONNECTION.send_up_msg(up_msg).await {
            Ok(_) => {
                // zoon::println!("=== SEND_UP_MSG: Message sent successfully via raw connection ===");
            }
            Err(error) => {
                zoon::println!("ERROR: Raw connection send error - {:?}", error);
                
                // Create and display connection error alert
                let error_alert = ErrorAlert::new_connection_error(format!("Connection failed: {}", error));
                add_error_alert(error_alert);
            }
        }
    });
}


pub fn init_connection() {
    crate::debug_utils::debug_conditional("Connection init starting");
    
    // Initialize platform-specific connection handling
    // Platform-specific initialization would go here if needed
    #[cfg(NOVYWAVE_PLATFORM = "TAURI")]
    {
        crate::debug_utils::debug_conditional("Tauri platform integration not yet implemented");
    }
    
    // For web mode, CONNECTION static already handles messages - no additional setup needed
    #[cfg(NOVYWAVE_PLATFORM = "WEB")]
    {
        crate::debug_utils::debug_conditional("Web mode: using CONNECTION static for message handling");
        // CONNECTION is automatically initialized when first accessed
    }
}


