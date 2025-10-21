//! NovyWaveApp - Self-contained Actor+Relay Architecture

use futures::StreamExt;
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::SignalVecExt;
use gloo_timers::callback::Interval;
use indexmap::IndexSet;
use moonzoon_novyui::components::treeview::{
    TreeViewItemData, TreeViewItemType, TreeViewSize, TreeViewVariant, tree_view,
};
use moonzoon_novyui::*;
use zoon::RawHtmlEl;
use zoon::events::{Click, KeyDown, KeyUp};
use zoon::events_extra;
use zoon::{EventOptions, *};

use crate::config::AppConfig;
use crate::dataflow::atom::Atom;
use crate::dataflow::{Actor, Relay, relay};
use crate::platform::CurrentPlatform;
use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::WaveformTimeline;
use shared::{DownMsg, SignalStatistics, SignalValue, UnifiedSignalData, UpMsg};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use wasm_bindgen::{JsCast, closure::Closure};

/// Event payload emitted for each `UnifiedSignalResponse` down message.
#[derive(Clone, Debug)]
pub struct UnifiedSignalResponseEvent {
    pub request_id: String,
    pub signal_data: Vec<UnifiedSignalData>,
    pub cursor_values: BTreeMap<String, SignalValue>,
    pub cached_time_range_ns: Option<(u64, u64)>,
    pub statistics: Option<SignalStatistics>,
}

#[derive(Clone, Debug)]
pub struct WorkspaceLoadedEvent {
    pub root: String,
    pub default_root: String,
    pub config: shared::AppConfig,
}

const KEY_REPEAT_INTERVAL_MS: u32 = 55;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum KeyAction {
    CursorLeft,
    CursorRight,
    PanLeft,
    PanRight,
    ZoomIn,
    ZoomOut,
}

/// Actor+Relay replacement for global message routing
/// Transforms DownMsg stream into domain-specific relay streams
#[derive(Clone)]
pub struct ConnectionMessageActor {
    // Message-specific relays that domains can subscribe to
    pub config_loaded_relay: Relay<shared::AppConfig>,
    pub workspace_loaded_relay: Relay<WorkspaceLoadedEvent>,
    pub config_error_relay: Relay<String>,
    pub config_saved_relay: Relay<()>,
    pub directory_contents_relay: Relay<(String, Vec<shared::FileSystemItem>)>,
    pub directory_error_relay: Relay<(String, String)>,
    pub file_loaded_relay: Relay<(String, shared::FileState)>,
    pub parsing_started_relay: Relay<(String, String)>,
    pub batch_signal_values_relay: Relay<Vec<(String, SignalValue)>>,
    pub unified_signal_response_relay: Relay<UnifiedSignalResponseEvent>,
    pub unified_signal_error_relay: Relay<(String, String)>,

    // Actor handles message processing
    _message_actor: Actor<()>,
}

pub(crate) fn emit_trace(target: &str, message: impl Into<String>) {
    let target_string = target.to_string();
    let message_string = message.into();
    Task::start(async move {
        let _ =
            <CurrentPlatform as crate::platform::Platform>::send_message(UpMsg::FrontendTrace {
                target: target_string,
                message: message_string,
            })
            .await;
    });
}

fn start_key_repeat<F>(
    handles: &Rc<RefCell<HashMap<KeyAction, Interval>>>,
    action: KeyAction,
    period_ms: u32,
    callback: F,
) where
    F: FnMut() + 'static,
{
    let mut map = handles.borrow_mut();
    if map.contains_key(&action) {
        return;
    }

    let mut callback = callback;
    let interval = Interval::new(period_ms, move || {
        callback();
    });
    map.insert(action, interval);
}

fn stop_key_repeat(handles: &Rc<RefCell<HashMap<KeyAction, Interval>>>, action: KeyAction) {
    handles.borrow_mut().remove(&action);
}

