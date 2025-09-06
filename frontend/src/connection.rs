use zoon::*;
// Removed legacy import check_loading_complete - migration complete
use crate::error_display::log_error_console_only;
use crate::state::ErrorAlert;
use crate::views::is_cursor_within_variable_time_range;
use crate::actors::dialog_manager::{set_file_error};
use shared::{UpMsg, DownMsg};
use shared::LoadingStatus;





pub(crate) static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    // TEMPORARY: Both web and Tauri use port 8080 for easier testing
    
    
    Connection::new(|down_msg, _| {
        // DownMsg logging disabled - causes CLI overflow with large files
        match down_msg {
            DownMsg::ParsingStarted { file_id, filename } => {
                // Update TRACKED_FILES with parsing started status
                crate::state::update_tracked_file_state(&file_id, shared::FileState::Loading(shared::LoadingStatus::Parsing));
                
                let tracked_files = crate::actors::global_domains::tracked_files_domain();
                tracked_files.loading_started_relay.send((file_id.clone(), filename.clone()));
            }
            DownMsg::ParsingProgress { file_id, progress } => {
                let tracked_files = crate::actors::global_domains::tracked_files_domain();
                tracked_files.parsing_progress_relay.send((file_id, progress, LoadingStatus::Parsing));
            }
            DownMsg::FileLoaded { file_id, hierarchy } => {
                // Update TRACKED_FILES with loaded waveform file
                if let Some(loaded_file) = hierarchy.files.first() {
                    crate::state::update_tracked_file_state(&file_id, shared::FileState::Loaded(loaded_file.clone()));
                    
                    // Scope restoration handled by SelectedVariables Actor when variables are selected
                    
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
                
                // The TrackedFiles Actor manages all loaded file state and handles scope restoration
                
                // The TrackedFiles Actor automatically manages loading completion business logic
                
                // Config automatically saved by ConfigSaver watching domain signals
            }
            DownMsg::ParsingError { file_id, error } => {
                
                // Update TRACKED_FILES with error state
                let file_error = shared::FileError::ParseError { 
                    source: error.clone(),
                    context: format!("Parsing file with ID: {}", file_id),
                };
                crate::state::update_tracked_file_state(&file_id, shared::FileState::Failed(file_error));
                
                let filename = {
                    let tracked_files_domain = crate::actors::global_domains::tracked_files_domain();
                    let current_files = tracked_files_domain.get_current_files();
                    current_files.iter()
                        .find(|file| file.id == file_id)
                        .map(|file| file.filename.clone())
                        .unwrap_or_else(|| "Unknown file".to_string())
                };
                
                // Create and display error alert
                let error_alert = ErrorAlert::new_file_parsing_error(
                    file_id.clone(),
                    filename,
                    error.clone()
                );
                crate::error_display::log_error_console_only(error_alert);
                
                // The TrackedFiles Actor will manage loading completion automatically
            }
            DownMsg::DirectoryContents { path, items } => {
                // Cache directory contents → Use DialogManager domain
                let cache_mutable = crate::actors::dialog_manager::get_file_tree_cache_mutable();
                cache_mutable.lock_mut().insert(path.clone(), items.clone());
                
                // Auto-expand home directory path and its parent directories
                if path.contains("/home/") || path.starts_with("/Users/") {
                    let mut paths_to_expand = Vec::new();
                    
                    // Expand the home directory itself
                    paths_to_expand.push(path.clone());
                    
                    // Only expand parent directories, don't browse them automatically
                    // This prevents infinite loops
                    let mut parent_path = std::path::Path::new(&path);
                    while let Some(parent) = parent_path.parent() {
                        let parent_str = parent.to_string_lossy().to_string();
                        if parent_str == "" || parent_str == "/" {
                            break;
                        }
                        paths_to_expand.push(parent_str);
                        parent_path = parent;
                    }
                    
                    // Use domain function for bulk directory expansion
                    crate::actors::dialog_manager::insert_expanded_directories(paths_to_expand);
                }
                
                // Clear any previous error for this directory (fresh data overwrites cached errors)
                set_file_error(None);
                crate::actors::dialog_manager::clear_file_error(Some(path.clone()));
            }
            DownMsg::DirectoryError { path, error } => {
                // Log to console for debugging but don't show toast (UX redundancy)
                let error_alert = ErrorAlert::new_directory_error(path.clone(), error.clone());
                log_error_console_only(error_alert);
                
                // Store error for UI display in dialog tree
                crate::actors::dialog_manager::report_file_error(path.clone(), error);
                
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
                            // Update cache with successful directory scan → Use DialogManager domain
                            let cache_mutable = crate::actors::dialog_manager::get_file_tree_cache_mutable();
                            cache_mutable.lock_mut().insert(path.clone(), items);
                            
                            // Clear any previous error for this directory
                            crate::actors::dialog_manager::clear_file_error(Some(path.clone()));
                        }
                        Err(error) => {
                            // Log to console for debugging but don't show toast (UX redundancy)
                            let error_alert = crate::state::ErrorAlert::new_directory_error(path.clone(), error.clone());
                            log_error_console_only(error_alert);
                            
                            // Store directory scan error for UI display 
                            crate::actors::dialog_manager::report_file_error(path.clone(), error);
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
                    
                    
                    // Store backend data in unified cache using Actor+Relay pattern
                    crate::visualizer::timeline::timeline_actor::insert_raw_transitions_to_cache(
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
                // Currently using static data in canvas, will integrate later
            }
            DownMsg::BatchSignalValues { batch_id: _, file_results } => {
                // Process batch signal values from backend using domain relay
                let mut batch_signal_values = std::collections::HashMap::new();
                
                for file_result in file_results {
                    for result in file_result.results {
                        let unique_id = format!("{}|{}|{}", 
                            file_result.file_path,
                            result.scope_path,
                            result.variable_name
                        );
                        
                        // Check if cursor time is within this variable's file time range
                        let cursor_time = Some(0.0); // Fallback to avoid deprecated function
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
                        batch_signal_values.insert(unique_id, signal_value);
                    }
                }
                
                if !batch_signal_values.is_empty() {
                    crate::visualizer::timeline::timeline_actor::signal_values_updated_relay()
                        .send(batch_signal_values);
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
                crate::visualizer::timeline::timeline_actor::handle_unified_response(request_id, signal_data, cursor_values, statistics);
            }
            DownMsg::UnifiedSignalError { request_id, error } => {
                // Handle unified signal error through the unified timeline service
                crate::visualizer::timeline::timeline_actor::handle_unified_error(request_id, error);
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
                crate::error_display::log_error_console_only(error_alert);
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

