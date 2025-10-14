//! NovyWaveApp - Self-contained Actor+Relay Architecture

use futures::StreamExt;
use futures_signals::signal_vec::SignalVecExt;
use gloo_timers::callback::Interval;
use indexmap::IndexSet;
use moonzoon_novyui::*;
use zoon::events::{KeyDown, KeyUp};
use zoon::events_extra;
use zoon::{EventOptions, *};

use crate::config::AppConfig;
use crate::dataflow::atom::Atom;
use crate::dataflow::{Actor, Relay, relay};
use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::WaveformTimeline;
use shared::{DownMsg, SignalStatistics, SignalValue, UnifiedSignalData, UpMsg};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

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
                            DownMsg::WorkspaceLoaded { root, config } => {
                                workspace_loaded_relay_clone
                                    .send(WorkspaceLoadedEvent { root, config });
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

    /// Message routing actor (keeps relays alive)
    _connection_message_actor: ConnectionMessageActor,
    _workspace_event_actor: Actor<()>,
    _config_error_actor: Actor<()>,

    /// Synchronizes Files panel scope selection into SelectedVariables domain
    _scope_selection_sync_actor: Actor<()>,

    /// Bridges connection message relays into the timeline domain
    _timeline_message_bridge_actor: Actor<()>,

    // === UI STATE (Atom pattern for local UI concerns) ===
    /// File picker dialog visibility
    pub file_dialog_visible: Atom<bool>,

    pub workspace_path: Mutable<Option<String>>,
    pub workspace_loading: Mutable<bool>,

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

        let workspace_event_actor = {
            let mut workspace_stream = connection_message_actor.workspace_loaded_relay.subscribe();
            let workspace_path = workspace_path.clone();
            let workspace_loading = workspace_loading.clone();

            Actor::new((), async move |_state| {
                while let Some(event) = workspace_stream.next().await {
                    workspace_loading.set(false);
                    workspace_path.set(Some(event.root.clone()));
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

        // Create main config with proper connection and message routing
        let config = AppConfig::new(
            connection_arc.clone(),
            connection_message_actor.clone(),
            tracked_files.clone(),
            selected_variables.clone(),
        )
        .await;

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
            _config_error_actor: config_error_actor,
            _scope_selection_sync_actor: scope_selection_sync_actor,
            _timeline_message_bridge_actor: timeline_message_bridge_actor,
            file_dialog_visible,
            workspace_path,
            workspace_loading,
            key_repeat_handles,
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
        use moonzoon_novyui::tokens::color::{neutral_8, neutral_11};

        let workspace_label = {
            let label_color_signal = neutral_11();

            El::new()
                .s(Font::new().weight(FontWeight::Medium))
                .s(Font::new().color_signal(label_color_signal))
                .child_signal(self.workspace_path.signal_cloned().map(|maybe_path| {
                    let (label, tooltip) = match maybe_path.clone() {
                        Some(path) => {
                            let display = NovyWaveApp::workspace_display_name(&path);
                            (format!("Workspace: {}", display), Some(path))
                        }
                        None => ("Workspace: (loading)".to_string(), None),
                    };

                    let mut element = El::new().child(label);
                    if let Some(tooltip) = tooltip {
                        element = element.update_raw_el(|raw_el| raw_el.attr("title", &tooltip));
                    }
                    element.into_element()
                }))
        };

        let loading_indicator =
            El::new().child_signal(self.workspace_loading.signal().map_true(|| {
                El::new()
                    .s(Font::new().size(12).color_signal(neutral_8()))
                    .child("Loading workspace…")
                    .into_element()
            }));

        let workspace_loading_for_prompt = self.workspace_loading.clone();
        let workspace_path_for_prompt = self.workspace_path.clone();
        let tracked_files_for_prompt = self.tracked_files.clone();
        let selected_variables_for_prompt = self.selected_variables.clone();

        let open_workspace_button = button()
            .label("Open Workspace…")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Small)
            .on_press(move || {
                if workspace_loading_for_prompt.lock_ref().clone() {
                    return;
                }

                let default_path = workspace_path_for_prompt
                    .lock_ref()
                    .clone()
                    .unwrap_or_default();

                if let Some(window) = web_sys::window() {
                    if let Ok(result) = window.prompt_with_message_and_default(
                        "Enter workspace folder path",
                        &default_path,
                    ) {
                        if let Some(input) = result {
                            let trimmed = input.trim();
                            if trimmed.is_empty() {
                                return;
                            }
                            NovyWaveApp::start_workspace_switch(
                                workspace_loading_for_prompt.clone(),
                                workspace_path_for_prompt.clone(),
                                tracked_files_for_prompt.clone(),
                                selected_variables_for_prompt.clone(),
                                trimmed.to_string(),
                            );
                        }
                    }
                }
            })
            .build();

        let workspace_loading_for_buttons = self.workspace_loading.clone();
        let workspace_path_for_buttons = self.workspace_path.clone();
        let tracked_files_for_buttons = self.tracked_files.clone();
        let selected_variables_for_buttons = self.selected_variables.clone();

        let quick_buttons = Row::new().s(Gap::new().x(SPACING_4)).items(
            ["workspace_a", "workspace_b"]
                .into_iter()
                .zip(
                    [
                        "test_files/my_workspaces/workspace_a",
                        "test_files/my_workspaces/workspace_b",
                    ]
                    .into_iter(),
                )
                .map(|(label, path)| {
                    let workspace_loading = workspace_loading_for_buttons.clone();
                    let workspace_path = workspace_path_for_buttons.clone();
                    let tracked_files = tracked_files_for_buttons.clone();
                    let selected_variables = selected_variables_for_buttons.clone();

                    button()
                        .label(label)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Small)
                        .on_press(move || {
                            if workspace_loading.lock_ref().clone() {
                                return;
                            }
                            NovyWaveApp::start_workspace_switch(
                                workspace_loading.clone(),
                                workspace_path.clone(),
                                tracked_files.clone(),
                                selected_variables.clone(),
                                path.to_string(),
                            );
                        })
                        .build()
                        .into_element()
                }),
        );

        let background_color_signal = self.config.theme_actor.signal().map(|theme| match theme {
            shared::Theme::Light => "oklch(99% 0.025 255)",
            shared::Theme::Dark => "oklch(68% 0.025 255)",
        });

        Row::new()
            .s(Width::fill())
            .s(Padding::new().x(SPACING_6).y(SPACING_4))
            .s(Background::new().color_signal(background_color_signal))
            .s(Align::new().center_y())
            .item(workspace_label)
            .item(
                El::new()
                    .s(Padding::new().left(SPACING_4))
                    .child(loading_indicator),
            )
            .item(
                Row::new()
                    .s(Padding::new().left(SPACING_6))
                    .s(Gap::new().x(SPACING_4))
                    .item(open_workspace_button)
                    .item(quick_buttons),
            )
            .item(El::new().s(Width::growable()))
            .item(crate::action_buttons::theme_toggle_button(&self.config))
            .item(
                El::new()
                    .s(Padding::new().left(SPACING_4))
                    .child(crate::action_buttons::dock_toggle_button(&self.config)),
            )
    }

    fn workspace_display_name(path: &str) -> String {
        path.rsplit(|c| c == '/' || c == '\\')
            .find(|segment| !segment.is_empty())
            .unwrap_or(path)
            .to_string()
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

        workspace_loading.set(true);
        workspace_path.set(None);
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
