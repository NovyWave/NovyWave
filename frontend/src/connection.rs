use zoon::*;
use futures::stream::StreamExt;
use crate::error_display::log_error_console_only;
use crate::state::ErrorAlert;
use crate::views::is_cursor_within_variable_time_range;
use shared::LoadingStatus;
use shared::{DownMsg, UpMsg};

/// Actor+Relay compatible Connection adapter
pub struct ConnectionAdapter {
    connection: Connection<UpMsg, DownMsg>,
}

impl ConnectionAdapter {
    pub fn new() -> (Self, impl futures::stream::Stream<Item = DownMsg>) {
        let (message_sender, message_stream) = futures::channel::mpsc::unbounded();
        
        let connection = Connection::new(move |down_msg, _| {
            // Simple forwarding - no business logic here
            let _ = message_sender.unbounded_send(down_msg);
        });
        
        let adapter = ConnectionAdapter { connection };
        (adapter, message_stream)
    }
    
    pub async fn send_up_msg(&self, up_msg: UpMsg) {
        if let Err(error) = self.connection.send_up_msg(up_msg).await {
            zoon::println!("Failed to send message: {:?}", error);
        }
    }
}

/// Create message processor that handles DownMsg with domain access
pub fn create_connection_message_handler(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) -> (ConnectionAdapter, crate::dataflow::Actor<()>) {
    let (connection_adapter, mut down_msg_stream) = ConnectionAdapter::new();
    
    let tracked_files = tracked_files.clone();
    let selected_variables = selected_variables.clone();
    let waveform_timeline = waveform_timeline.clone();
    let app_config = app_config.clone();
    
    let message_handler = crate::dataflow::Actor::new((), async move |_state| {
        while let Some(down_msg) = down_msg_stream.next().await {
            handle_down_msg(down_msg, &tracked_files, &selected_variables, &waveform_timeline, &app_config);
        }
    });
    
    (connection_adapter, message_handler)
}

