use zoon::*;
use futures::stream::StreamExt;
use crate::error_display::{log_error_console_only, ErrorAlert};
use crate::tracked_files::update_tracked_file_state;
use crate::selected_variables::find_scope_full_name;
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
                update_tracked_file_state(
                    &file_id,
                    shared::FileState::Loading(shared::LoadingStatus::Parsing),
                    tracked_files,
                );

                tracked_files
                    .loading_started_relay
                    .send((file_id.clone(), filename.clone()));
            }
            DownMsg::ParsingProgress { file_id, progress } => {
                tracked_files.parsing_progress_relay.send((
                    file_id,
                    progress,
                    LoadingStatus::Parsing,
                ));
            }
            DownMsg::FileLoaded { file_id, hierarchy } => {
                if let Some(loaded_file) = hierarchy.files.first() {
                    update_tracked_file_state(
                        &file_id,
                        shared::FileState::Loaded(loaded_file.clone()),
                        tracked_files,
                    );

                }

            }
            DownMsg::ParsingError { file_id, error } => {
                let file_error = shared::FileError::ParseError {
                    source: error.clone(),
                    context: format!("Parsing file with ID: {}", file_id),
                };
                update_tracked_file_state(
                    &file_id,
                    shared::FileState::Failed(file_error),
                    tracked_files,
                );

                let filename = {
                    "Unknown file".to_string()
                };

                let error_alert =
                    ErrorAlert::new_file_parsing_error(file_id.clone(), filename, error.clone());
                crate::error_display::log_error_console_only(error_alert);

            }
            DownMsg::DirectoryContents { path, items } => {
                let cache_mutable = zoon::Mutable::new(std::collections::HashMap::new());
                cache_mutable.lock_mut().insert(path.clone(), items.clone());

                if path.contains("/home/") || path.starts_with("/Users/") {
                    let mut paths_to_expand = Vec::new();

                    paths_to_expand.push(path.clone());

                    let mut parent_path = std::path::Path::new(&path);
                    while let Some(parent) = parent_path.parent() {
                        let parent_str = parent.to_string_lossy().to_string();
                        if parent_str == "" || parent_str == "/" {
                            break;
                        }
                        paths_to_expand.push(parent_str);
                        parent_path = parent;
                    }

                    let mut expanded = app_config
                        .file_picker_expanded_directories
                        .lock_mut();
                    for path in paths_to_expand {
                        expanded.insert(path);
                    }
                }

            }
            DownMsg::DirectoryError { path, error } => {
                let error_alert = ErrorAlert::new_directory_error(path.clone(), error.clone());
                log_error_console_only(error_alert);

                log_error_console_only(ErrorAlert::new_directory_error(path.clone(), error));

            }
            DownMsg::ConfigLoaded(_config) => {
            }
            DownMsg::ConfigSaved => {
            }
            DownMsg::ConfigError(_error) => {
            }
            DownMsg::BatchDirectoryContents { results } => {
                for (path, result) in results {
                    match result {
                        Ok(items) => {
                            let cache_mutable =
                                zoon::Mutable::new(std::collections::HashMap::new());
                            cache_mutable.lock_mut().insert(path.clone(), items);

                        }
                        Err(error) => {
                                        let error_alert = ErrorAlert::new_directory_error(
                                path.clone(),
                                error.clone(),
                            );
                            log_error_console_only(error_alert);

                            log_error_console_only(ErrorAlert::new_directory_error(
                                path.clone(),
                                error,
                            ));
                        }
                    }
                }

            }
            DownMsg::SignalTransitions { file_path, results } => {
                for result in results {
                    let cache_key = format!(
                        "{}|{}|{}",
                        file_path, result.scope_path, result.variable_name
                    );

                    crate::visualizer::timeline::timeline_actor::insert_raw_transitions_to_cache(
                        waveform_timeline,
                        cache_key.clone(),
                        result.transitions,
                    );

                }

                crate::visualizer::canvas::waveform_canvas::trigger_canvas_redraw_global();
            }
            DownMsg::SignalTransitionsError {
                file_path: _,
                error: _,
            } => {
            }
            DownMsg::BatchSignalValues {
                batch_id: _,
                file_results,
            } => {
                let mut batch_signal_values = std::collections::HashMap::new();

                for file_result in file_results {
                    for result in file_result.results {
                        let unique_id = format!(
                            "{}|{}|{}",
                            file_result.file_path, result.scope_path, result.variable_name
                        );

                        let _cursor_time = Some(0.0);
                        let within_time_range = true;

                        let signal_value = if within_time_range {
                            if let Some(raw_binary) = result.raw_value {
                                shared::SignalValue::from_data(raw_binary)
                            } else {
                                shared::SignalValue::missing()
                            }
                        } else {
                            shared::SignalValue::missing()
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
                for (_signal_id, value) in &cursor_values {
                    match value {
                        shared::SignalValue::Present(_data) => {}
                        shared::SignalValue::Missing => {}
                        shared::SignalValue::Loading => {}
                    }
                }

                crate::visualizer::timeline::timeline_actor::handle_unified_response(
                    waveform_timeline,
                    request_id,
                    signal_data,
                    cursor_values,
                    statistics,
                );
            }
            DownMsg::UnifiedSignalError { request_id, error } => {
                crate::visualizer::timeline::timeline_actor::handle_unified_error(
                    waveform_timeline,
                    request_id,
                    error,
                );
            }

        DownMsg::SignalValues { file_path, results } => {
            zoon::println!("📨 Received SignalValues for {} with {} results", file_path, results.len());
        }

        DownMsg::SignalValuesError { file_path, error } => {
            zoon::println!("❌ SignalValuesError for {}: {}", file_path, error);
        }
    }
}