impl ConnectionMessageActor {
    /// Create ConnectionMessageActor with DownMsg stream from connection
    pub async fn new(
        down_msg_stream: impl futures::stream::Stream<Item = DownMsg> + Unpin + 'static,
        tracked_files_for_reload: TrackedFiles,
    ) -> Self {
        // Create all message-specific relays
        let (config_loaded_relay, _) = relay();
        let (workspace_loaded_relay, _) = relay();
        let (config_error_relay, _) = relay();
        let (config_saved_relay, _) = relay();
        let (directory_contents_relay, _) = relay();
        let (directory_error_relay, _) = relay();
        let (file_loaded_relay, _) = relay();
        let (parsing_started_relay, _) = relay();
        let (batch_signal_values_relay, _) = relay::<Vec<(String, SignalValue)>>();
        let (unified_signal_response_relay, _) = relay::<UnifiedSignalResponseEvent>();
        let (unified_signal_error_relay, _) = relay::<(String, String)>();

        // Clone relays for use in Actor closure
        let config_loaded_relay_clone = config_loaded_relay.clone();
        let workspace_loaded_relay_clone = workspace_loaded_relay.clone();
        let config_error_relay_clone = config_error_relay.clone();
        let config_saved_relay_clone = config_saved_relay.clone();
        let directory_contents_relay_clone = directory_contents_relay.clone();
        let directory_error_relay_clone = directory_error_relay.clone();
        let file_loaded_relay_clone = file_loaded_relay.clone();
        let parsing_started_relay_clone = parsing_started_relay.clone();
        let batch_signal_values_relay_clone = batch_signal_values_relay.clone();
        let unified_signal_response_relay_clone = unified_signal_response_relay.clone();
        let unified_signal_error_relay_clone = unified_signal_error_relay.clone();
        let tracked_files_for_reload = tracked_files_for_reload.clone();

        // Actor processes DownMsg stream and routes to appropriate relays

        let message_actor = Actor::new((), async move |_state| {
            let mut stream = down_msg_stream;
            loop {
                match stream.next().await {
                    Some(down_msg) => {
                        // Route each message type to its specific relay
                        match down_msg {
                            DownMsg::ConfigLoaded(config) => {
                                config_loaded_relay_clone.send(config);
                            }
                            DownMsg::WorkspaceLoaded {
                                root,
                                default_root,
                                config,
                            } => {
                                workspace_loaded_relay_clone.send(WorkspaceLoadedEvent {
                                    root,
                                    default_root,
                                    config,
                                });
                            }
                            DownMsg::ConfigError(error) => {
                                config_error_relay_clone.send(error);
                            }
                            DownMsg::ConfigSaved => {
                                config_saved_relay_clone.send(());
                            }
                            DownMsg::DirectoryContents { path, items } => {
                                directory_contents_relay_clone.send((path, items));
                            }
                            DownMsg::DirectoryError { path, error } => {
                                directory_error_relay_clone.send((path, error));
                            }
                            DownMsg::FileLoaded { file_id, hierarchy } => {
                                if let Some(loaded_file) = hierarchy.files.first() {
                                    file_loaded_relay_clone.send((
                                        file_id,
                                        shared::FileState::Loaded(loaded_file.clone()),
                                    ));
                                }
                            }
                            DownMsg::ParsingError { file_id, error } => {
                                let failed_path = file_id.clone();
                                file_loaded_relay_clone.send((
                                    failed_path.clone(),
                                    shared::FileState::Failed(shared::FileError::IoError {
                                        path: failed_path,
                                        error: error.clone(),
                                    }),
                                ));
                            }
                            DownMsg::ParsingStarted { file_id, filename } => {
                                parsing_started_relay_clone.send((file_id, filename));
                            }
                            DownMsg::BatchSignalValues { file_results, .. } => {
                                let mut values = Vec::new();

                                for file_result in file_results {
                                    for result in file_result.results {
                                        let unique_id = format!(
                                            "{}|{}|{}",
                                            file_result.file_path,
                                            result.scope_path,
                                            result.variable_name,
                                        );

                                        let value = match result.formatted_value {
                                            Some(formatted) => SignalValue::Present(formatted),
                                            None => match result.raw_value {
                                                Some(raw) => SignalValue::from_data(raw),
                                                None => SignalValue::missing(),
                                            },
                                        };

                                        values.push((unique_id, value));
                                    }
                                }

                                if !values.is_empty() {
                                    batch_signal_values_relay_clone.send(values);
                                }
                            }
                            DownMsg::UnifiedSignalResponse {
                                request_id,
                                signal_data,
                                cursor_values,
                                cached_time_range_ns,
                                statistics,
                            } => {
                                unified_signal_response_relay_clone.send(
                                    UnifiedSignalResponseEvent {
                                        request_id,
                                        signal_data,
                                        cursor_values,
                                        cached_time_range_ns,
                                        statistics,
                                    },
                                );
                            }
                            DownMsg::UnifiedSignalError { request_id, error } => {
                                unified_signal_error_relay_clone.send((request_id, error));
                            }
                            DownMsg::ReloadWaveformFiles { file_paths } => {
                                if !file_paths.is_empty() {
                                    tracked_files_for_reload.reload_existing_paths(file_paths);
                                }
                            }
                            DownMsg::OpenWaveformFiles { file_paths } => {
                                if !file_paths.is_empty() {
                                    tracked_files_for_reload.load_new_paths(file_paths);
                                }
                            }
                            _ => {
                                // Other message types can be added as needed
                            }
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        });

        Self {
            config_loaded_relay,
            workspace_loaded_relay,
            config_error_relay,
            config_saved_relay,
            directory_contents_relay,
            directory_error_relay,
            file_loaded_relay,
            parsing_started_relay,
            batch_signal_values_relay,
            unified_signal_response_relay,
            unified_signal_error_relay,
            _message_actor: message_actor,
        }
    }
}

use crate::file_picker::file_paths_dialog;

/// Self-contained NovyWave application
pub struct NovyWaveApp {
    /// File tracking and management domain
    pub tracked_files: TrackedFiles,

    /// Variable selection and scope management domain
    pub selected_variables: SelectedVariables,

    /// Timeline, cursor, and waveform visualization domain
    pub waveform_timeline: WaveformTimeline,

    /// Waveform canvas rendering and state domain
    pub waveform_canvas: crate::visualizer::canvas::waveform_canvas::WaveformCanvas,

    /// Application configuration (theme, panels, etc.)
    pub config: AppConfig,

    /// Panel dragging and resizing system
    pub dragging_system: crate::dragging::DraggingSystem,

    /// Backend communication connection (Arc for cloning)
    pub connection: std::sync::Arc<SendWrapper<Connection<UpMsg, DownMsg>>>,

    /// Dedicated file-picker style domain for workspace selection tree
    pub workspace_picker_domain: crate::config::FilePickerDomain,
    pub workspace_picker_target: Mutable<Option<String>>,
    _workspace_picker_restoring: Atom<bool>,

    /// Message routing actor (keeps relays alive)
    _connection_message_actor: ConnectionMessageActor,
    _workspace_event_actor: Actor<()>,
    _config_loaded_actor: Actor<()>,
    _config_error_actor: Actor<()>,

    /// Synchronizes Files panel scope selection into SelectedVariables domain
    _scope_selection_sync_actor: Actor<()>,

    /// Bridges connection message relays into the timeline domain
    _timeline_message_bridge_actor: Actor<()>,
    _workspace_history_selection_actor: Actor<()>,
    _workspace_history_expanded_actor: Actor<()>,
    _workspace_history_scroll_actor: Actor<()>,
    _workspace_history_restore_actor: Actor<()>,

    // === UI STATE (Atom pattern for local UI concerns) ===
    /// File picker dialog visibility
    pub file_dialog_visible: Atom<bool>,
    pub workspace_picker_visible: Atom<bool>,

    pub workspace_path: Mutable<Option<String>>,
    pub workspace_loading: Mutable<bool>,
    pub default_workspace_path: Mutable<Option<String>>,

    key_repeat_handles: Rc<RefCell<HashMap<KeyAction, Interval>>>,
}

// Remove Default implementation - use new() method instead

impl NovyWaveApp {
    fn propagate_scope_selection(
        selected_ids: Vec<String>,
        selected_variables: &SelectedVariables,
    ) {
        let mut selection_set = IndexSet::new();
        let mut last_scope: Option<String> = None;

        for raw_id in selected_ids.into_iter() {
            if !raw_id.starts_with("scope_") {
                continue;
            }

            let cleaned = raw_id
                .strip_prefix("scope_")
                .unwrap_or(raw_id.as_str())
                .to_string();
            last_scope = Some(cleaned);

            selection_set.insert(raw_id);
        }

        match last_scope {
            Some(cleaned_scope) => {
                selected_variables
                    .scope_selected_relay
                    .send(Some(cleaned_scope));
            }
            None => {
                selected_variables.scope_selected_relay.send(None);
            }
        }

        selected_variables
            .tree_selection_changed_relay
            .send(selection_set);
    }

    /// Create new NovyWaveApp with proper async initialization
    ///
    /// This replaces the complex global domain initialization from main.rs
    pub async fn new() -> Self {
        // init

        // Load fonts first
        Self::load_and_register_fonts().await;

        let tracked_files = TrackedFiles::new().await;
        let selected_variables = SelectedVariables::new().await;

        // ✅ ACTOR+RELAY: Use working connection with message actor integration
        let (connection, connection_message_actor) = Self::create_connection_with_message_actor(
            tracked_files.clone(),
            selected_variables.clone(),
        )
        .await;

        let workspace_path = Mutable::new(None);
        let workspace_loading = Mutable::new(true);
        let default_workspace_path = Mutable::new(None);
        let workspace_picker_visible = Atom::new(false);

        let workspace_event_actor = {
            let mut workspace_stream = connection_message_actor.workspace_loaded_relay.subscribe();
            let workspace_path = workspace_path.clone();
            let workspace_loading = workspace_loading.clone();
            let default_workspace_path = default_workspace_path.clone();

            Actor::new((), async move |_state| {
                while let Some(event) = workspace_stream.next().await {
                    workspace_loading.set(false);
                    workspace_path.set(Some(event.root.clone()));
                    default_workspace_path.set(Some(event.default_root.clone()));
                }
            })
        };

        let config_loaded_actor = {
            let mut config_stream = connection_message_actor.config_loaded_relay.subscribe();
            let workspace_loading = workspace_loading.clone();
            Actor::new((), async move |_state| {
                while config_stream.next().await.is_some() {
                    workspace_loading.set(false);
                }
            })
        };

        let config_error_actor = {
            let mut error_stream = connection_message_actor.config_error_relay.subscribe();
            let workspace_loading = workspace_loading.clone();

            Actor::new((), async move |_state| {
                while let Some(_error) = error_stream.next().await {
                    workspace_loading.set(false);
                }
            })
        };

        // Initialize platform layer with the working connection
        let connection_arc = std::sync::Arc::new(connection);
        crate::platform::set_platform_connection(connection_arc.clone());

        let (workspace_picker_save_relay, mut workspace_picker_save_stream) = relay();
        let workspace_picker_domain = crate::config::FilePickerDomain::new(
            IndexSet::new(),
            0,
            workspace_picker_save_relay.clone(),
            connection_arc.clone(),
            connection_message_actor.clone(),
        )
        .await;

        Task::start(async move { while workspace_picker_save_stream.next().await.is_some() {} });

        // Create main config with proper connection and message routing
        let config = AppConfig::new(
            connection_arc.clone(),
            connection_message_actor.clone(),
            tracked_files.clone(),
            selected_variables.clone(),
        )
        .await;

        let initial_history = config.workspace_history_state.get_cloned();
        let workspace_picker_target = Mutable::new(initial_history.last_selected.clone());
        let workspace_picker_restoring = Atom::new(false);

        let workspace_history_selection_actor = {
            let selected_signal = workspace_picker_domain.selected_files_vec_signal.clone();
            let config_clone = config.clone();
            let domain_clone = workspace_picker_domain.clone();
            let target_clone = workspace_picker_target.clone();
            Actor::new((), async move |_state| {
                let mut selection_stream = selected_signal.signal_cloned().to_stream().fuse();
                while let Some(selection) = selection_stream.next().await {
                    emit_trace("workspace_picker_selection", format!("paths={selection:?}"));
                    if let Some(path) = selection.first().cloned() {
                        target_clone.set_neq(Some(path.clone()));
                        config_clone.record_workspace_selection(&path);

                        let expanded_vec = domain_clone
                            .expanded_directories_actor
                            .state
                            .lock_ref()
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>();
                        let expanded_vec_for_log = expanded_vec.clone();
                        config_clone.update_workspace_tree_state(&path, expanded_vec);

                        let scroll_current =
                            *domain_clone.scroll_position_actor.state.lock_ref() as f64;
                        config_clone.update_workspace_scroll(&path, scroll_current);
                        emit_trace(
                            "workspace_picker_selection",
                            format!(
                                "selection={selection:?} expanded_paths={expanded_vec_for_log:?} scroll={scroll_current}"
                            ),
                        );
                    }
                }
            })
        };

        let workspace_history_expanded_actor = {
            let domain_clone = workspace_picker_domain.clone();
            let target_clone = workspace_picker_target.clone();
            let config_clone = config.clone();
            let restoring_flag = workspace_picker_restoring.clone();
            Actor::new((), async move |_state| {
                let mut expanded_stream = domain_clone
                    .expanded_directories_actor
                    .state
                    .signal_cloned()
                    .to_stream()
                    .fuse();
                let mut last_snapshot: Option<Vec<String>> = None;

                while let Some(snapshot) = expanded_stream.next().await {
                    let expanded_vec = snapshot.iter().cloned().collect::<Vec<String>>();
                    emit_trace(
                        "workspace_picker_expanded_state",
                        format!(
                            "paths={expanded_vec:?} restoring={}",
                            restoring_flag.get_cloned()
                        ),
                    );

                    let is_same = last_snapshot
                        .as_ref()
                        .map(|previous| previous == &expanded_vec)
                        .unwrap_or(false);
                    if is_same {
                        continue;
                    }
                    last_snapshot = Some(expanded_vec.clone());

                    if restoring_flag.get_cloned() {
                        continue;
                    }

                    NovyWaveApp::publish_workspace_picker_snapshot(
                        &config_clone,
                        &domain_clone,
                        &target_clone,
                    );
                }
            })
        };

        let workspace_history_scroll_actor = {
            let domain_clone = workspace_picker_domain.clone();
            let target_clone = workspace_picker_target.clone();
            let config_clone = config.clone();
            let restoring_flag = workspace_picker_restoring.clone();
            Actor::new((), async move |_state| {
                let mut scroll_stream = domain_clone
                    .scroll_position_actor
                    .signal()
                    .to_stream()
                    .fuse();

                while let Some(position) = scroll_stream.next().await {
                    if restoring_flag.get_cloned() {
                        continue;
                    }

                    let scroll_value = position as f64;
                    emit_trace(
                        "workspace_picker_scroll",
                        format!("scroll_top={scroll_value}"),
                    );
                    config_clone.update_workspace_picker_scroll(scroll_value);
                    if let Some(path) = target_clone.lock_ref().clone() {
                        config_clone.update_workspace_scroll(&path, scroll_value);
                    }
                }
            })
        };

        let workspace_history_restore_actor = {
            let history_state = config.workspace_history_state.clone();
            let domain_clone = workspace_picker_domain.clone();
            let visible_atom = workspace_picker_visible.clone();
            let target_clone = workspace_picker_target.clone();
            let config_clone = config.clone();
            let restoring_flag = workspace_picker_restoring.clone();
            Actor::new((), async move |_state| {
                let mut visibility_stream = visible_atom.signal().to_stream().fuse();
                while let Some(visible) = visibility_stream.next().await {
                    if visible {
                        restoring_flag.set(true);
                        let history = history_state.get_cloned();
                        Self::apply_workspace_picker_tree_state(&history, &domain_clone);
                        target_clone.set_neq(None);
                        domain_clone.selected_files_vec_signal.set_neq(Vec::new());
                        domain_clone.clear_selection_relay.send(());
                        Self::publish_workspace_picker_snapshot(
                            &config_clone,
                            &domain_clone,
                            &target_clone,
                        );
                        restoring_flag.set(false);
                    } else {
                        restoring_flag.set(false);
                    }
                }
            })
        };

        // Create MaximumTimelineRange standalone actor for centralized range computation
        let maximum_timeline_range = crate::visualizer::timeline::MaximumTimelineRange::new(
            tracked_files.clone(),
            selected_variables.clone(),
        )
        .await;

        let connection_adapter =
            crate::connection::ConnectionAdapter::from_arc(connection_arc.clone());

        let waveform_timeline = WaveformTimeline::new(
            selected_variables.clone(),
            tracked_files.clone(),
            maximum_timeline_range.clone(),
            connection_adapter,
            config.clone(),
        )
        .await;

        // Initialize waveform canvas rendering domain
        let waveform_canvas = crate::visualizer::canvas::waveform_canvas::WaveformCanvas::new(
            waveform_timeline.clone(),
            config.clone(),
        )
        .await;

        // Initialize dragging system after config is ready
        let dragging_system = crate::dragging::DraggingSystem::new(config.clone()).await;

        let scope_selection_sync_actor = {
            let selected_variables_for_scope = selected_variables.clone();
            let files_selected_scope = config.files_selected_scope.clone();

            Actor::new((), async move |_state| {
                // Emit initial selection once
                let initial = files_selected_scope.lock_ref().to_vec();
                Self::propagate_scope_selection(initial, &selected_variables_for_scope);

                // Subscribe to vector snapshots
                let mut selection_stream = files_selected_scope
                    .signal_vec_cloned()
                    .to_signal_cloned()
                    .to_stream()
                    .fuse();

                while let Some(current_selection) = selection_stream.next().await {
                    Self::propagate_scope_selection(
                        current_selection,
                        &selected_variables_for_scope,
                    );
                }
            })
        };

        let timeline_message_bridge_actor = {
            let unified_signal_response_stream = connection_message_actor
                .unified_signal_response_relay
                .subscribe()
                .fuse();
            let unified_signal_error_stream = connection_message_actor
                .unified_signal_error_relay
                .subscribe()
                .fuse();
            let batch_signal_values_stream = connection_message_actor
                .batch_signal_values_relay
                .subscribe()
                .fuse();

            let timeline_for_responses = waveform_timeline.clone();
            let timeline_for_errors = waveform_timeline.clone();
            let timeline_for_values = waveform_timeline.clone();

            Actor::new((), async move |_state| {
                let mut response_stream = unified_signal_response_stream;
                let response_timeline = timeline_for_responses.clone();
                Task::start(async move {
                    while let Some(event) = response_stream.next().await {
                        response_timeline.apply_unified_signal_response(
                            &event.request_id,
                            event.signal_data,
                            event.cursor_values,
                        );
                        // TODO: incorporate cached_time_range_ns & statistics into cache controller.
                    }
                });

                let mut error_stream = unified_signal_error_stream;
                let error_timeline = timeline_for_errors.clone();
                Task::start(async move {
                    while let Some((request_id, error)) = error_stream.next().await {
                        error_timeline.handle_unified_signal_error(&request_id, &error);
                    }
                });

                let mut cursor_values_stream = batch_signal_values_stream;
                let cursor_timeline = timeline_for_values.clone();
                Task::start(async move {
                    while let Some(values) = cursor_values_stream.next().await {
                        cursor_timeline.apply_cursor_values(values);
                    }
                });

                futures::future::pending::<()>().await;
            })
        };

        // Request config loading through platform layer
        if let Err(_e) =
            <crate::platform::CurrentPlatform as crate::platform::Platform>::send_message(
                shared::UpMsg::LoadConfig,
            )
            .await
        {}

        let file_dialog_visible = Atom::new(false);
        let key_repeat_handles = Rc::new(RefCell::new(HashMap::new()));

        Self::setup_app_coordination(&selected_variables, &config).await;

        NovyWaveApp {
            tracked_files,
            selected_variables,
            waveform_timeline,
            waveform_canvas,
            config,
            dragging_system,
            connection: connection_arc,
            _connection_message_actor: connection_message_actor,
            _workspace_event_actor: workspace_event_actor,
            _config_loaded_actor: config_loaded_actor,
            _config_error_actor: config_error_actor,
            _scope_selection_sync_actor: scope_selection_sync_actor,
            _timeline_message_bridge_actor: timeline_message_bridge_actor,
            file_dialog_visible,
            workspace_picker_visible,
            workspace_path,
            workspace_loading,
            default_workspace_path,
            workspace_picker_domain,
            workspace_picker_target,
            _workspace_picker_restoring: workspace_picker_restoring,
            key_repeat_handles,
            _workspace_history_selection_actor: workspace_history_selection_actor,
            _workspace_history_expanded_actor: workspace_history_expanded_actor,
            _workspace_history_scroll_actor: workspace_history_scroll_actor,
            _workspace_history_restore_actor: workspace_history_restore_actor,
        }
    }

    /// Load and register fonts (from main.rs)
    async fn load_and_register_fonts() {
        use zoon::futures_util::future::try_join_all;

        let fonts = try_join_all([
            fast2d::fetch_file("/_api/public/fonts/FiraCode-Regular.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Regular.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Bold.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Italic.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-BoldItalic.ttf"),
        ])
        .await
        .unwrap_throw();

        fast2d::register_fonts(fonts).unwrap_throw();
    }

    /// Create connection with ConnectionMessageActor integration
    /// Returns both connection and message actor for proper Actor+Relay architecture
    async fn create_connection_with_message_actor(
        tracked_files: TrackedFiles,
        _selected_variables: SelectedVariables,
    ) -> (
        SendWrapper<Connection<UpMsg, DownMsg>>,
        ConnectionMessageActor,
    ) {
        use futures::channel::mpsc;

        let (down_msg_sender, down_msg_receiver) = mpsc::unbounded::<DownMsg>();
        let tf_relay = tracked_files.file_load_completed_relay.clone();

        // Create ConnectionMessageActor with the message stream
        // ✅ FIX: Move receiver into closure to prevent reference capture after Send bounds removal
        let connection_message_actor =
            ConnectionMessageActor::new(down_msg_receiver, tracked_files.clone()).await;

        // Create connection that sends to the stream
        let connection = Connection::new({
            let sender = down_msg_sender; // Move sender explicitly before closure
            move |down_msg, _| {
                // Log the received message
                // Handle TrackedFiles messages directly (not routed through ConnectionMessageActor)
                match &down_msg {
                    DownMsg::FileLoaded { file_id, hierarchy } => {
                        if let Some(loaded_file) = hierarchy.files.first() {
                            tf_relay.send((
                                file_id.clone(),
                                shared::FileState::Loaded(loaded_file.clone()),
                            ));
                        }
                    }
                    DownMsg::ConfigLoaded(_config_loaded) => {}
                    DownMsg::WorkspaceLoaded { .. } => {}
                    DownMsg::ParsingStarted { file_id, .. } => {
                        tf_relay.send((
                            file_id.clone(),
                            shared::FileState::Loading(shared::LoadingStatus::Parsing),
                        ));
                    }
                    DownMsg::ParsingError { file_id, error: _ } => {
                        tf_relay.send((
                            file_id.clone(),
                            shared::FileState::Failed(shared::FileError::FileNotFound {
                                path: file_id.clone(),
                            }),
                        ));
                    }
                    _ => {
                        // All other messages go to ConnectionMessageActor for routing
                    }
                }

                // Send all messages to ConnectionMessageActor for domain routing
                let _ = sender.unbounded_send(down_msg);
            }
        });

        (SendWrapper::new(connection), connection_message_actor)
    }

    /// Setup app-level coordination
    async fn setup_app_coordination(selected_variables: &SelectedVariables, config: &AppConfig) {
        // Restore selected variables from config
        if !config.loaded_selected_variables.is_empty() {
            selected_variables
                .variables_restored_relay
                .send(config.loaded_selected_variables.clone());
        }
    }

    /// Root UI element
    pub fn root(&self) -> impl Element {
        let dragging_system = self.dragging_system.clone();
        let dragging_system_for_overlay = dragging_system.clone();

        Stack::new()
            .s(Height::screen())
            .s(Width::fill())
            .s(
                Background::new().color_signal(self.config.theme_actor.signal().map(|theme| {
                    match theme {
                        shared::Theme::Light => "rgb(255, 255, 255)",
                        shared::Theme::Dark => "rgb(13, 13, 13)",
                    }
                })),
            )
            .s(Font::new().family([
                FontFamily::new("Inter"),
                FontFamily::new("system-ui"),
                FontFamily::new("Segoe UI"),
                FontFamily::new("Arial"),
                FontFamily::SansSerif,
            ]))
            .update_raw_el({
                let theme_relay = self.config.theme_button_clicked_relay.clone();
                let dock_relay = self.config.dock_mode_button_clicked_relay.clone();

                // Timeline navigation relays
                let timeline = self.waveform_timeline.clone();
                let left_key_pressed = timeline.left_key_pressed_relay.clone();
                let right_key_pressed = timeline.right_key_pressed_relay.clone();
                let zoom_in_pressed = timeline.zoom_in_pressed_relay.clone();
                let zoom_out_pressed = timeline.zoom_out_pressed_relay.clone();
                let pan_left_pressed = timeline.pan_left_pressed_relay.clone();
                let pan_right_pressed = timeline.pan_right_pressed_relay.clone();
                let jump_to_previous = timeline.jump_to_previous_pressed_relay.clone();
                let jump_to_next = timeline.jump_to_next_pressed_relay.clone();
                let reset_zoom_center = timeline.reset_zoom_center_pressed_relay.clone();
                let reset_zoom = timeline.reset_zoom_pressed_relay.clone();
                let shift_key_pressed = timeline.shift_key_pressed_relay.clone();
                let shift_key_released = timeline.shift_key_released_relay.clone();
                let tooltip_toggle = timeline.tooltip_toggle_requested_relay.clone();
                let dragging_system_for_events = dragging_system.clone();
                let key_repeat_handles = self.key_repeat_handles.clone();

                move |raw_el| {
                    let repeat_handles_for_down = key_repeat_handles.clone();
                    let repeat_handles_for_up = key_repeat_handles.clone();
                    let raw_el = raw_el.global_event_handler_with_options(
                        EventOptions::new().preventable(),
                        move |event: KeyDown| {
                            // Check if the active element is an input/textarea to disable shortcuts
                            // This prevents conflicts when user is typing in input fields
                            let should_handle_shortcuts = if let Some(window) = web_sys::window() {
                                if let Some(document) = window.document() {
                                    if let Some(active_element) = document.active_element() {
                                        let tag_name = active_element.tag_name().to_lowercase();
                                        // Disable shortcuts when input fields are focused
                                        !matches!(tag_name.as_str(), "input" | "textarea")
                                    } else {
                                        true // No active element, allow shortcuts
                                    }
                                } else {
                                    true // No document, allow shortcuts
                                }
                            } else {
                                true // No window, allow shortcuts
                            };

                            if !should_handle_shortcuts {
                                return;
                            }

                            // Handle Shift key tracking
                            if event.shift_key() {
                                shift_key_pressed.send(());
                            }

                            // Handle keyboard shortcuts
                            if event.ctrl_key() {
                                match event.key().as_str() {
                                    "t" | "T" => {
                                        event.prevent_default();
                                        theme_relay.send(());
                                    }
                                    "d" | "D" => {
                                        event.prevent_default();
                                        dock_relay.send(());
                                    }
                                    _ => {}
                                }
                            } else {
                                // Timeline navigation shortcuts (without Ctrl)
                                match event.key().as_str() {
                                    // Cursor Movement
                                    "q" | "Q" => {
                                        event.prevent_default();
                                        if event.shift_key() {
                                            stop_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::CursorLeft,
                                            );
                                            jump_to_previous.send(());
                                        } else {
                                            left_key_pressed.send(());
                                            if !event.repeat() {
                                                let relay = left_key_pressed.clone();
                                                start_key_repeat(
                                                    &repeat_handles_for_down,
                                                    KeyAction::CursorLeft,
                                                    KEY_REPEAT_INTERVAL_MS,
                                                    move || relay.send(()),
                                                );
                                            }
                                        }
                                    }
                                    "e" | "E" => {
                                        event.prevent_default();
                                        if event.shift_key() {
                                            stop_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::CursorRight,
                                            );
                                            jump_to_next.send(());
                                        } else {
                                            right_key_pressed.send(());
                                            if !event.repeat() {
                                                let relay = right_key_pressed.clone();
                                                start_key_repeat(
                                                    &repeat_handles_for_down,
                                                    KeyAction::CursorRight,
                                                    KEY_REPEAT_INTERVAL_MS,
                                                    move || relay.send(()),
                                                );
                                            }
                                        }
                                    }

                                    // Viewport Panning
                                    "a" | "A" => {
                                        event.prevent_default();
                                        pan_left_pressed.send(());
                                        if !event.repeat() {
                                            let relay = pan_left_pressed.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::PanLeft,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || relay.send(()),
                                            );
                                        }
                                    }
                                    "d" | "D" => {
                                        event.prevent_default();
                                        pan_right_pressed.send(());
                                        if !event.repeat() {
                                            let relay = pan_right_pressed.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::PanRight,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || relay.send(()),
                                            );
                                        }
                                    }

                                    // Zoom Controls
                                    "w" | "W" => {
                                        event.prevent_default();
                                        zoom_in_pressed.send(());
                                        if !event.repeat() {
                                            let relay = zoom_in_pressed.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::ZoomIn,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || relay.send(()),
                                            );
                                        }
                                    }
                                    "s" | "S" => {
                                        event.prevent_default();
                                        zoom_out_pressed.send(());
                                        if !event.repeat() {
                                            let relay = zoom_out_pressed.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::ZoomOut,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || relay.send(()),
                                            );
                                        }
                                    }

                                    // Reset Controls
                                    "z" | "Z" => {
                                        event.prevent_default();
                                        reset_zoom_center.send(());
                                    }
                                    "r" | "R" => {
                                        event.prevent_default();
                                        reset_zoom.send(());
                                    }
                                    "t" | "T" => {
                                        event.prevent_default();
                                        tooltip_toggle.send(());
                                    }

                                    _ => {}
                                }
                            }
                        },
                    );

                    let raw_el = raw_el.global_event_handler_with_options(
                        EventOptions::new().preventable(),
                        move |event: KeyUp| match event.key().as_str() {
                            "Shift" | "ShiftLeft" | "ShiftRight" => {
                                shift_key_released.send(());
                            }
                            "q" | "Q" => {
                                stop_key_repeat(&repeat_handles_for_up, KeyAction::CursorLeft);
                            }
                            "e" | "E" => {
                                stop_key_repeat(&repeat_handles_for_up, KeyAction::CursorRight);
                            }
                            "a" | "A" => {
                                stop_key_repeat(&repeat_handles_for_up, KeyAction::PanLeft);
                            }
                            "d" | "D" => {
                                stop_key_repeat(&repeat_handles_for_up, KeyAction::PanRight);
                            }
                            "w" | "W" => {
                                stop_key_repeat(&repeat_handles_for_up, KeyAction::ZoomIn);
                            }
                            "s" | "S" => {
                                stop_key_repeat(&repeat_handles_for_up, KeyAction::ZoomOut);
                            }
                            _ => {}
                        },
                    );

                    let dragging_system_for_move = dragging_system_for_events.clone();
                    let raw_el =
                        raw_el.global_event_handler(move |event: events_extra::PointerMove| {
                            crate::dragging::process_drag_movement(
                                &dragging_system_for_move,
                                (event.x() as f32, event.y() as f32),
                            );
                        });

                    let dragging_system_for_up = dragging_system_for_events.clone();
                    let raw_el = raw_el.global_event_handler(move |_: events_extra::PointerUp| {
                        crate::dragging::end_drag(&dragging_system_for_up);
                    });

                    let dragging_system_for_cancel = dragging_system_for_events;
                    raw_el.global_event_handler(move |_: events_extra::PointerCancel| {
                        crate::dragging::end_drag(&dragging_system_for_cancel);
                    })
                }
            })
            .layer(
                Column::new()
                    .s(Width::fill())
                    .s(Height::fill())
                    .item(self.workspace_bar())
                    .item(
                        El::new()
                            .s(Width::fill())
                            .s(Height::growable())
                            .child(self.main_layout()),
                    ),
            )
            .layer_signal(
                dragging_system_for_overlay
                    .active_overlay_divider_signal()
                    .map({
                        let dragging_system = dragging_system_for_overlay.clone();
                        move |maybe_divider| {
                            maybe_divider.map(|divider| {
                                dragging_overlay_element(dragging_system.clone(), divider).unify()
                            })
                        }
                    }),
            )
            .layer_signal(self.workspace_picker_visible.signal().map_true({
                let workspace_picker_visible = self.workspace_picker_visible.clone();
                let workspace_loading = self.workspace_loading.clone();
                let workspace_path = self.workspace_path.clone();
                let tracked_files = self.tracked_files.clone();
                let selected_variables = self.selected_variables.clone();
                let default_workspace_path = self.default_workspace_path.clone();
                let workspace_picker_domain = self.workspace_picker_domain.clone();
                let config = self.config.clone();
                let workspace_picker_target = self.workspace_picker_target.clone();
                move || {
                    workspace_picker_dialog(
                        workspace_picker_visible.clone(),
                        workspace_loading.clone(),
                        workspace_path.clone(),
                        tracked_files.clone(),
                        selected_variables.clone(),
                        default_workspace_path.clone(),
                        config.clone(),
                        workspace_picker_target.clone(),
                        workspace_picker_domain.clone(),
                    )
                }
            }))
            .layer_signal(self.file_dialog_visible.signal().map_true({
                let tracked_files = self.tracked_files.clone();
                let selected_variables = self.selected_variables.clone();
                let config = self.config.clone();
                let file_dialog_visible = self.file_dialog_visible.clone();
                let connection = self.connection.clone();
                move || {
                    file_paths_dialog(
                        tracked_files.clone(),
                        selected_variables.clone(),
                        config.clone(),
                        file_dialog_visible.clone(),
                        crate::connection::ConnectionAdapter::from_arc(connection.clone()),
                    )
                }
            }))
            .layer(self.toast_notifications_container())
    }

    /// Main layout
    fn main_layout(&self) -> impl Element {
        crate::main_layout(
            &self.tracked_files,
            &self.selected_variables,
            &self.waveform_timeline,
            &self.config,
            &self.dragging_system,
            &self.waveform_canvas,
            &self.file_dialog_visible,
        )
    }

    fn workspace_bar(&self) -> impl Element {
        use moonzoon_novyui::components::input::{InputSize, input};
        use moonzoon_novyui::tokens::color::{neutral_8, neutral_12};
        use moonzoon_novyui::tokens::typography::font_mono;

        let background_color_signal = self.config.theme_actor.signal().map(|theme| match theme {
            shared::Theme::Light => Some("oklch(96% 0.015 255)"),
            shared::Theme::Dark => Some("oklch(26% 0.02 255)"),
        });

        let divider_color_signal = self.config.theme_actor.signal().map(|theme| match theme {
            shared::Theme::Light => Border::new().width(1).color("oklch(82% 0.015 255)"),
            shared::Theme::Dark => Border::new().width(1).color("oklch(40% 0.02 255)"),
        });

        let path_signal = map_ref! {
            let current = self.workspace_path.signal_cloned(),
            let default = self.default_workspace_path.signal_cloned(),
            let loading = self.workspace_loading.signal() => {
                if let Some(path) = current.clone() {
                    path
                } else if let Some(default_path) = default.clone() {
                    default_path
                } else if *loading {
                    String::from("Loading workspace...")
                } else {
                    String::from("No workspace selected")
                }
            }
        };

        let workspace_input = El::new().s(Width::fill()).child(
            input()
                .size(InputSize::Small)
                .value_signal(path_signal)
                .readonly()
                .build(),
        );

        let open_workspace_button = {
            let workspace_picker_visible = self.workspace_picker_visible.clone();

            button()
                .label("Open Workspace…")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(move || {
                    workspace_picker_visible.set(true);
                })
                .build()
        };

        let loading_indicator = El::new().child_signal(map_ref! {
            let loading = self.workspace_loading.signal(),
            let current = self.workspace_path.signal_cloned() => {
                if *loading && current.is_none() {
                    Some(
                        El::new()
                            .s(Font::new().size(12).color_signal(neutral_8()))
                            .child("Loading workspace…")
                            .into_element()
                    )
                } else {
                    None
                }
            }
        });

        Row::new()
            .s(Width::fill())
            .s(Padding::new().x(SPACING_6).y(SPACING_4))
            .s(Gap::new().x(SPACING_6))
            .s(Background::new().color_signal(background_color_signal))
            .s(Borders::new().bottom_signal(divider_color_signal))
            .s(Align::new().center_y())
            .item(
                El::new().s(Padding::new().x(SPACING_8)).child(
                    El::new()
                        .s(Font::new()
                            .size(18)
                            .weight(FontWeight::Bold)
                            .tracking(1)
                            .no_wrap()
                            .color_signal(neutral_12()))
                        .s(font_mono())
                        .child("NovyWave"),
                ),
            )
            .item(open_workspace_button)
            .item(workspace_input)
            .item(loading_indicator)
            .item(El::new().s(Width::growable()))
            .item(crate::action_buttons::theme_toggle_button(&self.config))
    }

    fn start_workspace_switch(
        workspace_loading: Mutable<bool>,
        workspace_path: Mutable<Option<String>>,
        tracked_files: TrackedFiles,
        selected_variables: SelectedVariables,
        path: String,
    ) {
        let trimmed = path.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let is_same_path = workspace_path
            .lock_ref()
            .as_ref()
            .map(|current| current == &trimmed)
            .unwrap_or(false);

        if is_same_path {
            workspace_loading.set(false);
            return;
        }

        workspace_loading.set(true);
        workspace_path.set(Some(trimmed.clone()));
        tracked_files.all_files_cleared_relay.send(());
        selected_variables.selection_cleared_relay.send(());

        Task::start({
            let request_path = trimmed.clone();
            let workspace_loading = workspace_loading.clone();
            let workspace_path = workspace_path.clone();
            async move {
                if let Err(err) =
                    <crate::platform::CurrentPlatform as crate::platform::Platform>::send_message(
                        shared::UpMsg::SelectWorkspace {
                            root: request_path.clone(),
                        },
                    )
                    .await
                {
                    workspace_loading.set(false);
                    zoon::println!("Failed to request workspace '{}': {}", request_path, err);
                    workspace_path.set(Some(request_path));
                }
            }
        });
    }

    fn apply_workspace_history_state(
        history: &shared::WorkspaceHistory,
        path: &str,
        domain: &crate::config::FilePickerDomain,
    ) {
        let mut expanded_set = IndexSet::new();
        let mut scroll_value: i32 = 0;

        if !path.is_empty() {
            if let Some(tree_state) = history.tree_state.get(path) {
                for entry in &tree_state.expanded_paths {
                    expanded_set.insert(entry.clone());
                }
                let scroll_clamped = tree_state.scroll_top.max(0.0).round();
                scroll_value = scroll_clamped.clamp(0.0, i32::MAX as f64) as i32;
            }
        }

        domain
            .expanded_directories_actor
            .state
            .set_neq(expanded_set);
        domain.scroll_position_actor.state.set_neq(scroll_value);
    }

    fn apply_workspace_picker_tree_state(
        history: &shared::WorkspaceHistory,
        domain: &crate::config::FilePickerDomain,
    ) {
        if let Some(tree_state) = history.picker_tree_state.as_ref() {
            let mut expanded_set = IndexSet::new();
            for entry in &tree_state.expanded_paths {
                expanded_set.insert(entry.clone());
            }
            let scroll_clamped = tree_state.scroll_top.max(0.0).round();
            let scroll_value = scroll_clamped.clamp(0.0, i32::MAX as f64) as i32;

            domain
                .expanded_directories_actor
                .state
                .set_neq(expanded_set);
            domain.scroll_position_actor.state.set_neq(scroll_value);
        }
    }

    fn publish_workspace_picker_snapshot(
        config: &AppConfig,
        domain: &crate::config::FilePickerDomain,
        target: &Mutable<Option<String>>,
    ) {
        let expanded_vec = domain
            .expanded_directories_actor
            .state
            .lock_ref()
            .iter()
            .cloned()
            .collect::<Vec<String>>();
        let scroll_current = *domain.scroll_position_actor.state.lock_ref() as f64;
        emit_trace(
            "workspace_picker_snapshot",
            format!("expanded_paths={expanded_vec:?} scroll_top={scroll_current}"),
        );
        config.update_workspace_picker_tree_state(expanded_vec.clone());
        if let Some(path) = target.lock_ref().clone() {
            config.update_workspace_tree_state(&path, expanded_vec);

            config.update_workspace_scroll(&path, scroll_current);
        }

        config.update_workspace_picker_scroll(scroll_current);
    }

    fn toast_notifications_container(&self) -> impl Element {
        crate::error_ui::toast_notifications_container(self.config.clone())
    }
}