/// Handle incoming DownMsg with proper domain access
fn handle_down_msg(
    down_msg: DownMsg,
    tracked_files: &crate::tracked_files::TrackedFiles,
    _selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) {
    match down_msg {
        DownMsg::ParsingStarted { file_id, filename } => {
                // Update TRACKED_FILES with parsing started status
                crate::state::update_tracked_file_state(
                    &file_id,
                    shared::FileState::Loading(shared::LoadingStatus::Parsing),
                    tracked_files,
                );

                // TODO: Pass TrackedFiles instance as parameter
                tracked_files
                    .loading_started_relay
                    .send((file_id.clone(), filename.clone()));
            }
            DownMsg::ParsingProgress { file_id, progress } => {
                // TODO: Pass TrackedFiles instance as parameter
                tracked_files.parsing_progress_relay.send((
                    file_id,
                    progress,
                    LoadingStatus::Parsing,
                ));
            }
            DownMsg::FileLoaded { file_id, hierarchy } => {
                // Update TRACKED_FILES with loaded waveform file
                if let Some(loaded_file) = hierarchy.files.first() {
                    crate::state::update_tracked_file_state(
                        &file_id,
                        shared::FileState::Loaded(loaded_file.clone()),
                        tracked_files,
                    );

                    // Scope restoration handled by SelectedVariables Actor when variables are selected

                    // Timeline bounds calculation now handled by MaximumTimelineRange Actor
                    // The Actor automatically computes bounds from tracked files and selected variables
                    // No manual bounds calculation needed here
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
                crate::state::update_tracked_file_state(
                    &file_id,
                    shared::FileState::Failed(file_error),
                    tracked_files,
                );

                let filename = {
                    // Use TrackedFiles instance parameter instead of global
                    // For now, use fallback since TrackedFiles API needs implementation
                    "Unknown file".to_string()
                };

                // Create and display error alert
                let error_alert =
                    ErrorAlert::new_file_parsing_error(file_id.clone(), filename, error.clone());
                crate::error_display::log_error_console_only(error_alert);

                // The TrackedFiles Actor will manage loading completion automatically
            }
            DownMsg::DirectoryContents { path, items } => {
                // Cache directory contents → Use DialogManager domain
                // File tree cache simplified - no complex enterprise manager needed
                let cache_mutable = zoon::Mutable::new(std::collections::HashMap::new());
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
                    // Directory expansion handled directly by app_config parameter
                    let mut expanded = app_config
                        .file_picker_expanded_directories
                        .lock_mut();
                    for path in paths_to_expand {
                        expanded.insert(path);
                    }
                }

                // Clear any previous error for this directory (fresh data overwrites cached errors)
                // Error clearing simplified - no enterprise manager needed
                // Error clearing simplified - no complex manager needed
            }
            DownMsg::DirectoryError { path, error } => {
                // Log to console for debugging but don't show toast (UX redundancy)
                let error_alert = ErrorAlert::new_directory_error(path.clone(), error.clone());
                log_error_console_only(error_alert);

                // Store error for UI display in dialog tree
                // Error reporting simplified - log to console for debugging
                log_error_console_only(ErrorAlert::new_directory_error(path.clone(), error));

                // Clear global error (we now use per-directory errors)
                // Error clearing simplified - no enterprise manager needed
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
                            // File tree cache simplified - no complex enterprise manager needed
                            let cache_mutable =
                                zoon::Mutable::new(std::collections::HashMap::new());
                            cache_mutable.lock_mut().insert(path.clone(), items);

                            // Clear any previous error for this directory
                            // Error clearing simplified - no complex manager needed
                        }
                        Err(error) => {
                            // Log to console for debugging but don't show toast (UX redundancy)
                            let error_alert = crate::state::ErrorAlert::new_directory_error(
                                path.clone(),
                                error.clone(),
                            );
                            log_error_console_only(error_alert);

                            // Store directory scan error for UI display
                            // Error reporting simplified - log to console for debugging
                            log_error_console_only(ErrorAlert::new_directory_error(
                                path.clone(),
                                error,
                            ));
                        }
                    }
                }

                // Clear global error (batch operations successful)
                // Error clearing simplified - no enterprise manager needed
            }
            DownMsg::SignalTransitions { file_path, results } => {
                // Process signal transitions from backend - UPDATE CACHE
                for result in results {
                    let cache_key = format!(
                        "{}|{}|{}",
                        file_path, result.scope_path, result.variable_name
                    );

                    // Store backend data in unified cache using Actor+Relay pattern
                    crate::visualizer::timeline::timeline_actor::insert_raw_transitions_to_cache(
                        waveform_timeline,
                        cache_key.clone(),
                        result.transitions,
                    );

                    // Don't clear processed cache - data hasn't changed, just updated
                    // Processed cache will remain valid for existing time ranges
                }

                // Trigger canvas redraw when data arrives
                crate::visualizer::canvas::waveform_canvas::trigger_canvas_redraw_global();
            }
            DownMsg::SignalTransitionsError {
                file_path: _,
                error: _,
            } => {
                // Currently using static data in canvas, will integrate later
            }
            DownMsg::BatchSignalValues {
                batch_id: _,
                file_results,
            } => {
                // Process batch signal values from backend using domain relay
                let mut batch_signal_values = std::collections::HashMap::new();

                for file_result in file_results {
                    for result in file_result.results {
                        let unique_id = format!(
                            "{}|{}|{}",
                            file_result.file_path, result.scope_path, result.variable_name
                        );

                        // Check if cursor time is within this variable's file time range
                        let _cursor_time = Some(0.0); // Fallback to avoid deprecated function
                        // TODO: Pass tracked_files parameter when CONNECTION has domain access
                        let within_time_range = true; // Temporary: assume within range until domain integration

                        let signal_value = if within_time_range {
                            if let Some(raw_binary) = result.raw_value {
                                shared::SignalValue::from_data(raw_binary)
                            } else {
                                shared::SignalValue::missing()
                            }
                        } else {
                            shared::SignalValue::missing() // Beyond time range
                        };
                        batch_signal_values.insert(unique_id, signal_value);
                    }
                }

                if !batch_signal_values.is_empty() {
                    crate::visualizer::timeline::timeline_actor::signal_values_updated_relay(waveform_timeline)
                        .send(batch_signal_values);
                }
            }
            DownMsg::UnifiedSignalResponse {
                request_id,
                signal_data,
                cursor_values,
                statistics,
                cached_time_range_ns: _,
            } => {
                // Log cursor values for debugging
                for (_signal_id, value) in &cursor_values {
                    match value {
                        shared::SignalValue::Present(_data) => {}
                        shared::SignalValue::Missing => {}
                        shared::SignalValue::Loading => {}
                    }
                }

                // Handle unified signal response through the unified timeline service
                crate::visualizer::timeline::timeline_actor::handle_unified_response(
                    waveform_timeline,
                    request_id,
                    signal_data,
                    cursor_values,
                    statistics,
                );
            }
            DownMsg::UnifiedSignalError { request_id, error } => {
                // Handle unified signal error through the unified timeline service
                crate::visualizer::timeline::timeline_actor::handle_unified_error(
                    waveform_timeline,
                    request_id,
                    error,
                );
            }

        DownMsg::SignalValues { file_path, results } => {
            zoon::println!("📨 Received SignalValues for {} with {} results", file_path, results.len());
            // TODO: Implement proper handling or convert to unified format
        }

        DownMsg::SignalValuesError { file_path, error } => {
            zoon::println!("❌ SignalValuesError for {}: {}", file_path, error);
            // TODO: Implement proper error handling
        }
    }
}
