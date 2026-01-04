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
use std::sync::Arc;
// use crate::platform::CurrentPlatform; // not needed after silencing emit_trace
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

/// Message router for backend DownMsg handling
/// Uses direct method calls instead of relay pub/sub
#[derive(Clone)]
pub struct ConnectionMessageActor {
    // Task handles message processing (None until start() is called)
    _message_task: Option<Arc<TaskHandle>>,

    // TrackedFiles reference for reload handling
    _tracked_files: TrackedFiles,
}

pub(crate) fn emit_trace(_target: &str, _message: impl Into<String>) {
    // Silenced in normal builds to keep logs clean.
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

#[cfg(NOVYWAVE_PLATFORM = "WEB")]
fn apply_scrollbar_colors(
    raw_el: RawHtmlEl<web_sys::HtmlElement>,
) -> RawHtmlEl<web_sys::HtmlElement> {
    use moonzoon_novyui::tokens::color::{primary_3, primary_6};
    raw_el.style_signal(
        "scrollbar-color",
        primary_6()
            .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
            .flatten(),
    )
}

#[cfg(not(NOVYWAVE_PLATFORM = "WEB"))]
fn apply_scrollbar_colors(
    raw_el: RawHtmlEl<web_sys::HtmlElement>,
) -> RawHtmlEl<web_sys::HtmlElement> {
    raw_el
}

impl ConnectionMessageActor {
    /// Create ConnectionMessageActor - NO message processing yet.
    /// Call `start()` after all domains are ready.
    pub fn new_pending(tracked_files: TrackedFiles) -> Self {
        Self {
            _message_task: None,
            _tracked_files: tracked_files,
        }
    }

    /// Start message processing with DIRECT method calls.
    /// This eliminates timing dependencies - no need to wait for subscribers.
    pub fn start(
        &mut self,
        down_msg_stream: impl futures::stream::Stream<Item = DownMsg> + Unpin + 'static,
        workspace_path: Mutable<Option<String>>,
        workspace_loading: Mutable<bool>,
        default_workspace_path: Mutable<Option<String>>,
        tracked_files: TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
        waveform_timeline: crate::visualizer::timeline::WaveformTimeline,
        config: crate::config::AppConfig,
        connection: std::sync::Arc<SendWrapper<Connection<UpMsg, DownMsg>>>,
    ) {
        let tracked_files_for_reload = self._tracked_files.clone();

        let connection_for_init = connection.clone();
        let message_task = Arc::new(Task::start_droppable(async move {
            let mut stream = down_msg_stream;
            loop {
                match stream.next().await {
                    Some(down_msg) => {
                        match down_msg {
                            DownMsg::ConfigLoaded(loaded_config) => {
                                workspace_loading.set(false);
                                // Phase 3: Apply config directly to state
                                config.restore_config(
                                    loaded_config.clone(),
                                    &selected_variables,
                                );
                                // Phase 4: Trigger actual loading DIRECTLY to backend (no relays!)
                                config.complete_initialization(
                                    &connection_for_init,
                                    &tracked_files,
                                    &selected_variables,
                                    loaded_config.workspace.opened_files.clone(),
                                    loaded_config.workspace.load_files_expanded_directories.clone(),
                                ).await;
                            }
                            DownMsg::WorkspaceLoaded {
                                root,
                                default_root,
                                config: workspace_config,
                            } => {
                                emit_trace(
                                    "workspace_loaded_event",
                                    format!("root={root} default_root={default_root}"),
                                );
                                workspace_loading.set(false);
                                workspace_path.set(Some(root.clone()));
                                default_workspace_path.set(Some(default_root.clone()));
                                tracked_files.clear_all_files();
                                selected_variables.clear_selection();
                                // Apply workspace config
                                config.restore_config(workspace_config, &selected_variables);
                            }
                            DownMsg::ConfigError(error) => {
                                workspace_loading.set(false);
                                zoon::println!("Config error: {error}");
                            }
                            DownMsg::ConfigSaved => {
                                // Config saved successfully - no action needed
                            }
                            DownMsg::DirectoryContents { path, items } => {
                                zoon::println!(
                                    "frontend: DirectoryContents path='{}' items={}",
                                    path,
                                    items.len()
                                );
                                config.file_picker_domain.on_directory_contents(path, items);
                            }
                            DownMsg::DirectoryError { path, error } => {
                                zoon::println!(
                                    "frontend: DirectoryError path='{}' err={}",
                                    path,
                                    error
                                );
                                config.file_picker_domain.on_directory_error(path, error);
                            }
                            // FileLoaded, ParsingError, ParsingStarted are handled directly
                            // in the Connection callback via tf.update_file_state()
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
                                    waveform_timeline.apply_cursor_values(values);
                                }
                            }
                            DownMsg::UnifiedSignalResponse {
                                request_id,
                                signal_data,
                                cursor_values,
                                ..
                            } => {
                                zoon::println!("[CONN] UnifiedSignalResponse: request_id={request_id} signal_data={} cursor_values={}",
                                    signal_data.len(), cursor_values.len());
                                waveform_timeline.apply_unified_signal_response(
                                    &request_id,
                                    signal_data,
                                    cursor_values,
                                );
                            }
                            DownMsg::UnifiedSignalError { request_id, error } => {
                                waveform_timeline.handle_unified_signal_error(&request_id, &error);
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
                            DownMsg::TestNotification { variant, title, message } => {
                                zoon::println!("🔔 FRONTEND: Received test notification: {variant} - {title}: {message}");
                            }
                            _ => {}
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        }));

        self._message_task = Some(message_task);
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
    _workspace_picker_restoring: Mutable<bool>,

    /// Message routing actor (keeps relays alive)
    _connection_message_actor: ConnectionMessageActor,

    /// Synchronizes Files panel scope selection into SelectedVariables domain
    _scope_selection_sync_task: Arc<TaskHandle>,

    _workspace_history_selection_task: Arc<TaskHandle>,
    _workspace_history_expanded_task: Arc<TaskHandle>,
    _workspace_history_scroll_task: Arc<TaskHandle>,
    _workspace_history_restore_task: Arc<TaskHandle>,

    // === UI STATE ===
    /// File picker dialog visibility
    pub file_dialog_visible: Mutable<bool>,
    pub workspace_picker_visible: Mutable<bool>,

    pub workspace_path: Mutable<Option<String>>,
    pub workspace_loading: Mutable<bool>,
    pub default_workspace_path: Mutable<Option<String>>,

    key_repeat_handles: Rc<RefCell<HashMap<KeyAction, Interval>>>,
    debug_notification_task: Rc<RefCell<Option<TaskHandle>>>,
    workspace_switch_task: Rc<RefCell<Option<TaskHandle>>>,
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
                selected_variables.select_scope(Some(cleaned_scope));
            }
            None => {
                selected_variables.select_scope(None);
            }
        }

        selected_variables.set_tree_selection(selection_set);
    }

    /// Create new NovyWaveApp with proper async initialization
    ///
    /// This replaces the complex global domain initialization from main.rs
    pub async fn new() -> Self {
        // init

        // Load fonts first
        Self::load_and_register_fonts().await;

        let tracked_files = TrackedFiles::new();
        let selected_variables = SelectedVariables::new();

        // ✅ STEP 1: Create connection and PENDING actor (relays only, no processing)
        // The mpsc channel buffers messages until actor.start() is called
        let (connection, mut connection_message_actor, down_msg_receiver) =
            Self::create_connection_and_pending_actor(tracked_files.clone());

        let workspace_path = Mutable::new(None);
        // Start in loading state until WorkspaceLoaded/ConfigLoaded arrives
        let workspace_loading = Mutable::new(true);
        let default_workspace_path = Mutable::new(None);
        let workspace_picker_visible = Mutable::new(false);

        // Initialize platform layer with the working connection
        let connection_arc = std::sync::Arc::new(connection);
        crate::platform::set_platform_connection(connection_arc.clone());

        // Create a dummy sender for workspace picker domain - its saves are not persisted
        let (workspace_picker_save_sender, _workspace_picker_save_receiver) = futures::channel::mpsc::unbounded::<()>();
        let workspace_picker_domain = crate::config::FilePickerDomain::new(
            IndexSet::new(),
            0,
            workspace_picker_save_sender,
            connection_arc.clone(),
            connection_message_actor.clone(),
        );

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
        let workspace_picker_restoring = Mutable::new(false);

        let workspace_history_selection_task = {
            let selected_signal = workspace_picker_domain.selected_files_vec_signal.clone();
            let config_clone = config.clone();
            let domain_clone = workspace_picker_domain.clone();
            let target_clone = workspace_picker_target.clone();
            Arc::new(Task::start_droppable(async move {
                let mut selection_stream = selected_signal.signal_cloned().to_stream().fuse();
                while let Some(selection) = selection_stream.next().await {
                    emit_trace("workspace_picker_selection", format!("paths={selection:?}"));
                    if let Some(path) = selection.first().cloned() {
                        target_clone.set_neq(Some(path.clone()));
                        config_clone.record_workspace_selection(&path);

                        let expanded_vec = domain_clone
                            .expanded_directories
                            .lock_ref()
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>();
                        let expanded_vec_for_log = expanded_vec.clone();
                        let scroll_current =
                            *domain_clone.scroll_position.lock_ref() as f64;
                        emit_trace(
                            "workspace_picker_selection",
                            format!(
                                "selection={selection:?} expanded_paths={expanded_vec_for_log:?} scroll={scroll_current}"
                            ),
                        );
                        NovyWaveApp::publish_workspace_picker_snapshot(
                            &config_clone,
                            &domain_clone,
                            &target_clone,
                            Some(expanded_vec_for_log),
                        );
                    }
                }
            }))
        };

        let workspace_history_expanded_task = {
            let domain_clone = workspace_picker_domain.clone();
            let config_clone = config.clone();
            let restoring_flag = workspace_picker_restoring.clone();
            let visible_atom = workspace_picker_visible.clone();
            Arc::new(Task::start_droppable(async move {
                use futures::{StreamExt, select};
                // Listen to expanded_directories signal instead of relay
                let mut expanded_stream = domain_clone.expanded_directories_signal().to_stream().fuse();
                let mut visibility_stream = visible_atom.signal().to_stream().fuse();
                let mut is_visible = visible_atom.get_cloned();
                // Skip initial value to avoid persisting on startup
                let _ = expanded_stream.next().await;
                loop {
                    select! {
                        expanded = expanded_stream.next() => {
                            if let Some(expanded_set) = expanded {
                                let expanded_vec: Vec<String> = expanded_set.iter().cloned().collect();
                                let restoring = restoring_flag.get_cloned();
                                emit_trace(
                                    "workspace_history_expanded_actor",
                                    format!(
                                        "paths={expanded_vec:?} restoring={restoring} visible={is_visible}"
                                    ),
                                );
                                // Ignore teardown-driven empty updates when the dialog is no longer visible
                                if !is_visible && expanded_vec.is_empty() {
                                    emit_trace(
                                        "workspace_history_expanded_actor",
                                        "skip_empty_invisible".to_string(),
                                    );
                                    continue;
                                }
                                if restoring {
                                    emit_trace(
                                        "workspace_history_expanded_actor",
                                        "skip_restoring".to_string(),
                                    );
                                    continue;
                                }
                                // Only persist picker expanded paths; do not touch per-workspace state here
                                config_clone.update_workspace_picker_tree_state(expanded_vec.clone());
                            } else {
                                break;
                            }
                        }
                        visible = visibility_stream.next() => {
                            if let Some(visible) = visible {
                                is_visible = visible;
                                emit_trace(
                                    "workspace_history_expanded_actor_visibility",
                                    format!("visible={visible}"),
                                );
                            } else {
                                break;
                            }
                        }
                    }
                }
            }))
        };

        let workspace_history_scroll_task = {
            let domain_clone = workspace_picker_domain.clone();
            let _target_clone = workspace_picker_target.clone();
            let config_clone = config.clone();
            let restoring_flag = workspace_picker_restoring.clone();
            Arc::new(Task::start_droppable(async move {
                let mut scroll_stream = domain_clone
                    .scroll_position
                    .signal()
                    .to_stream()
                    .fuse();
                // No injection here; picker_tree_state is restored before tree builds

                while let Some(position) = scroll_stream.next().await {
                    if restoring_flag.get_cloned() {
                        continue;
                    }

                    let scroll_value = position as f64;
                    emit_trace(
                        "workspace_picker_scroll",
                        format!("scroll_top={scroll_value}"),
                    );
                    // Mirror Load Files dialog: update picker scroll via config; snapshot happens via saver
                    config_clone.update_workspace_picker_scroll(scroll_value);
                }
            }))
        };

        // Removed scroll polling; rely on the same event-driven logic as Load Files dialog

        let workspace_history_restore_task = {
            let history_state = config.workspace_history_state.clone();
            let domain_clone = workspace_picker_domain.clone();
            let visible_atom = workspace_picker_visible.clone();
            let target_clone = workspace_picker_target.clone();
            let config_clone = config.clone();
            let restoring_flag = workspace_picker_restoring.clone();
            Arc::new(Task::start_droppable(async move {
                let mut visibility_stream = visible_atom.signal().to_stream().fuse();
                while let Some(visible) = visibility_stream.next().await {
                    if visible {
                        restoring_flag.set(true);
                        let history = history_state.get_cloned();
                        emit_trace(
                            "workspace_picker_restore",
                            format!("stage=apply history={:?}", history.picker_tree_state),
                        );
                        NovyWaveApp::apply_workspace_picker_tree_state(&history, &domain_clone);
                        let applied_state = domain_clone
                            .expanded_directories
                            .lock_ref()
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>();
                        // Sync applied expansions into history state so scroll writes are accepted.
                        config_clone.update_workspace_picker_tree_state(applied_state.clone());
                        emit_trace(
                            "workspace_picker_restore",
                            format!("stage=post_apply expanded_paths={applied_state:?}"),
                        );
                        let pre_clear_state = domain_clone
                            .expanded_directories
                            .lock_ref()
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>();
                        emit_trace(
                            "workspace_picker_restore",
                            format!("stage=pre_clear expanded_paths={pre_clear_state:?}"),
                        );
                        target_clone.set_neq(None);
                        domain_clone.selected_files_vec_signal.set_neq(Vec::new());
                        domain_clone.clear_file_selection();
                        // Do not publish a snapshot on open; wait for user interaction
                        let post_snapshot_state = domain_clone
                            .expanded_directories
                            .lock_ref()
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>();
                        emit_trace(
                            "workspace_picker_restore",
                            format!("stage=post_snapshot expanded_paths={post_snapshot_state:?}"),
                        );
                        emit_trace(
                            "workspace_picker_restore",
                            format!(
                                "stage=final expanded_paths={:?}",
                                domain_clone
                                    .expanded_directories
                                    .lock_ref()
                                    .iter()
                                    .cloned()
                                    .collect::<Vec<String>>()
                            ),
                        );
                        restoring_flag.set(false);
                    } else {
                        // Dialog is closing: publish a final snapshot so scroll-only changes persist
                        NovyWaveApp::publish_workspace_picker_snapshot(
                            &config_clone,
                            &domain_clone,
                            &target_clone,
                            None,
                        );
                        restoring_flag.set(false);
                    }
                }
            }))
        };

        // Create MaximumTimelineRange standalone actor for centralized range computation
        let maximum_timeline_range = crate::visualizer::timeline::MaximumTimelineRange::new(
            tracked_files.clone(),
            selected_variables.clone(),
        );

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
        let dragging_system = crate::dragging::DraggingSystem::new(config.clone());

        let scope_selection_sync_task = {
            let selected_variables_for_scope = selected_variables.clone();
            let files_selected_scope = config.files_selected_scope.clone();

            Arc::new(Task::start_droppable(async move {
                // Emit initial selection once
                let initial = files_selected_scope.lock_ref().to_vec();
                NovyWaveApp::propagate_scope_selection(initial, &selected_variables_for_scope);

                // Subscribe to vector snapshots
                let mut selection_stream = files_selected_scope
                    .signal_vec_cloned()
                    .to_signal_cloned()
                    .to_stream()
                    .fuse();

                while let Some(current_selection) = selection_stream.next().await {
                    NovyWaveApp::propagate_scope_selection(
                        current_selection,
                        &selected_variables_for_scope,
                    );
                }
            }))
        };

        connection_message_actor.start(
            down_msg_receiver,
            workspace_path.clone(),
            workspace_loading.clone(),
            default_workspace_path.clone(),
            tracked_files.clone(),
            selected_variables.clone(),
            waveform_timeline.clone(),
            config.clone(),
            connection_arc.clone(),
        );

        // Request config loading once; the Web platform gate handles
        // readiness probes, single-flight, and quiet retry during dev-server swaps.
        let _ = <crate::platform::CurrentPlatform as crate::platform::Platform>::send_message(
            shared::UpMsg::LoadConfig,
        )
        .await;

        let file_dialog_visible = Mutable::new(false);
        let key_repeat_handles = Rc::new(RefCell::new(HashMap::new()));
        let debug_notification_task = Rc::new(RefCell::new(None));
        let workspace_switch_task = Rc::new(RefCell::new(None));

        Self::setup_app_coordination(&selected_variables, &config).await;

        // Setup Tauri update event listeners (desktop only - no-op on web)
        crate::platform::setup_update_event_listeners(config.error_display.clone());

        NovyWaveApp {
            tracked_files,
            selected_variables,
            waveform_timeline,
            waveform_canvas,
            config,
            dragging_system,
            connection: connection_arc,
            _connection_message_actor: connection_message_actor,
            _scope_selection_sync_task: scope_selection_sync_task,
            file_dialog_visible,
            workspace_picker_visible,
            workspace_path,
            workspace_loading,
            default_workspace_path,
            workspace_picker_domain,
            workspace_picker_target,
            _workspace_picker_restoring: workspace_picker_restoring,
            key_repeat_handles,
            debug_notification_task,
            workspace_switch_task,
            _workspace_history_selection_task: workspace_history_selection_task,
            _workspace_history_expanded_task: workspace_history_expanded_task,
            _workspace_history_scroll_task: workspace_history_scroll_task,
            _workspace_history_restore_task: workspace_history_restore_task,
        }
    }

    /// Load and register fonts (from main.rs)
    async fn load_and_register_fonts() {
        use zoon::futures_util::future::try_join_all;

        let fonts_result = try_join_all([
            fast2d::fetch_file("/_api/public/fonts/FiraCode-Regular.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Regular.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Bold.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Italic.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-BoldItalic.ttf"),
        ])
        .await;

        match fonts_result {
            Ok(fonts) => {
                let _ = fast2d::register_fonts(fonts);
            }
            Err(err) => {
                zoon::println!("Fonts load skipped (backend not ready yet?): {:?}", err);
            }
        }
    }

    /// Create connection and message actor with PROPER ordering to avoid race conditions.
    /// Returns: (connection, pending_actor, mpsc_receiver)
    ///
    /// Usage:
    /// 1. Call this to get connection and pending actor
    /// 2. Subscribe to actor's relays
    /// 3. Call actor.start(receiver) to begin processing
    ///
    /// The mpsc channel buffers messages until start() is called.
    fn create_connection_and_pending_actor(
        tracked_files: TrackedFiles,
    ) -> (
        SendWrapper<Connection<UpMsg, DownMsg>>,
        ConnectionMessageActor,
        futures::channel::mpsc::UnboundedReceiver<DownMsg>,
    ) {
        use futures::channel::mpsc;

        let (down_msg_sender, down_msg_receiver) = mpsc::unbounded::<DownMsg>();
        let tf = tracked_files.clone();

        // Create actor with relays only - NO processing yet
        let connection_message_actor = ConnectionMessageActor::new_pending(tracked_files.clone());

        // Connection sends directly to mpsc channel (channel buffers messages)
        let connection = Connection::new({
            let sender = down_msg_sender;
            move |down_msg, _| {
                zoon::println!("connection: received DownMsg {:?}", down_msg);
                crate::platform::notify_server_alive();

                // Handle file state updates immediately via direct method calls
                match &down_msg {
                    DownMsg::FileLoaded { file_id, hierarchy } => {
                        if let Some(loaded_file) = hierarchy.files.first() {
                            tf.update_file_state(
                                file_id.clone(),
                                shared::FileState::Loaded(loaded_file.clone()),
                            );
                        }
                    }
                    DownMsg::ParsingStarted { file_id, .. } => {
                        tf.update_file_state(
                            file_id.clone(),
                            shared::FileState::Loading(shared::LoadingStatus::Parsing),
                        );
                    }
                    DownMsg::ParsingError { file_id, error: _ } => {
                        tf.update_file_state(
                            file_id.clone(),
                            shared::FileState::Failed(shared::FileError::FileNotFound {
                                path: file_id.clone(),
                            }),
                        );
                    }
                    _ => {}
                }

                // Send to mpsc channel (buffered until actor.start() is called)
                let _ = sender.unbounded_send(down_msg);
            }
        });

        (SendWrapper::new(connection), connection_message_actor, down_msg_receiver)
    }

    /// Setup app-level coordination
    async fn setup_app_coordination(selected_variables: &SelectedVariables, config: &AppConfig) {
        // Restore selected variables from config
        if !config.loaded_selected_variables.is_empty() {
            selected_variables.restore_variables(config.loaded_selected_variables.clone());
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
                Background::new().color_signal(self.config.theme.signal().map(|theme| {
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
                let app_config = self.config.clone();
                let timeline = self.waveform_timeline.clone();
                let dragging_system_for_events = dragging_system.clone();
                let key_repeat_handles = self.key_repeat_handles.clone();
                let debug_notification_task = self.debug_notification_task.clone();

                move |raw_el| {
                    let app_config_for_keydown = app_config.clone();
                    let repeat_handles_for_down = key_repeat_handles.clone();
                    let repeat_handles_for_up = key_repeat_handles.clone();
                    let debug_notification_task_for_keydown = debug_notification_task.clone();
                    let timeline_for_keydown = timeline.clone();
                    let timeline_for_keyup = timeline.clone();
                    let raw_el = raw_el.global_event_handler_with_options(
                        EventOptions::new().preventable(),
                        move |event: KeyDown| {
                            let should_handle_shortcuts = if let Some(window) = web_sys::window() {
                                if let Some(document) = window.document() {
                                    if let Some(active_element) = document.active_element() {
                                        let tag_name = active_element.tag_name().to_lowercase();
                                        !matches!(tag_name.as_str(), "input" | "textarea")
                                    } else {
                                        true
                                    }
                                } else {
                                    true
                                }
                            } else {
                                true
                            };

                            if !should_handle_shortcuts {
                                return;
                            }

                            if event.shift_key() {
                                timeline_for_keydown.set_shift_active(true);
                            }

                            if event.ctrl_key() {
                                match event.key().as_str() {
                                    "t" | "T" => {
                                        event.prevent_default();
                                        app_config_for_keydown.toggle_theme();
                                    }
                                    "d" | "D" => {
                                        event.prevent_default();
                                        app_config_for_keydown.toggle_dock_mode();
                                    }
                                    _ => {}
                                }
                            } else {
                                match event.key().as_str() {
                                    "q" | "Q" => {
                                        event.prevent_default();
                                        if event.shift_key() {
                                            stop_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::CursorLeft,
                                            );
                                            timeline_for_keydown.jump_to_previous_transition();
                                        } else {
                                            timeline_for_keydown.move_cursor_left();
                                            if !event.repeat() {
                                                let tl = timeline_for_keydown.clone();
                                                start_key_repeat(
                                                    &repeat_handles_for_down,
                                                    KeyAction::CursorLeft,
                                                    KEY_REPEAT_INTERVAL_MS,
                                                    move || tl.move_cursor_left(),
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
                                            timeline_for_keydown.jump_to_next_transition();
                                        } else {
                                            timeline_for_keydown.move_cursor_right();
                                            if !event.repeat() {
                                                let tl = timeline_for_keydown.clone();
                                                start_key_repeat(
                                                    &repeat_handles_for_down,
                                                    KeyAction::CursorRight,
                                                    KEY_REPEAT_INTERVAL_MS,
                                                    move || tl.move_cursor_right(),
                                                );
                                            }
                                        }
                                    }

                                    "a" | "A" => {
                                        event.prevent_default();
                                        let faster = timeline_for_keydown.shift_active.get_cloned();
                                        timeline_for_keydown.pan_left(faster);
                                        if !event.repeat() {
                                            let tl = timeline_for_keydown.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::PanLeft,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || {
                                                    let faster = tl.shift_active.get_cloned();
                                                    tl.pan_left(faster);
                                                },
                                            );
                                        }
                                    }
                                    "d" | "D" => {
                                        event.prevent_default();
                                        let faster = timeline_for_keydown.shift_active.get_cloned();
                                        timeline_for_keydown.pan_right(faster);
                                        if !event.repeat() {
                                            let tl = timeline_for_keydown.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::PanRight,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || {
                                                    let faster = tl.shift_active.get_cloned();
                                                    tl.pan_right(faster);
                                                },
                                            );
                                        }
                                    }

                                    // Zoom Controls
                                    "w" | "W" => {
                                        event.prevent_default();
                                        let faster = timeline_for_keydown.shift_active.get_cloned();
                                        timeline_for_keydown.zoom_in(faster);
                                        if !event.repeat() {
                                            let tl = timeline_for_keydown.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::ZoomIn,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || {
                                                    let faster = tl.shift_active.get_cloned();
                                                    tl.zoom_in(faster);
                                                },
                                            );
                                        }
                                    }
                                    "s" | "S" => {
                                        event.prevent_default();
                                        let faster = timeline_for_keydown.shift_active.get_cloned();
                                        timeline_for_keydown.zoom_out(faster);
                                        if !event.repeat() {
                                            let tl = timeline_for_keydown.clone();
                                            start_key_repeat(
                                                &repeat_handles_for_down,
                                                KeyAction::ZoomOut,
                                                KEY_REPEAT_INTERVAL_MS,
                                                move || {
                                                    let faster = tl.shift_active.get_cloned();
                                                    tl.zoom_out(faster);
                                                },
                                            );
                                        }
                                    }

                                    // Reset Controls
                                    "z" | "Z" => {
                                        event.prevent_default();
                                        timeline_for_keydown.reset_zoom_center();
                                    }
                                    "r" | "R" => {
                                        event.prevent_default();
                                        timeline_for_keydown.reset_zoom();
                                    }
                                    "t" | "T" => {
                                        event.prevent_default();
                                        timeline_for_keydown.toggle_tooltip();
                                    }

                                    // Debug: Trigger test notifications
                                    "n" | "N" => {
                                        event.prevent_default();
                                        let config = app_config_for_keydown.clone();
                                        let handle = Task::start_droppable(async move {
                                            crate::error_display::trigger_test_notifications(&config).await;
                                        });
                                        // Store handle to cancel previous task if retriggered
                                        *debug_notification_task_for_keydown.borrow_mut() = Some(handle);
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
                                timeline_for_keyup.set_shift_active(false);
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
                let workspace_switch_task = self.workspace_switch_task.clone();
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
                        workspace_switch_task.clone(),
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

        let background_color_signal = self.config.theme.signal().map(|theme| match theme {
            shared::Theme::Light => Some("oklch(96% 0.015 255)"),
            shared::Theme::Dark => Some("oklch(26% 0.02 255)"),
        });

        let divider_color_signal = self.config.theme.signal().map(|theme| match theme {
            shared::Theme::Light => Border::new().width(1).color("oklch(82% 0.015 255)"),
            shared::Theme::Dark => Border::new().width(1).color("oklch(40% 0.02 255)"),
        });

        let path_signal = {
            let server_ready_signal = crate::platform::server_ready_signal();
            map_ref! {
                let current = self.workspace_path.signal_cloned(),
                let default = self.default_workspace_path.signal_cloned(),
                let loading = self.workspace_loading.signal(),
                let server_ready = server_ready_signal => {
                    if let Some(path) = current.clone() {
                        if let Some(def) = default.clone() {
                            if def == path {
                                format!("Default ({})", path)
                            } else { path }
                        } else { path }
                    } else if let Some(default_path) = default.clone() {
                        format!("Default ({})", default_path)
                    } else if !*server_ready || *loading {
                        String::from("Loading workspace…")
                    } else { String::from("No workspace") }
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
        _tracked_files: TrackedFiles,
        _selected_variables: SelectedVariables,
        app_config: AppConfig,
        path: String,
        workspace_switch_task: Rc<RefCell<Option<TaskHandle>>>,
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

        let previous_path = workspace_path.lock_ref().clone();
        workspace_loading.set(true);
        // Optimistic update: reflect the user's choice immediately in the input.
        workspace_path.set(Some(trimmed.clone()));

        let handle = Task::start_droppable({
            let request_path = trimmed.clone();
            let workspace_loading = workspace_loading.clone();
            let workspace_path_for_revert = workspace_path.clone();
            // Pause config saves until new ConfigLoaded arrives
            app_config.mark_workspace_switching();
            async move {
                // Keep it simple: send once; on error, one fallback after a brief wait.
                let first =
                    <crate::platform::CurrentPlatform as crate::platform::Platform>::send_message(
                        shared::UpMsg::SelectWorkspace {
                            root: request_path.clone(),
                        },
                    )
                    .await;
                if first.is_err() {
                    zoon::Timer::sleep(500).await;
                    let second = <crate::platform::CurrentPlatform as crate::platform::Platform>::send_message(
                        shared::UpMsg::SelectWorkspace { root: request_path.clone() }
                    ).await;
                    if second.is_err() {
                        // Revert optimistic input and stop loading
                        workspace_loading.set(false);
                        workspace_path_for_revert.set(previous_path.clone());
                        zoon::println!("Workspace switch failed twice: {}", request_path);
                        return;
                    }
                }
                // Watchdog: if no WorkspaceLoaded arrives within 6s, clear loading so UI doesn't get stuck.
                // NOTE: We await directly here instead of spawning a separate task that would be dropped.
                Timer::sleep(6000).await;
                let still_loading = *workspace_loading.lock_ref();
                let current = workspace_path_for_revert.lock_ref().clone();
                if still_loading && current.as_ref().map(|p| p == &request_path).unwrap_or(false) {
                    zoon::println!("ERROR: Workspace load timeout after 6s for: {}", request_path);
                    workspace_loading.set(false);
                }
            }
        });
        // Store handle to cancel previous task if retriggered
        *workspace_switch_task.borrow_mut() = Some(handle);
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
            .expanded_directories
            .set_neq(expanded_set);
        domain.scroll_position.set_neq(scroll_value);
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
                .expanded_directories
                .set_neq(expanded_set);
            domain.scroll_position.set_neq(scroll_value);
        }
    }

    fn publish_workspace_picker_snapshot(
        config: &AppConfig,
        domain: &crate::config::FilePickerDomain,
        _target: &Mutable<Option<String>>,
        known_expanded: Option<Vec<String>>,
    ) {
        let expanded_vec = known_expanded.unwrap_or_else(|| {
            domain
                .expanded_directories
                .lock_ref()
                .iter()
                .cloned()
                .collect::<Vec<String>>()
        });
        if expanded_vec.is_empty() {
            emit_trace(
                "workspace_picker_snapshot",
                "skipped empty expanded_paths".to_string(),
            );
            return;
        }
        // Scroll persistence is event-driven via workspace_history_scroll_actor.
        emit_trace(
            "workspace_picker_snapshot",
            format!("expanded_paths={expanded_vec:?}"),
        );
        config.update_workspace_picker_tree_state(expanded_vec.clone());
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
    workspace_picker_visible: Mutable<bool>,
    workspace_loading: Mutable<bool>,
    workspace_path: Mutable<Option<String>>,
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    default_workspace_path: Mutable<Option<String>>,
    app_config: AppConfig,
    workspace_picker_target: Mutable<Option<String>>,
    workspace_picker_domain: crate::config::FilePickerDomain,
    workspace_switch_task: Rc<RefCell<Option<TaskHandle>>>,
) -> impl Element {
    use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_4, neutral_8, neutral_11};
    use moonzoon_novyui::tokens::theme::{Theme, theme};

    // Do not snapshot default path here; read it on demand so filtering reflects
    // the latest default even if it was set moments after dialog opens.

    // Reset any stale selection when the dialog opens so the user
    // can pick a new workspace immediately.
    workspace_picker_domain.clear_file_selection();
    workspace_picker_target.set_neq(None);

    // Apply saved picker tree state (expanded paths + scroll) before building the tree
    // to prevent the initialization actor from injecting default entries.
    {
        let history_snapshot = app_config.workspace_history_state.get_cloned();
        NovyWaveApp::apply_workspace_picker_tree_state(&history_snapshot, &workspace_picker_domain);
    }

    let open_action = {
        let workspace_loading = workspace_loading.clone();
        let workspace_path = workspace_path.clone();
        let tracked_files = tracked_files.clone();
        let selected_variables = selected_variables.clone();
        let workspace_picker_visible = workspace_picker_visible.clone();
        let workspace_picker_domain = workspace_picker_domain.clone();
        let app_config_for_open = app_config.clone();
        let workspace_picker_target_for_open = workspace_picker_target.clone();
        let workspace_switch_task_for_open = workspace_switch_task.clone();
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
            // Persist the latest picker tree snapshot (expanded + scroll) before switching
            NovyWaveApp::publish_workspace_picker_snapshot(
                &app_config_for_open,
                &workspace_picker_domain,
                &workspace_picker_target_for_open,
                None,
            );
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
                app_config_for_open.clone(),
                path,
                workspace_switch_task_for_open.clone(),
            );
            workspace_picker_visible.set(false);
        }
    };
    let open_action = Rc::new(open_action);

    // Keep a local recent-paths mirror that only updates when the list actually changes.
    let recent_paths_vec = MutableVec::<String>::new();
    let recent_paths_task = Arc::new(Task::start_droppable({
        let history_state = app_config.workspace_history_state.clone();
        let recent_paths_vec = recent_paths_vec.clone();
        async move {
            use futures::StreamExt;
            let mut stream = history_state.signal_cloned().to_stream();
            let mut prev: Option<Vec<String>> = None;
            while let Some(history) = stream.next().await {
                let recents = history.recent_paths.clone();
                if prev.as_ref().map(|p| p == &recents).unwrap_or(false) {
                    continue;
                }
                prev = Some(recents.clone());
                recent_paths_vec.lock_mut().replace_cloned(recents);
            }
        }
    }));

    El::new()
        .s(Background::new().color_signal(theme().map(|t| match t {
            Theme::Light => "rgba(255, 255, 255, 0.85)",
            Theme::Dark => "rgba(0, 0, 0, 0.85)",
        })))
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .s(Padding::all(40))
        .after_remove(move |_| { drop(recent_paths_task); })
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
            let app_config_for_close = app_config.clone();
            let domain_for_close = workspace_picker_domain.clone();
            let target_for_close = workspace_picker_target.clone();
            move |raw_el| {
                raw_el
                    .event_handler({
                        let app_config_for_close = app_config_for_close.clone();
                        let domain_for_close = domain_for_close.clone();
                        let target_for_close = target_for_close.clone();
                        let workspace_picker_visible_for_click =
                            workspace_picker_visible_for_click.clone();
                        move |event: Click| {
                            NovyWaveApp::publish_workspace_picker_snapshot(
                                &app_config_for_close,
                                &domain_for_close,
                                &target_for_close,
                                None,
                            );
                            workspace_picker_visible_for_click.set(false);
                            event.stop_propagation();
                        }
                    })
                    .global_event_handler({
                        let app_config_for_close = app_config_for_close.clone();
                        let domain_for_close = domain_for_close.clone();
                        let target_for_close = target_for_close.clone();
                        let workspace_picker_visible_for_key =
                            workspace_picker_visible_for_key.clone();
                        move |event: KeyDown| {
                            if event.key() == "Escape" {
                                NovyWaveApp::publish_workspace_picker_snapshot(
                                    &app_config_for_close,
                                    &domain_for_close,
                                    &target_for_close,
                                    None,
                                );
                                workspace_picker_visible_for_key.set(false);
                            }
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

                            if let Some(default_path) = default_workspace_path.lock_ref().clone() {
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

                                    let workspace_switch_task_for_default = workspace_switch_task.clone();
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
                                                            workspace_picker_domain.clear_file_selection();
                                                            workspace_picker_domain.select_file(action_path.clone());
                                                            NovyWaveApp::start_workspace_switch(
                                                                workspace_loading.clone(),
                                                                workspace_path.clone(),
                                                                tracked_files.clone(),
                                                                selected_variables.clone(),
                                                                app_config.clone(),
                                                                action_path.clone(),
                                                                workspace_switch_task_for_default.clone(),
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
                                        let recent_paths_vec = recent_paths_vec.clone();
                                        let workspace_switch_task = workspace_switch_task.clone();
                                        recent_paths_vec
                                            .signal_vec_cloned()
                                            .to_signal_cloned()
                                            .map(move |recents| {
                                                let current_workspace = workspace_path.lock_ref().clone();
                                                let default_workspace_snapshot = default_workspace_path.lock_ref().clone();

                                                let filtered: Vec<String> = recents
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
                                                    let workspace_switch_task_first = workspace_switch_task.clone();

                                                    let mut column = Column::new()
                                                        .s(Gap::new().y(SPACING_2))
                                                        .item(
                                                            Row::new()
                                                                .s(Width::fill())
                                                                .s(Align::new().center_y())
                                                                .item(
                                                                    button()
                                                                        .label(first_path.clone())
                                                                        .variant(ButtonVariant::Ghost)
                                                                        .size(ButtonSize::Small)
                                                                        .on_press({
                                                                            let app_config_sel = app_config_first.clone();
                                                                            let first_path_sel = first_path.clone();
                                                                            let workspace_picker_target_sel = workspace_picker_target_first.clone();
                                                                            let workspace_picker_domain_sel = workspace_picker_domain_first.clone();
                                                                            let workspace_loading_sel = workspace_loading_first.clone();
                                                                            let workspace_path_sel = workspace_path_first.clone();
                                                                            let tracked_files_sel = tracked_files_first.clone();
                                                                            let selected_variables_sel = selected_variables_first.clone();
                                                                            let workspace_picker_visible_sel = workspace_picker_visible_first.clone();
                                                                            let workspace_switch_task_sel = workspace_switch_task_first.clone();
                                                                            move || {
                                                                                app_config_sel.record_workspace_selection(&first_path_sel);
                                                                                workspace_picker_target_sel.set_neq(Some(first_path_sel.clone()));
                                                                                let history_snapshot = app_config_sel.workspace_history_state.get_cloned();
                                                                                NovyWaveApp::apply_workspace_history_state(&history_snapshot, &first_path_sel, &workspace_picker_domain_sel);
                                                                                workspace_picker_domain_sel.clear_file_selection();
                                                                                workspace_picker_domain_sel.select_file(first_path_sel.clone());
                                                                                NovyWaveApp::start_workspace_switch(
                                                                                    workspace_loading_sel.clone(),
                                                                                    workspace_path_sel.clone(),
                                                                                    tracked_files_sel.clone(),
                                                                                    selected_variables_sel.clone(),
                                                                                    app_config_sel.clone(),
                                                                                    first_path_sel.clone(),
                                                                                    workspace_switch_task_sel.clone(),
                                                                                );
                                                                                workspace_picker_visible_sel.set(false);
                                                                            }
                                                                        })
                                                                        .build()
                                                                )
                                                                .item(El::new().s(Width::fill()))
                                                                .item(
                                                                    button()
                                                                        .left_icon(IconName::X)
                                                                        .variant(ButtonVariant::DestructiveGhost)
                                                                        .size(ButtonSize::Small)
                                                                        .custom_padding(2, 2)
                                                                        .on_press({
                                                                            let app_config_rm = app_config_first.clone();
                                                                            let path_rm = first_path.clone();
                                                                            move || {
                                                                                app_config_rm.remove_recent_workspace(&path_rm);
                                                                            }
                                                                        })
                                                                        .build()
                                                                )
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
                                                        let workspace_switch_task = workspace_switch_task.clone();
                                                        column = column.item(
                                                            Row::new()
                                                                .s(Width::fill())
                                                                .s(Align::new().center_y())
                                                                .item(
                                                                    button()
                                                                        .label(path.clone())
                                                                        .variant(ButtonVariant::Ghost)
                                                                        .size(ButtonSize::Small)
                                                                        .on_press({
                                                                            let app_config_sel = app_config.clone();
                                                                            let path_sel = path.clone();
                                                                            let workspace_picker_target_sel = workspace_picker_target.clone();
                                                                            let workspace_picker_domain_sel = workspace_picker_domain.clone();
                                                                            let workspace_loading_sel = workspace_loading.clone();
                                                                            let workspace_path_sel = workspace_path.clone();
                                                                            let tracked_files_sel = tracked_files.clone();
                                                                            let selected_variables_sel = selected_variables.clone();
                                                                            let workspace_picker_visible_sel = workspace_picker_visible.clone();
                                                                            let workspace_switch_task_sel = workspace_switch_task.clone();
                                                                            move || {
                                                                                app_config_sel.record_workspace_selection(&path_sel);
                                                                                workspace_picker_target_sel.set_neq(Some(path_sel.clone()));
                                                                                let history_snapshot = app_config_sel.workspace_history_state.get_cloned();
                                                                                NovyWaveApp::apply_workspace_history_state(&history_snapshot, &path_sel, &workspace_picker_domain_sel);
                                                                                workspace_picker_domain_sel.clear_file_selection();
                                                                                workspace_picker_domain_sel.select_file(path_sel.clone());
                                                                                NovyWaveApp::start_workspace_switch(
                                                                                    workspace_loading_sel.clone(),
                                                                                    workspace_path_sel.clone(),
                                                                                    tracked_files_sel.clone(),
                                                                                    selected_variables_sel.clone(),
                                                                                    app_config_sel.clone(),
                                                                                    path_sel.clone(),
                                                                                    workspace_switch_task_sel.clone(),
                                                                                );
                                                                                workspace_picker_visible_sel.set(false);
                                                                            }
                                                                        })
                                                                        .build()
                                                                )
                                                                .item(El::new().s(Width::fill()))
                                                                .item(
                                                                    button()
                                                                        .left_icon(IconName::X)
                                                                        .variant(ButtonVariant::DestructiveGhost)
                                                                        .size(ButtonSize::Small)
                                                                        .custom_padding(2, 2)
                                                                        .on_press({
                                                                            let app_config_rm = app_config.clone();
                                                                            let path_rm = path.clone();
                                                                            move || {
                                                                                app_config_rm.remove_recent_workspace(&path_rm);
                                                                            }
                                                                        })
                                                                        .build()
                                                                )
                                                        );
                                                    }

                                                    column.into_raw_el()
                                                }
                                            })
                                    })
                                );
                            top_sections.push(recent_section.into_raw_el());

                            let scroll_task_handle: std::sync::Arc<std::sync::Mutex<Option<zoon::TaskHandle>>> =
                                std::sync::Arc::new(std::sync::Mutex::new(None));
                            let scroll_task_handle_for_insert = scroll_task_handle.clone();
                            let scroll_task_handle_for_remove = scroll_task_handle.clone();

                            let tree_scroll_container = El::new()
                                .s(Height::fill())
                                .s(Width::fill())
                                .s(Scrollbars::both())
                                .viewport_y_signal({
                                    let scroll_position_actor =
                                        workspace_picker_domain.scroll_position.clone();
                                    zoon::map_ref! {
                                        let position = scroll_position_actor.signal() => {
                                            *position
                                        }
                                    }
                                })
                                .update_raw_el({
                                    let domain_for_scroll = workspace_picker_domain.clone();
                                    let app_config_for_scroll = app_config.clone();
                                    move |raw_el| {
                                        let dom_element = raw_el.dom_element();
                                        dom_element
                                            .set_attribute("data-scroll-container", "workspace-picker")
                                            .unwrap();
                                        // Attach scroll listener like in Load Files dialog
                                        let scroll_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                                            if let Some(target) = event.current_target() {
                                                if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                                                    let top = element.scroll_top();
                                                    crate::app::emit_trace(
                                                        "workspace_picker_dom_scroll",
                                                        format!("top={}", top),
                                                    );
                                                    // Direct method call instead of relay
                                                    domain_for_scroll.set_scroll_position(top);
                                                    // Persist picker scroll immediately
                                                    app_config_for_scroll.update_workspace_picker_scroll(top as f64);
                                                } else {
                                                    crate::app::emit_trace(
                                                        "workspace_picker_dom_scroll",
                                                        "no_element".to_string(),
                                                    );
                                                }
                                            } else {
                                                crate::app::emit_trace(
                                                    "workspace_picker_dom_scroll",
                                                    "no_target".to_string(),
                                                );
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
                                            .apply(|raw_el| apply_scrollbar_colors(raw_el))
                                    }
                                })
                                .after_insert({
                                    let scroll_position_mutable =
                                        workspace_picker_domain.scroll_position.clone();
                            move |_element| {
                                let handle = Task::start_droppable({
                                    let scroll_position_mutable = scroll_position_mutable.clone();
                                    async move {
                                        Task::next_macro_tick().await;
                                        let position = scroll_position_mutable.get();
                                        if let Some(window) = web_sys::window() {
                                            if let Some(document) = window.document() {
                                                if let Ok(Some(element)) = document
                                                    .query_selector(
                                                        "[data-scroll-container='workspace-picker']",
                                                    )
                                                {
                                                    if position > 0 {
                                                        element.set_scroll_top(position);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                                *scroll_task_handle_for_insert.lock().unwrap() = Some(handle);
                            }
                        })
                                .after_remove({
                                    move |_| {
                                        drop(scroll_task_handle_for_remove.lock().unwrap().take());
                                    }
                                })
                                .child(workspace_picker_tree(
                                    app_config.clone(),
                                    workspace_picker_target.clone(),
                                    workspace_picker_domain.clone(),
                                ));

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
                                        let app_config_cancel = app_config.clone();
                                        let domain_cancel = workspace_picker_domain.clone();
                                        let target_cancel = workspace_picker_target.clone();
                                        move || {
                                            NovyWaveApp::publish_workspace_picker_snapshot(
                                                &app_config_cancel,
                                                &domain_cancel,
                                                &target_cancel,
                                                None,
                                            );
                                            workspace_picker_visible.set(false)
                                        }
                                    })
                                    .build()
                            )
                                .item(
                                    button()
                                        .label("Open")
                                        .variant(ButtonVariant::Primary)
                                        .size(ButtonSize::Small)
                                        .disabled_signal(
                                            workspace_picker_domain
                                                .selected_files_vec_signal
                                                .signal_cloned()
                                                .map(|selected| selected.is_empty())
                                        )
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

fn workspace_picker_tree(
    app_config: AppConfig,
    workspace_picker_target: Mutable<Option<String>>,
    domain: crate::config::FilePickerDomain,
) -> impl Element {
    use crate::file_picker::initialize_directories_and_request_contents;
    use moonzoon_novyui::tokens::color::neutral_8;

    let initialization_actor = initialize_directories_and_request_contents(&domain);
    let cache_signal = domain.directory_cache_signal();

    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .after_remove(move |_| {
            drop(initialization_actor);
        })
        .child_signal({
            let domain_for_treeview = domain.clone();
            let app_config_for_tree = app_config.clone();
            let target_for_tree = workspace_picker_target.clone();
            cache_signal.map(move |cache| {
                if cache.contains_key("/") {
                    let tree_data = workspace_build_tree_data("/", &cache);
                    // Single Source of Truth: use domain's Mutable directly
                    let external_expanded = domain_for_treeview.expanded_directories.clone();
                    let scroll_position_actor = domain_for_treeview.scroll_position.clone();

                    // Persist expanded paths to global history whenever they change.
                    // Skip the very first sync to avoid writing initial state.
                    let persist_task = Arc::new(Task::start_droppable({
                        let external_for_persist = external_expanded.clone();
                        let config_for_persist = app_config_for_tree.clone();
                        async move {
                            let mut is_first = true;
                            let mut stream = external_for_persist.signal_cloned().to_stream();
                            while let Some(set) = stream.next().await {
                                let vec = set.iter().cloned().collect::<Vec<String>>();
                                if is_first {
                                    is_first = false;
                                    continue;
                                }
                                config_for_persist.update_workspace_picker_tree_state(vec.clone());
                            }
                        }
                    }));

                    let scroll_task_handle: std::sync::Arc<std::sync::Mutex<Option<zoon::TaskHandle>>> =
                        std::sync::Arc::new(std::sync::Mutex::new(None));
                    let scroll_task_handle_for_insert = scroll_task_handle.clone();
                    let scroll_task_handle_for_remove = scroll_task_handle.clone();

                    El::new()
                        .s(Height::fill())
                        .s(Width::fill())
                        .after_insert(move |_element| {
                            // Restore scroll position after tree renders
                            let handle = Task::start_droppable({
                                let scroll_position_mutable = scroll_position_actor.clone();
                                async move {
                                    Task::next_macro_tick().await;
                                    let position = scroll_position_mutable.get();
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
                            *scroll_task_handle_for_insert.lock().unwrap() = Some(handle);
                        })
                        .after_remove(move |_| {
                            drop(persist_task);
                            drop(scroll_task_handle_for_remove.lock().unwrap().take());
                        })
                        .child(
                            tree_view()
                                .data(tree_data)
                                .size(TreeViewSize::Medium)
                                .variant(TreeViewVariant::Basic)
                                .show_icons(true)
                                .show_checkboxes(true)
                                .external_expanded(external_expanded)
                                // Single Source of Truth: use domain's MutableVec directly
                                .external_selected_vec(domain_for_treeview.selected_files.clone())
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