fn dragging_overlay_element(
    dragging_system: crate::dragging::DraggingSystem,
    divider_type: crate::dragging::DividerType,
) -> impl Element {
    use crate::dragging::DividerType;

    let cursor_icon = match divider_type {
        DividerType::FilesPanelSecondary => CursorIcon::RowResize,
        DividerType::VariablesNameColumn
        | DividerType::VariablesValueColumn
        | DividerType::FilesPanelMain => CursorIcon::ColumnResize,
    };

    let system_for_move = dragging_system.clone();
    let system_for_up = dragging_system.clone();
    let system_for_cancel = dragging_system.clone();

    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Cursor::new(cursor_icon))
        .update_raw_el(|raw_el| {
            raw_el
                .style("position", "fixed")
                .style("inset", "0")
                .style("z-index", "9998")
                .style("user-select", "none")
                .style("touch-action", "none")
        })
        .on_pointer_move_event(move |event: PointerEvent| {
            crate::dragging::process_drag_movement(
                &system_for_move,
                (event.x() as f32, event.y() as f32),
            );
        })
        .on_pointer_up(move || {
            crate::dragging::end_drag(&system_for_up);
        })
        .update_raw_el(move |raw_el| {
            raw_el.event_handler(move |_: events_extra::PointerCancel| {
                crate::dragging::end_drag(&system_for_cancel);
            })
        })
}

