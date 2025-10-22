use crate::dataflow::{Actor, Relay, relay};
use crate::platform::{CurrentPlatform, Platform};
use crate::visualizer::timeline::TimePs;
use futures::{FutureExt, StreamExt, select};
use moonzoon_novyui::tokens::theme;
use serde::{Deserialize, Serialize};
use shared::UpMsg;
use shared::{
    self, AppConfig as SharedAppConfig, CanonicalPathPayload, DockMode, Theme as SharedTheme,
};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use zoon::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: TimePs,
    pub end: TimePs,
}

async fn compose_shared_app_config(
    theme_actor: &Actor<SharedTheme>,
    dock_mode_actor: &Actor<DockMode>,
    session_actor: &Actor<SessionState>,
    toast_actor: &Actor<u32>,
    file_picker_domain: &FilePickerDomain,
    selected_variables_snapshot_actor: &Actor<Vec<shared::SelectedVariable>>,
    files_width_right_actor: &Actor<f32>,
    files_height_right_actor: &Actor<f32>,
    files_width_bottom_actor: &Actor<f32>,
    files_height_bottom_actor: &Actor<f32>,
    name_column_width_bottom_state: &Mutable<f32>,
    name_column_width_right_state: &Mutable<f32>,
    value_column_width_bottom_state: &Mutable<f32>,
    value_column_width_right_state: &Mutable<f32>,
    timeline_state_actor: &Actor<TimelineState>,
    workspace_history_state: &Mutable<shared::WorkspaceHistory>,
    plugins_state: &Mutable<shared::PluginsSection>,
) -> Option<shared::AppConfig> {
    let theme = theme_actor.signal().to_stream().next().await?;
    let dock_mode = dock_mode_actor.signal().to_stream().next().await?;
    let session = session_actor.signal().to_stream().next().await?;
    let toast_dismiss_ms = toast_actor.signal().to_stream().next().await?;

    let expanded_directories_set = file_picker_domain
        .expanded_directories_signal()
        .to_stream()
        .next()
        .await?;
    let expanded_directories: Vec<String> = expanded_directories_set.into_iter().collect();
    let scroll_position = file_picker_domain
        .scroll_position_signal()
        .to_stream()
        .next()
        .await?;
    let selected_variables_snapshot = selected_variables_snapshot_actor
        .signal()
        .to_stream()
        .next()
        .await?;

    let files_width_right = files_width_right_actor.signal().to_stream().next().await?;
    let files_height_right = files_height_right_actor.signal().to_stream().next().await?;
    let files_width_bottom = files_width_bottom_actor.signal().to_stream().next().await?;
    let files_height_bottom = files_height_bottom_actor
        .signal()
        .to_stream()
        .next()
        .await?;

    let name_column_width_bottom = name_column_width_bottom_state.get_cloned();
    let name_column_width_right = name_column_width_right_state.get_cloned();
    let value_column_width_bottom = value_column_width_bottom_state.get_cloned();
    let value_column_width_right = value_column_width_right_state.get_cloned();
    let timeline_state = timeline_state_actor.signal().to_stream().next().await?;

    let (visible_start_ps, visible_end_ps) = if let Some(range) = timeline_state.visible_range {
        let start = range.start.picoseconds();
        let end = range.end.picoseconds().max(start + 1);
        (start, end)
    } else {
        (0, shared::DEFAULT_TIMELINE_RANGE_PS)
    };

    let cursor_position_ps = timeline_state
        .cursor_position
        .map(|time| time.picoseconds())
        .unwrap_or(visible_start_ps);

    let zoom_center_ps = timeline_state
        .zoom_center
        .map(|time| time.picoseconds())
        .unwrap_or(visible_start_ps);

    let tooltip_enabled = timeline_state.tooltip_enabled;

    let timeline_config = shared::TimelineConfig {
        cursor_position_ps,
        visible_range_start_ps: visible_start_ps,
        visible_range_end_ps: visible_end_ps,
        zoom_center_ps,
        tooltip_enabled,
    };

    let mut workspace_history = workspace_history_state.get_cloned();
    workspace_history.clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);

    Some(shared::AppConfig {
        app: shared::AppSection::default(),
        workspace: shared::WorkspaceSection {
            opened_files: session.opened_files.clone(),
            docked_bottom_dimensions: shared::DockedBottomDimensions {
                files_and_scopes_panel_width: files_width_bottom as f64,
                files_and_scopes_panel_height: files_height_bottom as f64,
                selected_variables_panel_name_column_width: Some(name_column_width_bottom as f64),
                selected_variables_panel_value_column_width: Some(value_column_width_bottom as f64),
            },
            docked_right_dimensions: shared::DockedRightDimensions {
                files_and_scopes_panel_width: files_width_right as f64,
                files_and_scopes_panel_height: files_height_right as f64,
                selected_variables_panel_name_column_width: Some(name_column_width_right as f64),
                selected_variables_panel_value_column_width: Some(value_column_width_right as f64),
            },
            dock_mode,
            expanded_scopes: session.expanded_scopes.clone(),
            load_files_expanded_directories: expanded_directories,
            selected_scope_id: session.selected_scope_id.clone(),
            load_files_scroll_position: scroll_position,
            variables_search_filter: session.variables_search_filter.clone(),
            selected_variables: selected_variables_snapshot,
            timeline: timeline_config,
            ..shared::WorkspaceSection::default()
        },
        ui: shared::UiSection {
            theme,
            toast_dismiss_ms: toast_dismiss_ms as u64,
        },
        global: shared::GlobalSection { workspace_history },
        plugins: plugins_state.get_cloned(),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionState {
    pub opened_files: Vec<CanonicalPathPayload>,
    pub expanded_scopes: Vec<String>,
    pub selected_scope_id: Option<String>,
    pub variables_search_filter: String,
    pub file_picker_scroll_position: i32,
    pub file_picker_expanded_directories: Vec<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            expanded_scopes: Vec::new(),
            selected_scope_id: None,
            variables_search_filter: String::new(),
            file_picker_scroll_position: 0,
            file_picker_expanded_directories: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimelineState {
    pub cursor_position: Option<TimePs>,
    pub visible_range: Option<TimeRange>,
    pub zoom_center: Option<TimePs>,
    pub tooltip_enabled: bool,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            cursor_position: None,
            visible_range: None,
            zoom_center: None,
            tooltip_enabled: true,
        }
    }
}

pub const DEFAULT_PANEL_WIDTH: f32 = 300.0;
pub const DEFAULT_TIMELINE_HEIGHT: f32 = 200.0;
pub const DEFAULT_NAME_COLUMN_WIDTH: f32 = 190.0;
pub const DEFAULT_VALUE_COLUMN_WIDTH: f32 = 220.0;

/// File picker domain with proper Actor+Relay architecture
#[derive(Clone)]
pub struct FilePickerDomain {
    pub expanded_directories_actor: Actor<indexmap::IndexSet<String>>,
    pub scroll_position_actor: Actor<i32>,
    pub directory_cache_actor:
        Actor<std::collections::HashMap<String, Vec<shared::FileSystemItem>>>,
    pub directory_errors_actor: Actor<std::collections::HashMap<String, String>>,
    pub directory_loading_actor: Actor<std::collections::HashSet<String>>,
    pub backend_sender_actor: Actor<()>,
    pub selected_files: crate::dataflow::ActorVec<String>,
    pub selected_files_vec_signal: zoon::Mutable<Vec<String>>,

    // Event-source relays for UI interactions
    pub directory_expanded_relay: Relay<String>,
    pub directory_collapsed_relay: Relay<String>,
    pub expanded_state_relay: Relay<Vec<String>>,
    pub scroll_position_changed_relay: Relay<i32>,
    pub config_save_requested_relay: Relay,

    // File selection relays for load files dialog
    pub file_selected_relay: Relay<String>,
    pub file_deselected_relay: Relay<String>,
    pub clear_selection_relay: Relay,

    // Internal relays for async operations (replaces zoon::Task)
    pub directory_load_requested_relay: Relay<String>,

    // Tree rendering timing coordination
    pub tree_rendering_relay: Relay,
}

impl FilePickerDomain {
    pub async fn new(
        initial_expanded: indexmap::IndexSet<String>,
        initial_scroll: i32,
        config_save_relay: Relay,
        connection: std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        connection_message_actor: crate::app::ConnectionMessageActor,
    ) -> Self {
        let (directory_expanded_relay, mut directory_expanded_stream) = relay::<String>();
        let (directory_collapsed_relay, mut directory_collapsed_stream) = relay::<String>();
        let (expanded_state_relay, _expanded_state_stream) = relay::<Vec<String>>();
        let (scroll_position_changed_relay, scroll_position_changed_stream) = relay::<i32>();
        let (directory_load_requested_relay, directory_load_requested_stream) = relay::<String>();
        let (tree_rendering_relay, _tree_rendering_stream) = relay();

        // File selection relays for load files dialog
        let (file_selected_relay, mut file_selected_stream) = relay::<String>();
        let (file_deselected_relay, mut file_deselected_stream) = relay::<String>();
        let (clear_selection_relay, mut clear_selection_stream) = relay();

        // ✅ ACTOR+RELAY: Subscribe to ConnectionMessageActor relays instead of internal ones
        let directory_contents_received_stream = connection_message_actor
            .directory_contents_relay
            .subscribe();
        let directory_error_received_stream =
            connection_message_actor.directory_error_relay.subscribe();
        let config_save_requested_relay = config_save_relay;

        let expanded_directories_actor = Actor::new(initial_expanded, {
            let save_relay = config_save_requested_relay.clone();
            let expanded_state_relay = expanded_state_relay.clone();
            async move |state| {
                loop {
                    futures::select! {
                        dir = directory_expanded_stream.next() => {
                            if let Some(dir) = dir {
                                let mut current = state.get_cloned();
                                if current.insert(dir.clone()) {
                                    crate::app::emit_trace(
                                        "workspace_picker_expanded_applied",
                                        format!("path={dir}"),
                                    );
                                    crate::app::emit_trace(
                                        "expanded_actor_state",
                                        format!(
                                            "after_insert paths={:?}",
                                            current.iter().cloned().collect::<Vec<_>>()
                                        ),
                                    );
                                    let snapshot =
                                        current.iter().cloned().collect::<Vec<_>>();
                                    crate::app::emit_trace(
                                        "expanded_state_snapshot",
                                        format!("action=insert paths={snapshot:?}"),
                                    );
                                    expanded_state_relay.send(snapshot);
                                    state.set_neq(current);
                                    save_relay.send(()); // Trigger config save
                                }
                            }
                        }
                        dir = directory_collapsed_stream.next() => {
                            if let Some(dir) = dir {
                                let mut current = state.get_cloned();
                                if current.shift_remove(&dir) {
                                    crate::app::emit_trace(
                                        "workspace_picker_collapsed_applied",
                                        format!("path={dir}"),
                                    );
                                    crate::app::emit_trace(
                                        "expanded_actor_state",
                                        format!(
                                            "after_remove paths={:?}",
                                            current.iter().cloned().collect::<Vec<_>>()
                                        ),
                                    );
                                    let snapshot =
                                        current.iter().cloned().collect::<Vec<_>>();
                                    crate::app::emit_trace(
                                        "expanded_state_snapshot",
                                        format!("action=remove paths={snapshot:?}"),
                                    );
                                    expanded_state_relay.send(snapshot);
                                    state.set_neq(current);
                                    save_relay.send(()); // Trigger config save
                                }
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        let scroll_position_actor = Actor::new(initial_scroll, {
            let save_relay = config_save_requested_relay.clone();
            async move |state| {
                let mut position_stream = scroll_position_changed_stream.fuse();

                loop {
                    futures::select! {
                        position = position_stream.next() => {
                            if let Some(position) = position {
                                crate::app::emit_trace(
                                    "workspace_picker_scroll_capture",
                                    format!("actor_set position={}", position),
                                );
                                // Update actor state immediately for UI reactivity
                                state.set_neq(position);

                                // ✅ Debounce pattern: nested select loop for config save
                                loop {
                                    futures::select! {
                                        // Check for newer scroll position updates
                                        newer_position = position_stream.next() => {
                                            if let Some(newer_pos) = newer_position {
                                                state.set_neq(newer_pos); // Update state immediately
                                            }
                                        }
                                        // Debounce timer - save after 500ms of no changes
                                        _ = zoon::Timer::sleep(500).fuse() => {
                                            save_relay.send(()); // Trigger config save
                                            break; // Back to outer loop
                                        }
                                    }
                                }
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        let directory_cache_actor = Actor::new(std::collections::HashMap::new(), {
            // ✅ FIX: Move stream into closure to prevent reference capture after Send bounds removal
            let mut directory_contents_stream = directory_contents_received_stream;
            async move |state| {
                loop {
                    use futures::StreamExt;
                    if let Some((path, items)) = directory_contents_stream.next().await {
                        // Check current cache state before update
                        let current_cache = state.get_cloned();

                        // Use set_neq with proper change detection - this MUST trigger signals
                        let mut cache = current_cache;
                        cache.insert(path.clone(), items);

                        state.set_neq(cache);

                        // Verify the update took effect
                        // Cache successfully updated
                    } else {
                        break;
                    }
                }
            }
        });

        let directory_errors_actor = Actor::new(std::collections::HashMap::new(), {
            // ✅ FIX: Move stream into closure to prevent reference capture after Send bounds removal
            let mut directory_error_stream = directory_error_received_stream;
            async move |state| loop {
                futures::select! {
                    error = directory_error_stream.next() => {
                        if let Some((path, error_message)) = error {
                            let mut errors = state.get_cloned();
                            errors.insert(path, error_message);
                            state.set_neq(errors);
                        }
                    }
                    complete => break,
                }
            }
        });

        // ✅ ACTOR+RELAY FIX: Add directory loading Actor to handle load requests
        // Since connection can't be used directly in Actor (Send trait issues), we use a different approach:
        // The Actor tracks loading requests and the UI layer polls for pending requests
        let directory_loading_actor = Actor::new(std::collections::HashSet::<String>::new(), {
            let mut load_stream = directory_load_requested_stream.fuse();
            async move |state| loop {
                futures::select! {
                    requested_path = load_stream.next() => {
                        if let Some(path) = requested_path {
                            let mut pending_requests = state.get_cloned();
                            pending_requests.insert(path);
                            state.set_neq(pending_requests);
                        }
                    }
                    complete => break,
                }
            }
        });

        // ✅ ACTOR+RELAY PATTERN: Backend request sender using nested Actor pattern
        let backend_sender_actor = {
            let connection_clone = connection.clone();
            let directory_cache_for_sender = directory_cache_actor.clone();
            let directory_loading_for_sender = directory_loading_actor.clone();
            // ✅ FIX: Create separate stream subscription for directory expansion events
            let mut directory_expanded_stream_for_sender = directory_expanded_relay.subscribe();

            Actor::new((), async move |_state| {
                let mut directory_loading_stream =
                    directory_loading_for_sender.signal().to_stream().fuse();
                // Use global platform readiness which flips on first DownMsg
                let mut ready_stream = crate::platform::server_ready_signal().to_stream().fuse();
                let mut is_ready = crate::platform::server_is_ready();

                loop {
                    futures::select! {
                        // Handle directory loading requests (existing logic)
                        pending_requests = directory_loading_stream.next() => {
                            if let Some(pending_requests) = pending_requests {

                                // Check cache to avoid sending requests for directories that already have data
                                let current_cache = directory_cache_for_sender.signal().to_stream().next().await.unwrap_or_default();

                                if is_ready {
                                    for request_path in pending_requests.iter() {
                                        if !current_cache.contains_key(request_path) {
                                            let path_for_request = request_path.clone();
                                            let _ = connection_clone
                                                .send_up_msg(shared::UpMsg::BrowseDirectory(path_for_request))
                                                .await;
                                        }
                                    }
                                }
                            } else {
                                // Stream ended
                                break;
                            }
                        }
                        // ✅ NEW: Handle directory expansion events (auto-browse expanded directories)
                        expanded_dir = directory_expanded_stream_for_sender.next() => {
                            if let Some(dir_path) = expanded_dir {

                                // Check cache to avoid duplicate requests
                                let current_cache = directory_cache_for_sender.signal().to_stream().next().await.unwrap_or_default();

                                if is_ready && !current_cache.contains_key(&dir_path) {
                                    let _ = connection_clone
                                        .send_up_msg(shared::UpMsg::BrowseDirectory(dir_path))
                                        .await;
                                }
                            }
                        }
                        // Flush pending directory requests when backend becomes ready
                        ready = ready_stream.next() => {
                            if let Some(ready_now) = ready {
                                is_ready = ready_now;
                            }
                            if is_ready {
                                // Re-check current pending set and cache, then send
                                let pending_requests = directory_loading_for_sender
                                    .signal()
                                    .to_stream()
                                    .next()
                                    .await
                                    .unwrap_or_default();
                                let current_cache = directory_cache_for_sender
                                    .signal()
                                    .to_stream()
                                    .next()
                                    .await
                                    .unwrap_or_default();
                                for request_path in pending_requests.iter() {
                                    if !current_cache.contains_key(request_path) {
                                        let _ = connection_clone
                                            .send_up_msg(shared::UpMsg::BrowseDirectory(request_path.clone()))
                                            .await;
                                    }
                                }
                            }
                        }
                        complete => {
                            break;
                        }
                    }
                }
            })
        };

        // Create dedicated vector signal to avoid SignalVec → Signal conversion antipattern
        let selected_files_vec_signal = zoon::Mutable::new(Vec::<String>::new());

        // Selected files ActorVec for file selection management
        let selected_files = {
            let selected_files_vec_signal_clone = selected_files_vec_signal.clone();
            crate::dataflow::ActorVec::new(vec![], async move |files_vec| {
                loop {
                    futures::select! {
                        file_path = file_selected_stream.next() => {
                            if let Some(file_path) = file_path {
                                let mut files = files_vec.lock_mut();
                                let already_selected = files.len() == 1
                                    && files.iter().any(|path| path == &file_path);
                                if already_selected {
                                    drop(files);
                                    continue;
                                }
                                files.clear();
                                files.push_cloned(file_path.clone());
                                drop(files);
                                zoon::println!("config.rs:selected push single={file_path}");
                                selected_files_vec_signal_clone
                                    .set_neq(vec![file_path]);
                            }
                        }
                        file_path = file_deselected_stream.next() => {
                            if let Some(file_path) = file_path {
                                files_vec.lock_mut().retain(|f| f != &file_path);
                                let current_files = files_vec.lock_ref().to_vec();
                                zoon::println!("config.rs:selected removed {file_path}, remaining={current_files:?}");
                                selected_files_vec_signal_clone.set_neq(current_files);
                            }
                        }
                        _ = clear_selection_stream.next() => {
                            zoon::println!("config.rs:selected_files clear_selection_relay");
                            files_vec.lock_mut().clear();
                            selected_files_vec_signal_clone.set_neq(Vec::new());
                        }
                        complete => break,
                    }
                }
            })
        };

        // The selected_files_vec_signal is kept updated from within the ActorVec loop above

        Self {
            expanded_directories_actor,
            scroll_position_actor,
            directory_cache_actor,
            directory_errors_actor,
            directory_loading_actor,
            backend_sender_actor,
            selected_files,
            selected_files_vec_signal,
            directory_expanded_relay,
            directory_collapsed_relay,
            expanded_state_relay,
            scroll_position_changed_relay,
            config_save_requested_relay,
            file_selected_relay,
            file_deselected_relay,
            clear_selection_relay,
            directory_load_requested_relay,
            tree_rendering_relay,
        }
    }

    pub fn expanded_directories_signal(&self) -> impl Signal<Item = indexmap::IndexSet<String>> {
        self.expanded_directories_actor.signal()
    }

    pub fn scroll_position_signal(&self) -> impl Signal<Item = i32> {
        self.scroll_position_actor.signal()
    }

    pub fn directory_cache_signal(
        &self,
    ) -> impl Signal<Item = std::collections::HashMap<String, Vec<shared::FileSystemItem>>> + use<>
    {
        self.directory_cache_actor.signal()
    }

    pub fn directory_errors_signal(
        &self,
    ) -> impl Signal<Item = std::collections::HashMap<String, String>> {
        self.directory_errors_actor.signal()
    }

    pub fn pending_directory_loads_signal(
        &self,
    ) -> impl Signal<Item = std::collections::HashSet<String>> {
        self.directory_loading_actor.signal()
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub theme_actor: Actor<SharedTheme>,
    pub dock_mode_actor: Actor<DockMode>,

    pub files_panel_width_right_actor: Actor<f32>,
    pub files_panel_height_right_actor: Actor<f32>,
    pub files_panel_width_bottom_actor: Actor<f32>,
    pub files_panel_height_bottom_actor: Actor<f32>,
    pub variables_panel_width_actor: Actor<f32>,
    pub timeline_panel_height_actor: Actor<f32>,
    pub variables_name_column_width_actor: Actor<f32>,
    pub variables_value_column_width_actor: Actor<f32>,

    pub session_state_actor: Actor<SessionState>,
    pub toast_dismiss_ms_actor: Actor<u32>,
    pub plugins_state: Mutable<shared::PluginsSection>,
    pub workspace_history_state: Mutable<shared::WorkspaceHistory>,
    pub workspace_history_update_relay: Relay<shared::WorkspaceHistory>,

    // File picker domain with proper Actor+Relay architecture
    pub file_picker_domain: FilePickerDomain,

    // Keep ConnectionMessageActor alive to prevent channel disconnection
    _connection_message_actor: crate::app::ConnectionMessageActor,

    pub loaded_selected_variables: Vec<shared::SelectedVariable>,

    pub theme_button_clicked_relay: Relay,
    pub dock_mode_button_clicked_relay: Relay,
    pub theme_changed_relay: Relay<SharedTheme>,
    pub dock_mode_changed_relay: Relay<DockMode>,
    pub files_width_right_changed_relay: Relay<f32>,
    pub files_height_right_changed_relay: Relay<f32>,
    pub files_width_bottom_changed_relay: Relay<f32>,
    pub files_height_bottom_changed_relay: Relay<f32>,
    pub variables_width_changed_relay: Relay<f32>,
    pub timeline_height_changed_relay: Relay<f32>,
    pub timeline_state_changed_relay: Relay<TimelineState>,
    pub timeline_state_restore_relay: Relay<TimelineState>,
    pub timeline_state_actor: Actor<TimelineState>,
    pub name_column_width_changed_relay: Relay<f32>,
    pub value_column_width_changed_relay: Relay<f32>,
    pub session_state_changed_relay: Relay<SessionState>,

    pub config_save_requested_relay: Relay,

    pub clipboard_copy_requested_relay: Relay<String>,

    pub error_display: crate::error_display::ErrorDisplay,
    // TreeView state - Mutables required for TreeView external state API
    pub files_expanded_scopes: zoon::Mutable<indexmap::IndexSet<String>>,
    pub files_selected_scope: zoon::MutableVec<String>,
    _expanded_sync_actor: Actor<()>,
    _scroll_sync_actor: Actor<()>,
    _clipboard_actor: Actor<()>,
    _save_trigger_actor: Actor<()>,
    _config_save_debouncer_actor: Actor<()>,
    _workspace_history_actor: Actor<()>,
    _config_loaded_actor: Actor<()>,
    pub config_loaded_flag: Mutable<bool>,
    _treeview_sync_actor: Actor<()>,
    _tracked_files_sync_actor: Actor<()>,
    _variables_filter_bridge_actor: Actor<()>,
    _selected_variables_snapshot_actor: Actor<Vec<shared::SelectedVariable>>,
}

impl AppConfig {
    async fn load_config_from_backend() -> Result<SharedAppConfig, String> {
        // Platform layer fallback - using defaults until proper backend config loading
        Ok(SharedAppConfig::default())
    }

    pub async fn new(
        connection: std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        connection_message_actor: crate::app::ConnectionMessageActor,
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
    ) -> Self {
        let config = Self::load_config_from_backend()
            .await
            .unwrap_or_else(|_error| SharedAppConfig::default());

        let plugins_state = Mutable::new(config.plugins.clone());
        let workspace_history_state = Mutable::new(config.global.workspace_history.clone());
        let (workspace_history_update_relay, workspace_history_update_stream) =
            relay::<shared::WorkspaceHistory>();
        let workspace_history_actor = {
            let mut update_stream = workspace_history_update_stream.fuse();
            let mut config_loaded_stream = connection_message_actor
                .config_loaded_relay
                .subscribe()
                .fuse();
            let mut ready = false;
            Actor::new((), async move |_state| {
                while let Some(mut pending) = update_stream.next().await {
                    let picker_state = pending.picker_tree_state.clone();
                    let expanded_paths = picker_state
                        .as_ref()
                        .map(|state| state.expanded_paths.clone())
                        .unwrap_or_default();
                    let scroll_top = picker_state
                        .as_ref()
                        .map(|state| state.scroll_top)
                        .unwrap_or(0.0);
                    crate::app::emit_trace(
                        "workspace_history_actor",
                        format!(
                            "stage=pending expanded_paths={expanded_paths:?} scroll_top={scroll_top}"
                        ),
                    );
                    loop {
                        select! {
                            _ = config_loaded_stream.next() => {
                                ready = true;
                            }
                            next = update_stream.next() => {
                                match next {
                                    Some(next_history) => {
                                        // Merge strategy: keep previous non-empty expanded paths
                                        // and non-zero scroll_top if the incoming update would
                                        // inadvertently clear them (e.g., teardown empties).
                                        let prev_picker = pending.picker_tree_state.clone();
                                        let mut merged = next_history.clone();
                                        let next_picker = merged.picker_tree_state.get_or_insert_with(shared::WorkspaceTreeState::default);
                                        if let Some(prev) = prev_picker {
                                            let prev_len = prev.expanded_paths.len();
                                            let next_len = next_picker.expanded_paths.len();
                                            let prev_scroll = prev.scroll_top;
                                            let next_scroll = next_picker.scroll_top;
                                            if next_len == 0 && prev_len > 0 {
                                                next_picker.expanded_paths = prev.expanded_paths;
                                            }
                                            if next_scroll <= 0.0 && prev_scroll > 0.0 {
                                                next_picker.scroll_top = prev_scroll;
                                            }
                                            crate::app::emit_trace(
                                                "workspace_history_actor",
                                                format!(
                                                    "stage=merge prev_len={prev_len} prev_scroll={prev_scroll} next_len={next_len} next_scroll={next_scroll} -> merged_len={} merged_scroll={}",
                                                    next_picker.expanded_paths.len(),
                                                    next_picker.scroll_top
                                                ),
                                            );
                                        }
                                        pending = merged;
                                        continue;
                                    }
                                    None => {
                                        let picker_state = pending.picker_tree_state.clone();
                                        let expanded_paths = picker_state
                                            .as_ref()
                                            .map(|state| state.expanded_paths.clone())
                                            .unwrap_or_default();
                                        let scroll_top = picker_state
                                            .as_ref()
                                            .map(|state| state.scroll_top)
                                            .unwrap_or(0.0);
                                        crate::app::emit_trace(
                                            "workspace_history_actor",
                                            format!(
                                                "stage=send_final expanded_paths={expanded_paths:?} scroll_top={scroll_top}"
                                            ),
                                        );
                                        if ready { let _ = CurrentPlatform::send_message(UpMsg::UpdateWorkspaceHistory(pending.clone())).await; }
                                        break;
                                    }
                                }
                            }
                            _ = zoon::Timer::sleep(250).fuse() => {
                                let picker_state = pending.picker_tree_state.clone();
                                let expanded_paths = picker_state
                                    .as_ref()
                                    .map(|state| state.expanded_paths.clone())
                                    .unwrap_or_default();
                                let scroll_top = picker_state
                                    .as_ref()
                                    .map(|state| state.scroll_top)
                                    .unwrap_or(0.0);
                                crate::app::emit_trace(
                                    "workspace_history_actor",
                                    format!(
                                        "stage=send_debounced expanded_paths={expanded_paths:?} scroll_top={scroll_top}"
                                    ),
                                );
                                if ready { let _ = CurrentPlatform::send_message(UpMsg::UpdateWorkspaceHistory(pending.clone())).await; }
                                break;
                            }
                        }
                    }
                }
            })
        };

        let (theme_button_clicked_relay, mut theme_button_clicked_stream) = relay();
        let (dock_mode_button_clicked_relay, mut dock_mode_button_clicked_stream) = relay();
        let (theme_changed_relay, mut theme_changed_stream) = relay();
        let (dock_mode_changed_relay, mut dock_mode_changed_stream) = relay();
        let (session_state_changed_relay, _session_state_changed_stream) = relay::<SessionState>();
        let session_state_changed_stream_for_config_saver = session_state_changed_relay.subscribe();
        let session_state_changed_stream_for_session_actor =
            session_state_changed_relay.subscribe();
        let (config_save_requested_relay, config_save_requested_stream) = relay();
        let (timeline_state_changed_relay, mut timeline_state_stream) = relay::<TimelineState>();
        let (timeline_state_restore_relay, _timeline_state_restore_stream) =
            relay::<TimelineState>();
        let timeline_state_save_relay = config_save_requested_relay.clone();

        let config_loaded_flag = Mutable::new(false);

        let timeline_state_actor = Actor::new(TimelineState::default(), async move |state| {
            let save_relay = timeline_state_save_relay;
            while let Some(new_state) = timeline_state_stream.next().await {
                state.set(new_state);
                save_relay.send(());
            }
        });

        // Track dock mode and per-mode column widths for Selected Variables panel
        let dock_mode_state = Mutable::new(config.workspace.dock_mode.clone());
        let name_column_width_bottom_state = Mutable::new(
            config
                .workspace
                .docked_bottom_dimensions
                .selected_variables_panel_name_column_width
                .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64) as f32,
        );
        let name_column_width_right_state = Mutable::new(
            config
                .workspace
                .docked_right_dimensions
                .selected_variables_panel_name_column_width
                .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64) as f32,
        );
        let value_column_width_bottom_state = Mutable::new(
            config
                .workspace
                .docked_bottom_dimensions
                .selected_variables_panel_value_column_width
                .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64) as f32,
        );
        let value_column_width_right_state = Mutable::new(
            config
                .workspace
                .docked_right_dimensions
                .selected_variables_panel_value_column_width
                .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64) as f32,
        );

        let (clipboard_copy_requested_relay, mut clipboard_copy_requested_stream) =
            relay::<String>();

        let (files_width_right_changed_relay, mut files_width_right_changed_stream) = relay();
        let (files_height_right_changed_relay, mut files_height_right_changed_stream) = relay();
        let (files_width_bottom_changed_relay, mut files_width_bottom_changed_stream) = relay();
        let (files_height_bottom_changed_relay, mut files_height_bottom_changed_stream) = relay();
        let (variables_width_changed_relay, mut variables_width_changed_stream) = relay();
        let (timeline_height_changed_relay, mut timeline_height_changed_stream) = relay();
        let (name_column_width_changed_relay, mut name_column_width_changed_stream) = relay();
        let (value_column_width_changed_relay, mut value_column_width_changed_stream) = relay();

        let theme_actor = Actor::new(config.ui.theme, async move |state| {
            let mut current_theme = config.ui.theme;

            let initial_novyui_theme = match current_theme {
                SharedTheme::Light => theme::Theme::Light,
                SharedTheme::Dark => theme::Theme::Dark,
            };
            theme::init_theme(Some(initial_novyui_theme), None);

            loop {
                select! {
                    button_click = theme_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            let new_theme = match current_theme {
                                SharedTheme::Light => SharedTheme::Dark,
                                SharedTheme::Dark => SharedTheme::Light,
                            };
                            current_theme = new_theme;
                            state.set(new_theme);

                            let novyui_theme = match new_theme {
                                SharedTheme::Light => theme::Theme::Light,
                                SharedTheme::Dark => theme::Theme::Dark,
                            };
                            theme::set_theme(novyui_theme);

                            // Config save handled by Task-based ConfigSaver
                        }
                    }
                    direct_change = theme_changed_stream.next() => {
                        if let Some(new_theme) = direct_change {
                            current_theme = new_theme;
                            state.set(new_theme);

                            let novyui_theme = match new_theme {
                                SharedTheme::Light => theme::Theme::Light,
                                SharedTheme::Dark => theme::Theme::Dark,
                            };
                            theme::set_theme(novyui_theme);
                        }
                    }
                    complete => break,
                }
            }
        });

        let dock_mode_state_for_actor = dock_mode_state.clone();
        let name_bottom_state_for_dock = name_column_width_bottom_state.clone();
        let name_right_state_for_dock = name_column_width_right_state.clone();
        let value_bottom_state_for_dock = value_column_width_bottom_state.clone();
        let value_right_state_for_dock = value_column_width_right_state.clone();
        let name_width_relay_for_dock = name_column_width_changed_relay.clone();
        let value_width_relay_for_dock = value_column_width_changed_relay.clone();

        let dock_mode_actor = Actor::new(config.workspace.dock_mode.clone(), async move |state| {
            let mut current_dock_mode = config.workspace.dock_mode.clone();

            loop {
                select! {
                    button_click = dock_mode_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            let new_mode = match current_dock_mode {
                                DockMode::Right => DockMode::Bottom,
                                DockMode::Bottom => DockMode::Right,
                            };
                            current_dock_mode = new_mode;
                            dock_mode_state_for_actor.set(new_mode.clone());
                            state.set(new_mode.clone());

                            let target_name = match new_mode {
                                DockMode::Right => name_right_state_for_dock.get_cloned(),
                                DockMode::Bottom => name_bottom_state_for_dock.get_cloned(),
                            };
                            name_width_relay_for_dock.send(target_name);

                            let target_value = match new_mode {
                                DockMode::Right => value_right_state_for_dock.get_cloned(),
                                DockMode::Bottom => value_bottom_state_for_dock.get_cloned(),
                            };
                            value_width_relay_for_dock.send(target_value);
                        }
                    }
                    direct_change = dock_mode_changed_stream.next() => {
                        if let Some(new_mode) = direct_change {
                            current_dock_mode = new_mode;
                            dock_mode_state_for_actor.set(new_mode.clone());
                            state.set(new_mode.clone());

                            let target_name = match new_mode {
                                DockMode::Right => name_right_state_for_dock.get_cloned(),
                                DockMode::Bottom => name_bottom_state_for_dock.get_cloned(),
                            };
                            name_width_relay_for_dock.send(target_name);

                            let target_value = match new_mode {
                                DockMode::Right => value_right_state_for_dock.get_cloned(),
                                DockMode::Bottom => value_bottom_state_for_dock.get_cloned(),
                            };
                            value_width_relay_for_dock.send(target_value);
                        }
                    }
                    complete => break,
                }
            }
        });

        let files_panel_width_right_actor = Actor::new(
            config
                .workspace
                .docked_right_dimensions
                .files_and_scopes_panel_width as f32,
            async move |state| loop {
                select! {
                    new_width = files_width_right_changed_stream.next() => {
                        if let Some(width) = new_width {
                            state.set_neq(width);
                        }
                    }
                }
            },
        );

        let files_panel_height_right_actor = Actor::new(
            config
                .workspace
                .docked_right_dimensions
                .files_and_scopes_panel_height as f32,
            async move |state| loop {
                select! {
                    new_height = files_height_right_changed_stream.next() => {
                        if let Some(height) = new_height {
                            state.set_neq(height);
                        }
                    }
                }
            },
        );

        let files_panel_width_bottom_actor = Actor::new(
            config
                .workspace
                .docked_bottom_dimensions
                .files_and_scopes_panel_width as f32,
            async move |state| loop {
                select! {
                    new_width = files_width_bottom_changed_stream.next() => {
                        if let Some(width) = new_width {
                            state.set_neq(width);
                        }
                    }
                }
            },
        );

        let files_panel_height_bottom_actor = Actor::new(
            config
                .workspace
                .docked_bottom_dimensions
                .files_and_scopes_panel_height as f32,
            async move |state| loop {
                select! {
                    new_height = files_height_bottom_changed_stream.next() => {
                        if let Some(height) = new_height {
                            state.set_neq(height);
                        }
                    }
                }
            },
        );

        let variables_panel_width_actor = Actor::new(DEFAULT_PANEL_WIDTH, async move |state| {
            loop {
                select! {
                    new_width = variables_width_changed_stream.next() => {
                        if let Some(width) = new_width {
                            state.set_neq(width);
                        }
                    }
                }
            }
        });

        let timeline_panel_height_actor = Actor::new(DEFAULT_TIMELINE_HEIGHT, async move |state| {
            loop {
                select! {
                    new_height = timeline_height_changed_stream.next() => {
                        if let Some(height) = new_height {
                            state.set_neq(height);
                        }
                    }
                }
            }
        });

        let initial_name_width = match config.workspace.dock_mode {
            DockMode::Right => name_column_width_right_state.get_cloned(),
            DockMode::Bottom => name_column_width_bottom_state.get_cloned(),
        };
        let dock_mode_state_for_name_actor = dock_mode_state.clone();
        let name_bottom_state_for_actor = name_column_width_bottom_state.clone();
        let name_right_state_for_actor = name_column_width_right_state.clone();
        let variables_name_column_width_actor =
            Actor::new(initial_name_width, async move |state| {
                loop {
                    select! {
                        new_width = name_column_width_changed_stream.next() => {
                            if let Some(width) = new_width {
                                state.set_neq(width);
                                match dock_mode_state_for_name_actor.get_cloned() {
                                    DockMode::Right => name_right_state_for_actor.set(width),
                                    DockMode::Bottom => name_bottom_state_for_actor.set(width),
                                }
                            }
                        }
                    }
                }
            });

        let initial_value_width = match config.workspace.dock_mode {
            DockMode::Right => value_column_width_right_state.get_cloned(),
            DockMode::Bottom => value_column_width_bottom_state.get_cloned(),
        };
        let dock_mode_state_for_value_actor = dock_mode_state.clone();
        let value_bottom_state_for_actor = value_column_width_bottom_state.clone();
        let value_right_state_for_actor = value_column_width_right_state.clone();
        let variables_value_column_width_actor =
            Actor::new(initial_value_width, async move |state| {
                loop {
                    select! {
                        new_width = value_column_width_changed_stream.next() => {
                            if let Some(width) = new_width {
                                state.set_neq(width);
                                match dock_mode_state_for_value_actor.get_cloned() {
                                    DockMode::Right => value_right_state_for_actor.set(width),
                                    DockMode::Bottom => value_bottom_state_for_actor.set(width),
                                }
                            }
                        }
                    }
                }
            });

        let session_state_actor = Actor::new(
            SessionState {
                opened_files: Vec::new(),    // Will be populated from TrackedFiles sync
                expanded_scopes: Vec::new(), // Will be populated from TreeView sync
                selected_scope_id: None,     // Will be populated from TreeView sync
                variables_search_filter: config.workspace.variables_search_filter,
                file_picker_scroll_position: config.workspace.load_files_scroll_position,
                file_picker_expanded_directories: config
                    .workspace
                    .load_files_expanded_directories
                    .clone(),
            },
            async move |state| {
                let mut session_stream = session_state_changed_stream_for_session_actor;

                loop {
                    select! {
                        session_change = session_stream.next() => {
                            if let Some(new_session) = session_change {
                                state.set_neq(new_session);
                            }
                        }
                        complete => break,
                    }
                }
            },
        );

        let toast_dismiss_ms_actor =
            Actor::new(config.ui.toast_dismiss_ms as u32, async move |_state| {
                loop {
                    Task::next_macro_tick().await;
                }
            });

        // FIX: Connect ConfigSaver to button click relays for immediate save trigger

        // Clone the relay for struct return since it will be moved into Tasks
        let config_save_requested_relay_for_struct = config_save_requested_relay.clone();

        // Use Actor to handle config save triggering (eliminates zoon::Task)
        let save_trigger_actor = Actor::new((), {
            let theme_button_stream = theme_button_clicked_relay.subscribe();
            let dock_button_stream = dock_mode_button_clicked_relay.subscribe();
            let config_save_relay = config_save_requested_relay.clone();
            async move |_state| {
                let mut theme_stream = theme_button_stream;
                let mut dock_stream = dock_button_stream;

                loop {
                    select! {
                        result = theme_stream.next() => {
                            match result {
                                Some(_) => {
                                    config_save_relay.send(());
                                }
                                None => break, // Stream ended
                            }
                        }
                        result = dock_stream.next() => {
                            match result {
                                Some(_) => {
                                    config_save_relay.send(());
                                }
                                None => break, // Stream ended
                            }
                        }
                    }
                }
            }
        });

        // Create FilePickerDomain with proper Actor+Relay architecture
        let initial_expanded_set = {
            let mut expanded_set = indexmap::IndexSet::new();
            for dir in &config.workspace.load_files_expanded_directories {
                expanded_set.insert(dir.clone());
            }
            expanded_set
        };

        let file_picker_domain = FilePickerDomain::new(
            initial_expanded_set.clone(),
            config.workspace.load_files_scroll_position,
            config_save_requested_relay.clone(),
            connection,
            connection_message_actor.clone(),
        )
        .await;

        // Initialize TreeView Mutables for Files & Scopes panel
        let files_expanded_scopes = zoon::Mutable::new(indexmap::IndexSet::from_iter(
            config.workspace.expanded_scopes.iter().cloned(),
        ));
        let files_selected_scope = zoon::MutableVec::new_with_values(
            config
                .workspace
                .selected_scope_id
                .clone()
                .into_iter()
                .collect(),
        );

        // Sync TreeView state changes to session state
        let expanded_scopes_for_sync = files_expanded_scopes.clone();
        let selected_scope_for_sync = files_selected_scope.clone();
        let session_relay_for_treeview = session_state_changed_relay.clone();
        let session_actor_for_treeview = session_state_actor.clone();
        let treeview_sync_actor = Actor::new((), async move |_| {
            let mut expanded_stream = expanded_scopes_for_sync.signal_cloned().to_stream().fuse();
            let mut selected_stream = selected_scope_for_sync
                .signal_vec_cloned()
                .to_signal_cloned()
                .to_stream()
                .fuse();

            loop {
                select! {
                    expanded = expanded_stream.next() => {
                        if let Some(expanded_set) = expanded {
                            let current_session = session_actor_for_treeview.signal().to_stream().next().await.unwrap_or_default();
                            let updated_session = SessionState {
                                expanded_scopes: expanded_set.iter().cloned().collect(),
                                ..current_session
                            };
                            session_relay_for_treeview.send(updated_session);
                        }
                    }
                    selected = selected_stream.next() => {
                        if let Some(selected_vec) = selected {
                            // Choose the first scope_* entry if present
                            let scope_sel = selected_vec.iter().find(|id| id.starts_with("scope_")).cloned();
                            let current_session = session_actor_for_treeview.signal().to_stream().next().await.unwrap_or_default();
                            let updated_session = SessionState {
                                selected_scope_id: scope_sel,
                                ..current_session
                            };
                            session_relay_for_treeview.send(updated_session);
                        }
                    }
                }
            }
        });

        // Create sync actor for TrackedFiles to update opened_files
        let tracked_files_sync_actor = {
            let session_state_actor_for_files = session_state_actor.clone();
            let files_signal = tracked_files.files_vec_signal.clone();
            let session_relay_for_files = session_state_changed_relay.clone();
            let files_expanded_for_sync = files_expanded_scopes.clone();
            let files_selected_for_sync = files_selected_scope.clone();

            Actor::new((), async move |_state| {
                // Force initial sync with current value
                let initial_files = files_signal.get_cloned();
                if !initial_files.is_empty() {
                    let file_paths: Vec<CanonicalPathPayload> = initial_files
                        .iter()
                        .map(|tracked_file| {
                            CanonicalPathPayload::new(tracked_file.canonical_path.clone())
                        })
                        .collect();

                    let mut current_session = session_state_actor_for_files
                        .signal()
                        .to_stream()
                        .next()
                        .await
                        .unwrap_or_default();
                    current_session.opened_files = file_paths;

                    // Preserve expanded scopes and selected scope from TreeView Mutables
                    let current_expanded = files_expanded_for_sync.get_cloned();
                    current_session.expanded_scopes = current_expanded.into_iter().collect();

                    let current_selected = files_selected_for_sync.lock_ref();
                    current_session.selected_scope_id = current_selected.first().cloned();

                    session_relay_for_files.send(current_session);
                }

                let mut stream = files_signal.signal_cloned().to_stream();
                while let Some(files) = stream.next().await {
                    // Extract file paths from TrackedFile structs
                    let file_paths: Vec<CanonicalPathPayload> = files
                        .iter()
                        .map(|tracked_file| {
                            CanonicalPathPayload::new(tracked_file.canonical_path.clone())
                        })
                        .collect();

                    // Update session state - preserve other fields
                    let mut current_session = session_state_actor_for_files
                        .signal()
                        .to_stream()
                        .next()
                        .await
                        .unwrap_or_default();
                    current_session.opened_files = file_paths.clone();

                    // CRITICAL: Read expanded_scopes from TreeView Mutables, not from stale session
                    let current_expanded = files_expanded_for_sync.get_cloned();
                    current_session.expanded_scopes = current_expanded.into_iter().collect();

                    let current_selected = files_selected_for_sync.lock_ref();
                    current_session.selected_scope_id = current_selected.first().cloned();

                    // Trigger save
                    session_relay_for_files.send(current_session);
                }
            })
        };

        // Bridge variables search filter between SelectedVariables domain and SessionState
        let variables_filter_bridge_actor = {
            let session_state_actor_for_bridge = session_state_actor.clone();
            let session_state_relay_for_bridge = session_state_changed_relay.clone();
            let search_filter_relay = selected_variables.search_filter_changed_relay.clone();

            Actor::new((), async move |_state| {
                let mut session_state_stream =
                    session_state_actor_for_bridge.signal().to_stream().fuse();
                let mut filter_events_stream = search_filter_relay.subscribe().fuse();

                let mut current_session = session_state_stream.next().await.unwrap_or_default();
                let mut current_filter = current_session.variables_search_filter.clone();

                loop {
                    select! {
                        session = session_state_stream.next() => {
                            if let Some(session_state) = session {
                                if current_filter != session_state.variables_search_filter {
                                    current_filter = session_state.variables_search_filter.clone();
                                    search_filter_relay.send(current_filter.clone());
                                }
                                current_session = session_state;
                            } else {
                                break;
                            }
                        }
                        filter = filter_events_stream.next() => {
                            if let Some(filter_text) = filter {
                                if current_filter == filter_text {
                                    continue;
                                }
                                current_filter = filter_text.clone();
                                current_session.variables_search_filter = filter_text;
                                session_state_relay_for_bridge.send(current_session.clone());
                            } else {
                                break;
                            }
                        }
                    }
                }
            })
        };

        // Track SelectedVariables changes to trigger config saves with latest snapshot
        let selected_variables_snapshot_actor = {
            let selected_variables_for_snapshot = selected_variables.clone();
            let save_relay_for_snapshot = config_save_requested_relay.clone();

            Actor::new(Vec::<shared::SelectedVariable>::new(), async move |state| {
                let mut variables_stream = selected_variables_for_snapshot
                    .variables_signal()
                    .to_stream()
                    .fuse();

                while let Some(vars) = variables_stream.next().await {
                    let should_update = {
                        let current = state.lock_ref();
                        *current != vars
                    };

                    if should_update {
                        state.set_neq(vars.clone());
                        save_relay_for_snapshot.send(());
                    }
                }
            })
        };

        // File picker changes now trigger config save through FilePickerDomain
        // Use nested Actor pattern for debouncing instead of Task::start
        let config_save_debouncer_actor = {
            let theme_actor_clone = theme_actor.clone();
            let dock_mode_actor_clone = dock_mode_actor.clone();
            let session_actor_clone = session_state_actor.clone();
            let toast_actor_clone = toast_dismiss_ms_actor.clone();
            let timeline_state_actor_clone = timeline_state_actor.clone();
            let file_picker_domain_clone = file_picker_domain.clone();
            let selected_variables_snapshot_actor_clone = selected_variables_snapshot_actor.clone();
            let config_loaded_flag_for_saver = config_loaded_flag.clone();
            let files_width_right_actor_clone = files_panel_width_right_actor.clone();
            let files_height_right_actor_clone = files_panel_height_right_actor.clone();
            let files_width_bottom_actor_clone = files_panel_width_bottom_actor.clone();
            let files_height_bottom_actor_clone = files_panel_height_bottom_actor.clone();
            let name_column_width_bottom_state_clone = name_column_width_bottom_state.clone();
            let name_column_width_right_state_clone = name_column_width_right_state.clone();
            let value_column_width_bottom_state_clone = value_column_width_bottom_state.clone();
            let value_column_width_right_state_clone = value_column_width_right_state.clone();
            let plugins_state_clone = plugins_state.clone();
            let workspace_history_state_clone_for_save = workspace_history_state.clone();

            Actor::new((), async move |_state| {
                let mut config_save_requested_stream = config_save_requested_stream.fuse();
                let mut session_state_stream = session_state_changed_stream_for_config_saver.fuse();
                let mut config_loaded_stream =
                    config_loaded_flag_for_saver.signal().to_stream().fuse();
                let mut config_ready = false;

                loop {
                    if !config_ready {
                        match config_loaded_stream.next().await {
                            Some(is_loaded) => {
                                config_ready = is_loaded;
                                continue;
                            }
                            None => break,
                        }
                    }
                    select! {
                        result = config_save_requested_stream.next() => {
                            if let Some(()) = result {
                                // Debounce loop - wait for quiet period, cancelling if new request arrives
                                loop {
                                    select! {
                                        // New save request cancels timer
                                        result = config_save_requested_stream.next() => {
                                            if let Some(()) = result {
                                                continue; // Restart timer
                                            }
                                        }
                                        // Timer completes - do the save
                                        _ = zoon::Timer::sleep(300).fuse() => {
                                            if config_ready && crate::platform::server_is_ready() {
                                                if let Some(shared_config) = compose_shared_app_config(
                                                &theme_actor_clone,
                                                &dock_mode_actor_clone,
                                                &session_actor_clone,
                                                &toast_actor_clone,
                                                &file_picker_domain_clone,
                                                &selected_variables_snapshot_actor_clone,
                                                &files_width_right_actor_clone,
                                                &files_height_right_actor_clone,
                                                &files_width_bottom_actor_clone,
                                                &files_height_bottom_actor_clone,
                                                &name_column_width_bottom_state_clone,
                                                &name_column_width_right_state_clone,
                                                &value_column_width_bottom_state_clone,
                                                &value_column_width_right_state_clone,
                                                &timeline_state_actor_clone,
                                                &workspace_history_state_clone_for_save,
                                                &plugins_state_clone,
                                            )
                                            .await
                                            {
                                                let _ = CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config)).await;
                                            }
                                            }
                                            break; // Back to outer loop
                                    }
                                }
                            }
                        }
                    }

                        // Also trigger save when session state changes (file loads, etc.)
                        session_change = session_state_stream.next() => {
                            if let Some(_new_session) = session_change {
                                // Debounce loop for session state changes
                                loop {
                                    select! {
                                        // New session change cancels timer
                                        session_change = session_state_stream.next() => {
                                            if let Some(_) = session_change {
                                                continue; // Restart timer
                                            }
                                        }
                                        // Timer completes - do the save
                                        _ = zoon::Timer::sleep(300).fuse() => {
                                            if config_ready && crate::platform::server_is_ready() {
                                                if let Some(shared_config) = compose_shared_app_config(
                                                &theme_actor_clone,
                                                &dock_mode_actor_clone,
                                                &session_actor_clone,
                                                &toast_actor_clone,
                                                &file_picker_domain_clone,
                                                &selected_variables_snapshot_actor_clone,
                                                &files_width_right_actor_clone,
                                                &files_height_right_actor_clone,
                                                &files_width_bottom_actor_clone,
                                                &files_height_bottom_actor_clone,
                                                &name_column_width_bottom_state_clone,
                                                &name_column_width_right_state_clone,
                                                &value_column_width_bottom_state_clone,
                                                &value_column_width_right_state_clone,
                                                &timeline_state_actor_clone,
                                                &workspace_history_state_clone_for_save,
                                                &plugins_state_clone,
                                            )
                                            .await
                                            {
                                                let _ = CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config)).await;
                                            }
                                            }
                                            break; // Back to outer loop
                                        }
                                    }
                                }
                            }
                        }
                        loaded = config_loaded_stream.next() => {
                            match loaded {
                                Some(is_loaded) => {
                                    config_ready = is_loaded;
                                    if !config_ready {
                                        continue;
                                    }
                                }
                                None => break,
                            }
                        }
                    }
                }
            })
        };

        // Complex bridge pattern removed - using direct FilePickerDomain events

        let scroll_sync_actor = Actor::new((), {
            let scroll_position_sync = file_picker_domain.scroll_position_actor.clone();
            let session_scroll_sync = session_state_actor.clone();
            let session_scroll_changed_relay = session_state_changed_relay.clone();

            async move |_state| {
                let mut scroll_stream = scroll_position_sync.signal().to_stream();

                while let Some(scroll_position) = scroll_stream.next().await {
                    // Create updated session with new scroll position
                    let current_session = session_scroll_sync
                        .signal()
                        .to_stream()
                        .next()
                        .await
                        .unwrap_or_default();
                    let updated_session = SessionState {
                        file_picker_scroll_position: scroll_position,
                        ..current_session
                    };
                    session_scroll_changed_relay.send(updated_session);
                }
            }
        });

        let clipboard_actor = Actor::new((), async move |_state| {
            while let Some(text) = clipboard_copy_requested_stream.next().await {
                // Use spawn_local within Actor to handle WASM clipboard operations
                spawn_local(async move {
                    if let Some(window) = web_sys::window() {
                        let navigator = window.navigator();

                        #[cfg(web_sys_unstable_apis)]
                        {
                            let clipboard = navigator.clipboard();
                            let _ = JsFuture::from(clipboard.write_text(&text)).await;
                        }

                        #[cfg(not(web_sys_unstable_apis))]
                        {}
                    }
                });
            }
        });

        let error_display = crate::error_display::ErrorDisplay::new().await;

        // ✅ ACTOR+RELAY: Subscribe to config_loaded_relay from ConnectionMessageActor
        let config_loaded_actor = {
            let config_loaded_stream = connection_message_actor.config_loaded_relay.subscribe();
            let theme_relay = theme_changed_relay.clone();
            let dock_relay = dock_mode_changed_relay.clone();
            let file_picker_domain_clone = file_picker_domain.clone();
            let tracked_files_for_config = tracked_files.clone();
            let files_expanded_scopes_for_config = files_expanded_scopes.clone();
            let files_selected_scope_for_config = files_selected_scope.clone();
            let selected_variables_for_config = selected_variables.clone();
            let session_state_relay_for_config = session_state_changed_relay.clone();
            let config_loaded_flag_for_actor = config_loaded_flag.clone();
            let dock_mode_state_for_config = dock_mode_state.clone();
            let name_column_width_bottom_state_clone = name_column_width_bottom_state.clone();
            let name_column_width_right_state_clone = name_column_width_right_state.clone();
            let value_column_width_bottom_state_clone = value_column_width_bottom_state.clone();
            let value_column_width_right_state_clone = value_column_width_right_state.clone();
            let files_width_right_relay = files_width_right_changed_relay.clone();
            let files_height_right_relay = files_height_right_changed_relay.clone();
            let files_width_bottom_relay = files_width_bottom_changed_relay.clone();
            let files_height_bottom_relay = files_height_bottom_changed_relay.clone();
            let name_column_width_relay = name_column_width_changed_relay.clone();
            let value_column_width_relay = value_column_width_changed_relay.clone();
            let timeline_state_relay = timeline_state_changed_relay.clone();
            let timeline_state_restore_relay = timeline_state_restore_relay.clone();
            let plugins_state_clone = plugins_state.clone();
            let workspace_history_state_clone = workspace_history_state.clone();

            Actor::new((), async move |_state| {
                let mut config_stream = config_loaded_stream;

                while let Some(loaded_config) = config_stream.next().await {
                    plugins_state_clone.set(loaded_config.plugins.clone());
                    workspace_history_state_clone
                        .set_neq(loaded_config.global.workspace_history.clone());
                    dock_mode_state_for_config.set(loaded_config.workspace.dock_mode.clone());

                    let loaded_files_width_right = loaded_config
                        .workspace
                        .docked_right_dimensions
                        .files_and_scopes_panel_width
                        as f32;
                    let loaded_files_height_right = loaded_config
                        .workspace
                        .docked_right_dimensions
                        .files_and_scopes_panel_height
                        as f32;
                    let loaded_files_width_bottom = loaded_config
                        .workspace
                        .docked_bottom_dimensions
                        .files_and_scopes_panel_width
                        as f32;
                    let loaded_files_height_bottom = loaded_config
                        .workspace
                        .docked_bottom_dimensions
                        .files_and_scopes_panel_height
                        as f32;

                    files_width_right_relay.send(loaded_files_width_right);
                    files_height_right_relay.send(loaded_files_height_right);
                    files_width_bottom_relay.send(loaded_files_width_bottom);
                    files_height_bottom_relay.send(loaded_files_height_bottom);

                    let loaded_name_bottom = loaded_config
                        .workspace
                        .docked_bottom_dimensions
                        .selected_variables_panel_name_column_width
                        .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64)
                        as f32;
                    let loaded_name_right = loaded_config
                        .workspace
                        .docked_right_dimensions
                        .selected_variables_panel_name_column_width
                        .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64)
                        as f32;
                    name_column_width_bottom_state_clone.set(loaded_name_bottom);
                    name_column_width_right_state_clone.set(loaded_name_right);

                    let loaded_value_bottom = loaded_config
                        .workspace
                        .docked_bottom_dimensions
                        .selected_variables_panel_value_column_width
                        .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64)
                        as f32;
                    let loaded_value_right = loaded_config
                        .workspace
                        .docked_right_dimensions
                        .selected_variables_panel_value_column_width
                        .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64)
                        as f32;
                    value_column_width_bottom_state_clone.set(loaded_value_bottom);
                    value_column_width_right_state_clone.set(loaded_value_right);

                    let current_name_width = match loaded_config.workspace.dock_mode {
                        DockMode::Right => loaded_name_right,
                        DockMode::Bottom => loaded_name_bottom,
                    };
                    name_column_width_relay.send(current_name_width);

                    let current_value_width = match loaded_config.workspace.dock_mode {
                        DockMode::Right => loaded_value_right,
                        DockMode::Bottom => loaded_value_bottom,
                    };
                    value_column_width_relay.send(current_value_width);

                    let timeline_cfg = &loaded_config.workspace.timeline;

                    let visible_start_ps = timeline_cfg.visible_range_start_ps;
                    let mut visible_end_ps = timeline_cfg.visible_range_end_ps;
                    if visible_end_ps <= visible_start_ps {
                        visible_end_ps = visible_start_ps + 1;
                    }

                    let mut cursor_position_ps = timeline_cfg.cursor_position_ps;
                    if cursor_position_ps < visible_start_ps || cursor_position_ps > visible_end_ps
                    {
                        cursor_position_ps = visible_start_ps;
                    }

                    let mut zoom_center_ps = timeline_cfg.zoom_center_ps;
                    if zoom_center_ps < visible_start_ps || zoom_center_ps > visible_end_ps {
                        zoom_center_ps = visible_start_ps;
                    }

                    let visible_range = TimeRange {
                        start: TimePs::from_picoseconds(visible_start_ps),
                        end: TimePs::from_picoseconds(visible_end_ps),
                    };

                    let timeline_state = TimelineState {
                        cursor_position: Some(TimePs::from_picoseconds(cursor_position_ps)),
                        visible_range: Some(visible_range),
                        zoom_center: Some(TimePs::from_picoseconds(zoom_center_ps)),
                        tooltip_enabled: timeline_cfg.tooltip_enabled,
                    };

                    timeline_state_relay.send(timeline_state.clone());
                    timeline_state_restore_relay.send(timeline_state);

                    // Update theme using proper relay
                    theme_relay.send(loaded_config.ui.theme);

                    // Update dock mode using proper relay
                    dock_relay.send(loaded_config.workspace.dock_mode);

                    // Update expanded directories using FilePickerDomain
                    for dir in &loaded_config.workspace.load_files_expanded_directories {
                        file_picker_domain_clone
                            .directory_expanded_relay
                            .send(dir.clone());
                    }

                    // Update scroll position using FilePickerDomain relay
                    file_picker_domain_clone
                        .scroll_position_changed_relay
                        .send(loaded_config.workspace.load_files_scroll_position);

                    // Restore tracked files from config or clear when empty
                    if loaded_config.workspace.opened_files.is_empty() {
                        tracked_files_for_config.all_files_cleared_relay.send(());
                    } else {
                        tracked_files_for_config
                            .config_files_loaded_relay
                            .send(loaded_config.workspace.opened_files.clone());
                    }

                    // Synchronize session state with loaded config data
                    session_state_relay_for_config.send(SessionState {
                        opened_files: loaded_config.workspace.opened_files.clone(),
                        expanded_scopes: loaded_config.workspace.expanded_scopes.clone(),
                        selected_scope_id: loaded_config.workspace.selected_scope_id.clone(),
                        variables_search_filter: loaded_config
                            .workspace
                            .variables_search_filter
                            .clone(),
                        file_picker_scroll_position: loaded_config
                            .workspace
                            .load_files_scroll_position,
                        file_picker_expanded_directories: loaded_config
                            .workspace
                            .load_files_expanded_directories
                            .clone(),
                    });

                    // Restore selected variables into domain state
                    selected_variables_for_config
                        .variables_restored_relay
                        .send(loaded_config.workspace.selected_variables.clone());

                    // IMPORTANT: Restore expanded scopes AFTER files are sent
                    // Add a small delay to ensure files are processed first
                    let expanded_scopes_to_restore =
                        loaded_config.workspace.expanded_scopes.clone();
                    let selected_scope_to_restore =
                        loaded_config.workspace.selected_scope_id.clone();
                    let files_expanded_for_delay = files_expanded_scopes_for_config.clone();
                    let files_selected_for_delay = files_selected_scope_for_config.clone();

                    zoon::Task::start(async move {
                        // Wait for files to be processed
                        zoon::Timer::sleep(100).await;

                        // Now restore expanded scopes in TreeView
                        let expanded_set: indexmap::IndexSet<String> =
                            expanded_scopes_to_restore.iter().cloned().collect();

                        files_expanded_for_delay.set(expanded_set);

                        // Restore selected scope in TreeView
                        if let Some(scope_id) = selected_scope_to_restore {
                            files_selected_for_delay.lock_mut().clear();
                            files_selected_for_delay.lock_mut().push_cloned(scope_id);
                        }
                    });

                    config_loaded_flag_for_actor.set(true);
                }
            })
        };

        Self {
            theme_actor,
            dock_mode_actor,
            files_panel_width_right_actor,
            files_panel_height_right_actor,
            files_panel_width_bottom_actor,
            files_panel_height_bottom_actor,
            variables_panel_width_actor,
            timeline_panel_height_actor,
            variables_name_column_width_actor,
            variables_value_column_width_actor,
            session_state_actor,
            toast_dismiss_ms_actor,
            plugins_state,
            workspace_history_state,
            workspace_history_update_relay,

            file_picker_domain,

            loaded_selected_variables: config.workspace.selected_variables.clone(),

            theme_button_clicked_relay,
            dock_mode_button_clicked_relay,
            theme_changed_relay,
            dock_mode_changed_relay,
            files_width_right_changed_relay,
            files_height_right_changed_relay,
            files_width_bottom_changed_relay,
            files_height_bottom_changed_relay,
            variables_width_changed_relay,
            timeline_height_changed_relay,
            timeline_state_changed_relay,
            timeline_state_restore_relay,
            timeline_state_actor,
            name_column_width_changed_relay,
            value_column_width_changed_relay,
            session_state_changed_relay,
            config_save_requested_relay: config_save_requested_relay_for_struct,

            clipboard_copy_requested_relay,

            error_display,
            files_expanded_scopes,
            files_selected_scope,
            _expanded_sync_actor: Actor::new((), async move |_| {
                loop {
                    Task::next_macro_tick().await;
                }
            }),
            _scroll_sync_actor: scroll_sync_actor,
            _clipboard_actor: clipboard_actor,
            _save_trigger_actor: save_trigger_actor,
            _config_save_debouncer_actor: config_save_debouncer_actor,
            _workspace_history_actor: workspace_history_actor,
            _config_loaded_actor: config_loaded_actor,
            config_loaded_flag,
            _connection_message_actor: connection_message_actor,
            _treeview_sync_actor: treeview_sync_actor,
            _tracked_files_sync_actor: tracked_files_sync_actor,
            _variables_filter_bridge_actor: variables_filter_bridge_actor,
            _selected_variables_snapshot_actor: selected_variables_snapshot_actor,
        }
    }

    /// Mark that a workspace switch is in progress, pausing config saves
    /// until the next ConfigLoaded arrives.
    pub fn mark_workspace_switching(&self) {
        self.config_loaded_flag.set(false);
    }

    /// Update config from loaded backend data
    pub fn update_from_loaded_config(&self, loaded_config: shared::AppConfig) {
        self.plugins_state.set(loaded_config.plugins.clone());
        // Update theme using proper relay (not direct state access)
        self.theme_changed_relay.send(loaded_config.ui.theme);

        // Update dock mode using proper relay (not direct state access)
        self.dock_mode_changed_relay
            .send(loaded_config.workspace.dock_mode);

        self.workspace_history_state
            .set_neq(loaded_config.global.workspace_history.clone());

        // Update expanded directories using FilePickerDomain
        let mut expanded_set = indexmap::IndexSet::new();
        for dir in &loaded_config.workspace.load_files_expanded_directories {
            expanded_set.insert(dir.clone());
        }
        // Use FilePickerDomain relays to update expanded directories
        for dir in &expanded_set {
            self.file_picker_domain
                .directory_expanded_relay
                .send(dir.clone());
        }

        // Update scroll position using FilePickerDomain relay
        self.file_picker_domain
            .scroll_position_changed_relay
            .send(loaded_config.workspace.load_files_scroll_position);
    }

    pub fn record_workspace_selection(&self, path: &str) {
        if path.is_empty() {
            return;
        }
        let mut history = self.workspace_history_state.get_cloned();
        history.touch_path(path, shared::WORKSPACE_HISTORY_MAX_RECENTS);
        self.workspace_history_state.set_neq(history.clone());
        // Let the workspace_history_actor persist it after ConfigLoaded (simple and reliable).
        self.workspace_history_update_relay.send(history);
    }

    pub fn update_workspace_tree_state(&self, path: &str, expanded_paths: Vec<String>) {
        if path.is_empty() {
            return;
        }
        let mut history = self.workspace_history_state.get_cloned();
        let entry = history
            .tree_state
            .entry(path.to_string())
            .or_insert_with(shared::WorkspaceTreeState::default);
        entry.expanded_paths = expanded_paths;
        history.clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
        self.workspace_history_state.set_neq(history.clone());
        self.workspace_history_update_relay.send(history);
    }

    pub fn update_workspace_scroll(&self, path: &str, scroll_top: f64) {
        if path.is_empty() {
            return;
        }
        let mut history = self.workspace_history_state.get_cloned();
        let entry = history
            .tree_state
            .entry(path.to_string())
            .or_insert_with(shared::WorkspaceTreeState::default);
        entry.scroll_top = scroll_top;
        history.clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
        self.workspace_history_state.set_neq(history.clone());
        self.workspace_history_update_relay.send(history);
    }

    pub fn update_workspace_picker_tree_state(&self, expanded_paths: Vec<String>) {
        let mut history = self.workspace_history_state.get_cloned();
        let prev_scroll = history
            .picker_tree_state
            .as_ref()
            .map(|s| s.scroll_top)
            .unwrap_or(0.0);
        let entry = history.picker_state_mut();
        let state_ptr = {
            let guard = self.workspace_history_state.lock_ref();
            (&*guard as *const shared::WorkspaceHistory) as usize
        };
        crate::app::emit_trace(
            "workspace_history_mutation",
            format!(
                "origin=picker_tree_state ptr={state_ptr:#x} expanded_paths={expanded_paths:?}"
            ),
        );
        entry.expanded_paths = expanded_paths;
        // Preserve previous scroll_top if an entry already existed, so expand/collapse
        // updates don’t reset scroll while the dialog is open.
        if entry.scroll_top == 0.0 && prev_scroll > 0.0 {
            entry.scroll_top = prev_scroll;
        }
        history.clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
        self.workspace_history_state.set_neq(history.clone());
        self.workspace_history_update_relay.send(history);
    }

    pub fn update_workspace_picker_scroll(&self, scroll_top: f64) {
        let mut history = self.workspace_history_state.get_cloned();
        // Always ensure picker_tree_state exists and persist scroll.
        // Later expanded-path updates will preserve this value, and backend merge
        // ignores empty expanded_paths writes, so this is safe and robust.
        let entry = history.picker_state_mut();
        let state_ptr = {
            let guard = self.workspace_history_state.lock_ref();
            (&*guard as *const shared::WorkspaceHistory) as usize
        };
        crate::app::emit_trace(
            "workspace_history_mutation",
            format!("origin=picker_scroll ptr={state_ptr:#x} scroll_top={scroll_top}"),
        );
        entry.scroll_top = scroll_top;
        history.clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
        self.workspace_history_state.set_neq(history.clone());
        self.workspace_history_update_relay.send(history);
    }

    /// Remove a single path from Recent workspaces.
    /// If it was the last_selected, promote the next recent if available.
    pub fn remove_recent_workspace(&self, path: &str) {
        if path.is_empty() {
            return;
        }
        let mut history = self.workspace_history_state.get_cloned();
        history.recent_paths.retain(|p| p != path);
        history.tree_state.remove(path);
        if history
            .last_selected
            .as_ref()
            .map(|p| p == path)
            .unwrap_or(false)
        {
            history.last_selected = history.recent_paths.first().cloned();
        }
        history.clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
        self.workspace_history_state.set_neq(history.clone());
        self.workspace_history_update_relay.send(history);
    }
}