fn workspace_picker_dialog(
    workspace_picker_visible: Atom<bool>,
    workspace_loading: Mutable<bool>,
    workspace_path: Mutable<Option<String>>,
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    default_workspace_path: Mutable<Option<String>>,
    app_config: AppConfig,
    workspace_picker_target: Mutable<Option<String>>,
    workspace_picker_domain: crate::config::FilePickerDomain,
) -> impl Element {
    use moonzoon_novyui::tokens::color::{
        neutral_1, neutral_2, neutral_4, neutral_8, neutral_11, primary_3, primary_6,
    };
    use moonzoon_novyui::tokens::theme::{Theme, theme};

    let default_workspace_snapshot = default_workspace_path.lock_ref().clone();

    let open_action = {
        let workspace_loading = workspace_loading.clone();
        let workspace_path = workspace_path.clone();
        let tracked_files = tracked_files.clone();
        let selected_variables = selected_variables.clone();
        let workspace_picker_visible = workspace_picker_visible.clone();
        let workspace_picker_domain = workspace_picker_domain.clone();
        let app_config_for_open = app_config.clone();
        let workspace_picker_target_for_open = workspace_picker_target.clone();
        move || {
            if workspace_loading.lock_ref().clone() {
                return;
            }
            let selected_paths = workspace_picker_domain.selected_files_vec_signal.lock_ref();
            let Some(path) = selected_paths.first().cloned() else {
                return;
            };
            app_config_for_open.record_workspace_selection(&path);
            workspace_picker_target_for_open.set_neq(Some(path.clone()));
            let history_snapshot = app_config_for_open.workspace_history_state.get_cloned();
            NovyWaveApp::apply_workspace_history_state(
                &history_snapshot,
                &path,
                &workspace_picker_domain,
            );
            NovyWaveApp::start_workspace_switch(
                workspace_loading.clone(),
                workspace_path.clone(),
                tracked_files.clone(),
                selected_variables.clone(),
                path,
            );
            workspace_picker_visible.set(false);
        }
    };
    let open_action = Rc::new(open_action);

    El::new()
        .s(Background::new().color_signal(theme().map(|t| match t {
            Theme::Light => "rgba(255, 255, 255, 0.85)",
            Theme::Dark => "rgba(0, 0, 0, 0.85)",
        })))
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .s(Padding::all(40))
        .update_raw_el(|raw_el| {
            raw_el
                .style("display", "flex")
                .style("position", "fixed")
                .style("inset", "0")
                .style("z-index", "21000")
                .style("justify-content", "center")
                .style("align-items", "center")
        })
        .update_raw_el({
            let workspace_picker_visible_for_click = workspace_picker_visible.clone();
            let workspace_picker_visible_for_key = workspace_picker_visible.clone();
            move |raw_el| {
                raw_el
                    .event_handler(move |event: Click| {
                        workspace_picker_visible_for_click.set(false);
                        event.stop_propagation();
                    })
                    .global_event_handler(move |event: KeyDown| {
                        if event.key() == "Escape" {
                            workspace_picker_visible_for_key.set(false);
                        }
                    })
            }
        })
        .child(
            El::new()
                .s(Background::new().color_signal(neutral_1()))
                .s(RoundedCorners::all(8))
                .s(Borders::all_signal(neutral_4().map(|color| {
                    Border::new().width(1).color(color)
                })))
                .s(Width::fill().max(700))
                .s(Height::fill().max(640))
                .s(Padding::all(SPACING_6))
                .update_raw_el(|raw_el| raw_el.event_handler(|event: Click| event.stop_propagation()))
                .child(
                    Column::new()
                        .s(Height::fill())
                        .s(Gap::new().y(SPACING_12))
                        .s(Padding::new().bottom(SPACING_4))
                        .update_raw_el(|raw_el| raw_el.style("min-height", "0"))
                        .s(Padding::new().bottom(SPACING_6))
                        .item(
                            Column::new()
                                .s(Gap::new().y(SPACING_4))
                                .item(
                                    El::new()
                                        .s(Padding::new().x(SPACING_10).top(SPACING_6))
                                        .s(Font::new().size(18).weight(FontWeight::Bold).color_signal(neutral_11()))
                                        .child("Open Workspace")
                                )
                                .item(
                                    El::new()
                                        .s(Padding::new().x(SPACING_10))
                                        .s(Font::new().size(14).color_signal(neutral_8()))
                                        .child("Pick a directory to use as your workspace. NovyWave will create the .novywave file automatically if it is missing.")
                                )
                        )
                        .item({
                            let mut top_sections: Vec<RawHtmlEl> = Vec::new();

                            if let Some(default_path) = default_workspace_snapshot.clone() {
                                let workspace_loading = workspace_loading.clone();
                                let workspace_path = workspace_path.clone();
                                let tracked_files = tracked_files.clone();
                                let selected_variables = selected_variables.clone();
                                let workspace_picker_visible = workspace_picker_visible.clone();
                                let workspace_picker_domain = workspace_picker_domain.clone();
                                let app_config = app_config.clone();
                                let workspace_picker_target = workspace_picker_target.clone();
                                let default_path_for_disable = default_path.clone();
                                let display_path = default_path.clone();
                                let action_path = default_path.clone();

                                let should_show_default_section = workspace_path
                                    .lock_ref()
                                    .as_ref()
                                    .map(|current| current != &default_path_for_disable)
                                    .unwrap_or(true);

                                if should_show_default_section {
                                    let default_disabled_signal = workspace_path
                                        .signal_cloned()
                                        .map(move |current| {
                                            current
                                                .as_ref()
                                                .map(|path| path == &default_path_for_disable)
                                                .unwrap_or(false)
                                        });

                                    let default_section = Column::new()
                                        .s(Width::fill())
                                        .s(Background::new().color_signal(neutral_2()))
                                        .s(RoundedCorners::all(6))
                                        .s(Padding::new().x(SPACING_8).top(SPACING_4).bottom(SPACING_4))
                                        .s(Gap::new().y(SPACING_2))
                                        .item(
                                            El::new()
                                                .s(Font::new()
                                                    .size(13)
                                                    .weight(FontWeight::Medium)
                                                    .color_signal(neutral_11()))
                                                .child("Default workspace"),
                                        )
                                        .item(
                                            Row::new()
                                                .s(Width::fill())
                                                .s(Align::new().center_x())
                                                .item(
                                                    button()
                                                        .label("Open Default Workspace")
                                                        .left_icon(IconName::House)
                                                        .variant(ButtonVariant::Outline)
                                                        .size(ButtonSize::Small)
                                                        .disabled_signal(default_disabled_signal)
                                                        .on_press(move || {
                                                            if workspace_path
                                                                .lock_ref()
                                                                .as_ref()
                                                                .map(|current| current == &action_path)
                                                                .unwrap_or(false)
                                                            {
                                                                workspace_picker_visible.set(false);
                                                                return;
                                                            }
                                                            app_config.record_workspace_selection(&action_path);
                                                            workspace_picker_target.set_neq(Some(action_path.clone()));
                                                            let history = app_config.workspace_history_state.get_cloned();
                                                            NovyWaveApp::apply_workspace_history_state(&history, &action_path, &workspace_picker_domain);
                                                            workspace_picker_domain.clear_selection_relay.send(());
                                                            workspace_picker_domain.file_selected_relay.send(action_path.clone());
                                                            NovyWaveApp::start_workspace_switch(
                                                                workspace_loading.clone(),
                                                                workspace_path.clone(),
                                                                tracked_files.clone(),
                                                                selected_variables.clone(),
                                                                action_path.clone(),
                                                            );
                                                            workspace_picker_visible.set(false);
                                                        })
                                                        .build()
                                                )
                                        )
                                        .item(
                                            El::new()
                                                .s(Font::new()
                                                    .size(12)
                                                    .color_signal(neutral_8()))
                                                .child(display_path),
                                        );
                                    top_sections.push(default_section.into_raw_el());
                                }
                            }

                            let recent_section = Column::new()
                                .s(Width::fill())
                                .s(Background::new().color_signal(neutral_2()))
                                .s(RoundedCorners::all(6))
                                .s(Padding::new().x(SPACING_8).top(SPACING_4).bottom(SPACING_4))
                                .s(Gap::new().y(SPACING_2))
                                .item(
                                    El::new()
                                        .s(Font::new()
                                            .size(13)
                                            .weight(FontWeight::Medium)
                                            .color_signal(neutral_11()))
                                        .child("Recent workspaces"),
                                )
                                .item(
                                    El::new().child_signal({
                                        let workspace_loading = workspace_loading.clone();
                                        let workspace_path = workspace_path.clone();
                                        let tracked_files = tracked_files.clone();
                                        let selected_variables = selected_variables.clone();
                                        let workspace_picker_visible = workspace_picker_visible.clone();
                                        let workspace_picker_domain = workspace_picker_domain.clone();
                                        let workspace_picker_target = workspace_picker_target.clone();
                                        let app_config = app_config.clone();
                                        let default_workspace_snapshot = default_workspace_snapshot.clone();
                                        app_config
                                            .workspace_history_state
                                            .signal_cloned()
                                            .map(move |history| {
                                                let current_workspace = workspace_path.lock_ref().clone();

                                                let filtered: Vec<String> = history
                                                    .recent_paths
                                                    .iter()
                                                    .filter(|path| {
                                                        let matches_current = current_workspace
                                                            .as_ref()
                                                            .map(|current| current.as_str() == path.as_str())
                                                            .unwrap_or(false);
                                                        let matches_default = default_workspace_snapshot
                                                            .as_ref()
                                                            .map(|default_path| default_path.as_str() == path.as_str())
                                                            .unwrap_or(false);

                                                        !matches_current && !matches_default
                                                    })
                                                    .cloned()
                                                    .collect();

                                                if filtered.is_empty() {
                                                    Column::new()
                                                        .s(Gap::new().y(SPACING_2))
                                                        .item(
                                                            El::new()
                                                                .s(Font::new()
                                                                    .size(12)
                                                                    .color_signal(neutral_8()))
                                                                .s(Padding::new().x(SPACING_8))
                                                                .child("No recent workspaces"),
                                                        )
                                                        .into_raw_el()
                                                } else {
                                                    let mut iter = filtered.into_iter();
                                                    let first_path = iter.next().unwrap();
                                                    let workspace_loading_first = workspace_loading.clone();
                                                    let workspace_path_first = workspace_path.clone();
                                                    let tracked_files_first = tracked_files.clone();
                                                    let selected_variables_first = selected_variables.clone();
                                                    let workspace_picker_visible_first = workspace_picker_visible.clone();
                                                    let workspace_picker_domain_first = workspace_picker_domain.clone();
                                                    let workspace_picker_target_first = workspace_picker_target.clone();
                                                    let app_config_first = app_config.clone();

                                                    let mut column = Column::new()
                                                        .s(Gap::new().y(SPACING_2))
                                                        .item(
                                                            button()
                                                                .label(first_path.clone())
                                                                .variant(ButtonVariant::Ghost)
                                                                .size(ButtonSize::Small)
                                                                .on_press(move || {
                                                                    app_config_first.record_workspace_selection(&first_path);
                                                                    workspace_picker_target_first.set_neq(Some(first_path.clone()));
                                                                    let history_snapshot = app_config_first.workspace_history_state.get_cloned();
                                                                    NovyWaveApp::apply_workspace_history_state(&history_snapshot, &first_path, &workspace_picker_domain_first);
                                                                    workspace_picker_domain_first.clear_selection_relay.send(());
                                                                    workspace_picker_domain_first.file_selected_relay.send(first_path.clone());
                                                                    NovyWaveApp::start_workspace_switch(
                                                                        workspace_loading_first.clone(),
                                                                        workspace_path_first.clone(),
                                                                        tracked_files_first.clone(),
                                                                        selected_variables_first.clone(),
                                                                        first_path.clone(),
                                                                    );
                                                                    workspace_picker_visible_first.set(false);
                                                                })
                                                                .build(),
                                                        );

                                                    for path in iter {
                                                        let workspace_loading = workspace_loading.clone();
                                                        let workspace_path = workspace_path.clone();
                                                        let tracked_files = tracked_files.clone();
                                                        let selected_variables = selected_variables.clone();
                                                        let workspace_picker_visible = workspace_picker_visible.clone();
                                                        let workspace_picker_domain = workspace_picker_domain.clone();
                                                        let workspace_picker_target = workspace_picker_target.clone();
                                                        let app_config = app_config.clone();
                                                        column = column.item(
                                                            button()
                                                                .label(path.clone())
                                                                .variant(ButtonVariant::Ghost)
                                                                .size(ButtonSize::Small)
                                                                .on_press(move || {
                                                                    app_config.record_workspace_selection(&path);
                                                                    workspace_picker_target.set_neq(Some(path.clone()));
                                                                    let history_snapshot = app_config.workspace_history_state.get_cloned();
                                                                    NovyWaveApp::apply_workspace_history_state(&history_snapshot, &path, &workspace_picker_domain);
                                                                    workspace_picker_domain.clear_selection_relay.send(());
                                                                    workspace_picker_domain.file_selected_relay.send(path.clone());
                                                                    NovyWaveApp::start_workspace_switch(
                                                                        workspace_loading.clone(),
                                                                        workspace_path.clone(),
                                                                        tracked_files.clone(),
                                                                        selected_variables.clone(),
                                                                        path.clone(),
                                                                    );
                                                                    workspace_picker_visible.set(false);
                                                                })
                                                                .build(),
                                                        );
                                                    }

                                                    column.into_raw_el()
                                                }
                                            })
                                    })
                                );
                            top_sections.push(recent_section.into_raw_el());

                            let tree_scroll_container = El::new()
                                .s(Height::fill())
                                .s(Width::fill())
                                .s(Scrollbars::both())
                                .viewport_y_signal({
                                    let scroll_position_actor =
                                        workspace_picker_domain.scroll_position_actor.clone();
                                    zoon::map_ref! {
                                        let position = scroll_position_actor.signal() => {
                                            *position
                                        }
                                    }
                                })
                                .update_raw_el({
                                    let scroll_relay =
                                        workspace_picker_domain.scroll_position_changed_relay.clone();
                                    move |raw_el| {
                                        let dom_element = raw_el.dom_element();
                                        dom_element
                                            .set_attribute("data-scroll-container", "workspace-picker")
                                            .unwrap();

                                        let relay_for_event = scroll_relay.clone();
                                        let scroll_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                                            if let Some(target) = event.current_target() {
                                                if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                                                    relay_for_event.send(element.scroll_top());
                                                }
                                            }
                                        }) as Box<dyn FnMut(_)>);

                                        dom_element
                                            .add_event_listener_with_callback(
                                                "scroll",
                                                scroll_closure.as_ref().unchecked_ref(),
                                            )
                                            .unwrap();
                                        scroll_closure.forget();

                                        raw_el
                                            .style("min-height", "0")
                                            .style("overflow-x", "hidden")
                                            .style("overflow-y", "auto")
                                            .style("scrollbar-width", "thin")
                                            .style_signal(
                                                "scrollbar-color",
                                                primary_6()
                                                    .map(|thumb| {
                                                        primary_3().map(move |track| {
                                                            format!("{} {}", thumb, track)
                                                        })
                                                    })
                                                    .flatten(),
                                            )
                                    }
                                })
                                .after_insert({
                                    let scroll_position_actor =
                                        workspace_picker_domain.scroll_position_actor.clone();
                                    move |_element| {
                                        Task::start({
                                            let scroll_position_actor = scroll_position_actor.clone();
                                            async move {
                                                Task::next_macro_tick().await;
                                                let position = *scroll_position_actor.state.lock_ref();
                                                if position > 0 {
                                                    if let Some(window) = web_sys::window() {
                                                        if let Some(document) = window.document() {
                                                            if let Ok(Some(element)) = document
                                                                .query_selector(
                                                                    "[data-scroll-container='workspace-picker']",
                                                                )
                                                            {
                                                                element.set_scroll_top(position);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                })
                                .child(workspace_picker_tree(workspace_picker_domain.clone()));

                            let tree_container = El::new()
                                .s(Height::growable())
                                .s(Width::fill())
                                .s(Background::new().color_signal(neutral_1()))
                                .s(Borders::all_signal(neutral_4().map(|color| {
                                    Border::new().width(1).color(color)
                                })))
                                .s(RoundedCorners::all(6))
                                .s(Padding::all(SPACING_4))
                                .update_raw_el(|raw_el| raw_el.style("min-height", "0"))
                                .child(tree_scroll_container);

                            let selection_hint = El::new()
                                .s(Font::new()
                                    .size(12)
                                    .color_signal(neutral_8()))
                                .child_signal(
                                    workspace_picker_domain
                                        .selected_files_vec_signal
                                        .signal_cloned()
                                        .map(|paths| {
                                            paths
                                                .first()
                                                .cloned()
                                                .map(|path| format!("Selected: {path}"))
                                                .unwrap_or_else(|| "Select a workspace folder from the tree.".to_string())
                                        }),
                                );

                            let button_row = Row::new()
                                .s(Width::fill())
                                .s(Padding::new().top(SPACING_6))
                                .s(Gap::new().x(SPACING_4))
                                .item(El::new().s(Width::growable()))
                                .item(
                                    button()
                                        .label("Cancel")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Small)
                                        .on_press({
                                            let workspace_picker_visible = workspace_picker_visible.clone();
                                            move || workspace_picker_visible.set(false)
                                        })
                                        .build()
                                )
                                .item(
                                    button()
                                        .label("Open")
                                        .variant(ButtonVariant::Primary)
                                        .size(ButtonSize::Small)
                                        .disabled_signal(map_ref! {
                                            let loading = workspace_loading.signal(),
                                            let selected = workspace_picker_domain.selected_files_vec_signal.signal_cloned() => {
                                                *loading || selected.is_empty()
                                            }
                                        })
                                        .on_press({
                                            let open_action = open_action.clone();
                                            move || open_action()
                                        })
                                        .build()
                                );

                            let mut layout_items = top_sections;
                            layout_items.push(tree_container.into_raw_el());
                            layout_items.push(selection_hint.into_raw_el());
                            layout_items.push(button_row.into_raw_el());

                            Column::new()
                                .s(Height::fill())
                                .s(Width::fill())
                                .s(Padding::all(SPACING_6))
                                .s(Gap::new().y(SPACING_6))
                                .update_raw_el(|raw_el| {
                                    raw_el
                                        .style("min-height", "0")
                                        .style("overflow", "hidden")
                                })
                                .items(layout_items.into_iter())
                        })
                )
        )
}

fn workspace_picker_tree(domain: crate::config::FilePickerDomain) -> impl Element {
    use crate::file_picker::{
        SelectedFilesSyncActors, TreeViewSyncActors, initialize_directories_and_request_contents,
    };
    use indexmap::IndexSet;
    use moonzoon_novyui::tokens::color::neutral_8;

    let selected_vec = MutableVec::<String>::new();
    let selected_sync = SelectedFilesSyncActors::new(domain.clone(), selected_vec.clone());
    let initialization_actor = initialize_directories_and_request_contents(&domain);
    let cache_signal = domain.directory_cache_signal();

    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .after_remove(move |_| {
            drop(selected_sync);
            drop(initialization_actor);
        })
        .child_signal({
            let domain_for_treeview = domain.clone();
            let selected_vec_for_tree = selected_vec.clone();
            cache_signal.map(move |cache| {
                if cache.contains_key("/") {
                    let tree_data = workspace_build_tree_data("/", &cache);
                    let external_expanded = Mutable::new(IndexSet::<String>::new());
                    let sync_actors = TreeViewSyncActors::new(
                        domain_for_treeview.clone(),
                        external_expanded.clone(),
                    );
                    let tree_rendering_relay = domain_for_treeview.tree_rendering_relay.clone();
                    let scroll_position_actor = domain_for_treeview.scroll_position_actor.clone();

                    El::new()
                        .s(Height::fill())
                        .s(Width::fill())
                        .after_insert(move |_element| {
                            tree_rendering_relay.send(());
                            Task::start({
                                let scroll_position_actor = scroll_position_actor.clone();
                                async move {
                                    Task::next_macro_tick().await;
                                    let position = *scroll_position_actor.state.lock_ref();
                                    if position > 0 {
                                        if let Some(window) = web_sys::window() {
                                            if let Some(document) = window.document() {
                                                if let Ok(Some(element)) = document.query_selector(
                                                    "[data-scroll-container='workspace-picker']",
                                                ) {
                                                    element.set_scroll_top(position);
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        })
                        .after_remove(move |_| {
                            drop(sync_actors);
                        })
                        .child(
                            tree_view()
                                .data(tree_data)
                                .size(TreeViewSize::Medium)
                                .variant(TreeViewVariant::Basic)
                                .show_icons(true)
                                .show_checkboxes(true)
                                .external_expanded(external_expanded)
                                .external_selected_vec(selected_vec_for_tree.clone())
                                .build()
                                .into_raw(),
                        )
                        .into_element()
                } else {
                    El::new()
                        .s(Padding::all(20))
                        .s(Font::new().color_signal(neutral_8()).italic())
                        .child("Loading directory contents...")
                        .into_element()
                }
            })
        })
}

fn workspace_build_tree_data(
    root_path: &str,
    cache: &HashMap<String, Vec<shared::FileSystemItem>>,
) -> Vec<TreeViewItemData> {
    cache
        .get(root_path)
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry.is_directory)
                .map(|entry| workspace_build_directory_item(&entry.path, &entry.name, cache))
                .collect()
        })
        .unwrap_or_default()
}

fn workspace_build_directory_item(
    path: &str,
    name: &str,
    cache: &HashMap<String, Vec<shared::FileSystemItem>>,
) -> TreeViewItemData {
    let mut item = TreeViewItemData::new(path.to_string(), name.to_string())
        .icon("folder".to_string())
        .item_type(TreeViewItemType::File)
        .has_expandable_content(true)
        .is_waveform_file(true);

    if let Some(children) = cache.get(path) {
        let mut child_items = Vec::new();
        for child in children {
            if child.is_directory {
                child_items.push(workspace_build_directory_item(
                    &child.path,
                    &child.name,
                    cache,
                ));
            }
        }
        if child_items.is_empty() {
            item = item.with_children(vec![
                TreeViewItemData::new(format!("{path}::empty"), "Empty".to_string())
                    .disabled(true)
                    .item_type(TreeViewItemType::Default),
            ]);
        } else {
            item = item.with_children(child_items);
        }
    } else {
        item = item.with_children(vec![
            TreeViewItemData::new(format!("{path}::loading"), "Loading...".to_string())
                .disabled(true)
                .item_type(TreeViewItemType::Default),
        ]);
    }

    item
}
