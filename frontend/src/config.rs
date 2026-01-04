use crate::platform::{CurrentPlatform, Platform};
use crate::visualizer::timeline::TimePs;
use futures::{FutureExt, StreamExt, select};
use moonzoon_novyui::tokens::theme;
use serde::{Deserialize, Serialize};
use shared::UpMsg;
use shared::{
    self, AppConfig as SharedAppConfig, CanonicalPathPayload, DockMode, Theme as SharedTheme,
};
use std::sync::Arc;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use zoon::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: TimePs,
    pub end: TimePs,
}

fn compose_shared_app_config(
    theme: &Mutable<SharedTheme>,
    dock_mode: &Mutable<DockMode>,
    session: &Mutable<SessionState>,
    toast_dismiss_ms: &Mutable<u32>,
    file_picker_domain: &FilePickerDomain,
    selected_variables_snapshot: &Mutable<Vec<shared::SelectedVariable>>,
    files_width_right: &Mutable<f32>,
    files_height_right: &Mutable<f32>,
    files_width_bottom: &Mutable<f32>,
    files_height_bottom: &Mutable<f32>,
    name_column_width_bottom_state: &Mutable<f32>,
    name_column_width_right_state: &Mutable<f32>,
    value_column_width_bottom_state: &Mutable<f32>,
    value_column_width_right_state: &Mutable<f32>,
    timeline_state: &Mutable<TimelineState>,
    workspace_history_state: &Mutable<shared::WorkspaceHistory>,
    plugins_state: &Mutable<shared::PluginsSection>,
) -> Option<shared::AppConfig> {
    let theme = theme.get();
    let dock_mode = dock_mode.get_cloned();
    let session = session.get_cloned();
    let toast_dismiss_ms = toast_dismiss_ms.get();

    let expanded_directories_set = file_picker_domain.expanded_directories.get_cloned();
    let expanded_directories: Vec<String> = expanded_directories_set.into_iter().collect();
    let scroll_position = file_picker_domain.scroll_position.get();
    let selected_variables_snapshot = selected_variables_snapshot.get_cloned();

    let files_width_right = files_width_right.get();
    let files_height_right = files_height_right.get();
    let files_width_bottom = files_width_bottom.get();
    let files_height_bottom = files_height_bottom.get();

    let name_column_width_bottom = name_column_width_bottom_state.get();
    let name_column_width_right = name_column_width_right_state.get();
    let value_column_width_bottom = value_column_width_bottom_state.get();
    let value_column_width_right = value_column_width_right_state.get();
    let timeline_state = timeline_state.get_cloned();

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

/// File picker domain - simplified with direct method calls, minimal async
#[derive(Clone)]
pub struct FilePickerDomain {
    pub expanded_directories: Mutable<indexmap::IndexSet<String>>,
    pub scroll_position: Mutable<i32>,
    pub directory_cache: Mutable<std::collections::HashMap<String, Vec<shared::FileSystemItem>>>,
    pub directory_errors: Mutable<std::collections::HashMap<String, String>>,
    pub selected_files: MutableVec<String>,
    pub selected_files_vec_signal: zoon::Mutable<Vec<String>>,

    connection: std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
    save_sender: futures::channel::mpsc::UnboundedSender<()>,
}

impl FilePickerDomain {
    pub fn new(
        initial_expanded: indexmap::IndexSet<String>,
        initial_scroll: i32,
        save_sender: futures::channel::mpsc::UnboundedSender<()>,
        connection: std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        _connection_message_actor: crate::app::ConnectionMessageActor,
    ) -> Self {
        Self {
            expanded_directories: Mutable::new(initial_expanded),
            scroll_position: Mutable::new(initial_scroll),
            directory_cache: Mutable::new(std::collections::HashMap::new()),
            directory_errors: Mutable::new(std::collections::HashMap::new()),
            selected_files: MutableVec::new(),
            selected_files_vec_signal: zoon::Mutable::new(Vec::new()),
            connection,
            save_sender,
        }
    }

    pub fn expanded_directories_signal(&self) -> impl Signal<Item = indexmap::IndexSet<String>> {
        self.expanded_directories.signal_cloned()
    }

    pub fn scroll_position_signal(&self) -> impl Signal<Item = i32> {
        self.scroll_position.signal()
    }

    pub fn directory_cache_signal(
        &self,
    ) -> impl Signal<Item = std::collections::HashMap<String, Vec<shared::FileSystemItem>>> + use<>
    {
        self.directory_cache.signal_cloned()
    }

    pub fn directory_errors_signal(
        &self,
    ) -> impl Signal<Item = std::collections::HashMap<String, String>> {
        self.directory_errors.signal_cloned()
    }

    /// Handle directory contents received from backend
    pub fn on_directory_contents(&self, path: String, items: Vec<shared::FileSystemItem>) {
        self.directory_cache.lock_mut().insert(path, items);
    }

    /// Handle directory error received from backend
    pub fn on_directory_error(&self, path: String, error: String) {
        self.directory_errors.lock_mut().insert(path, error);
    }

    /// Expand a directory - updates state and sends backend request directly
    pub fn expand_directory(&self, path: String) {
        let mut current = self.expanded_directories.get_cloned();
        if current.insert(path.clone()) {
            self.expanded_directories.set_neq(current);
            let _ = self.save_sender.unbounded_send(());

            // Check cache, send backend request if not cached
            let cache = self.directory_cache.get_cloned();
            if !cache.contains_key(&path) && crate::platform::server_is_ready() {
                let conn = self.connection.clone();
                let errors = self.directory_errors.clone();
                let path_clone = path.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    zoon::println!("frontend: BrowseDirectory {path_clone}");
                    if let Err(e) = conn.send_up_msg(shared::UpMsg::BrowseDirectory(path_clone.clone())).await {
                        zoon::println!("ERROR: BrowseDirectory failed for {path_clone}: {:?}", e);
                        errors.lock_mut().insert(path_clone, format!("{:?}", e));
                    }
                });
            }
        }
    }

    /// Collapse a directory
    pub fn collapse_directory(&self, path: String) {
        let mut current = self.expanded_directories.get_cloned();
        if current.shift_remove(&path) {
            self.expanded_directories.set_neq(current);
            let _ = self.save_sender.unbounded_send(());
        }
    }

    /// Set scroll position
    pub fn set_scroll_position(&self, position: i32) {
        self.scroll_position.set_neq(position);
        let _ = self.save_sender.unbounded_send(());
    }

    /// Select a file
    pub fn select_file(&self, file_path: String) {
        let mut files = self.selected_files.lock_mut();
        if !files.iter().any(|p| p == &file_path) {
            files.push_cloned(file_path.clone());
            let current_files = files.to_vec();
            drop(files);
            self.selected_files_vec_signal.set_neq(current_files);
        }
    }

    /// Deselect a file
    pub fn deselect_file(&self, file_path: String) {
        self.selected_files.lock_mut().retain(|f| f != &file_path);
        self.selected_files_vec_signal.set_neq(self.selected_files.lock_ref().to_vec());
    }

    /// Clear all file selections
    pub fn clear_file_selection(&self) {
        self.selected_files.lock_mut().clear();
        self.selected_files_vec_signal.set_neq(Vec::new());
    }

    /// Get current expanded directories snapshot
    pub fn get_expanded_snapshot(&self) -> Vec<String> {
        self.expanded_directories.get_cloned().iter().cloned().collect()
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub theme: Mutable<SharedTheme>,
    pub dock_mode: Mutable<DockMode>,

    pub files_panel_width_right: Mutable<f32>,
    pub files_panel_height_right: Mutable<f32>,
    pub files_panel_width_bottom: Mutable<f32>,
    pub files_panel_height_bottom: Mutable<f32>,
    pub variables_panel_width: Mutable<f32>,
    pub timeline_panel_height: Mutable<f32>,
    pub variables_name_column_width: Mutable<f32>,
    pub variables_value_column_width: Mutable<f32>,

    pub session_state: Mutable<SessionState>,
    pub toast_dismiss_ms: Mutable<u32>,
    pub plugins_state: Mutable<shared::PluginsSection>,
    pub workspace_history_state: Mutable<shared::WorkspaceHistory>,
    pub workspace_history_sender: futures::channel::mpsc::UnboundedSender<shared::WorkspaceHistory>,

    // File picker domain
    pub file_picker_domain: FilePickerDomain,

    // Keep ConnectionMessageActor alive to prevent channel disconnection
    _connection_message_actor: crate::app::ConnectionMessageActor,

    pub loaded_selected_variables: Vec<shared::SelectedVariable>,

    pub timeline_restore_sender: futures::channel::mpsc::UnboundedSender<TimelineState>,
    pub timeline_restore_receiver: std::rc::Rc<std::cell::RefCell<Option<futures::channel::mpsc::UnboundedReceiver<TimelineState>>>>,
    pub timeline_state: Mutable<TimelineState>,
    pub session_state_sender: futures::channel::mpsc::UnboundedSender<SessionState>,

    pub save_sender: futures::channel::mpsc::UnboundedSender<()>,

    pub error_display: crate::error_display::ErrorDisplay,
    // TreeView state - Mutables required for TreeView external state API
    pub files_expanded_scopes: zoon::Mutable<indexmap::IndexSet<String>>,
    pub files_selected_scope: zoon::MutableVec<String>,
    pub config_loaded_flag: Mutable<bool>,

    // Internal state for dock-specific column widths (needed by restore_config)
    dock_mode_state: Mutable<DockMode>,
    name_column_width_bottom_state: Mutable<f32>,
    name_column_width_right_state: Mutable<f32>,
    value_column_width_bottom_state: Mutable<f32>,
    value_column_width_right_state: Mutable<f32>,

    pub selected_variables_snapshot: Mutable<Vec<shared::SelectedVariable>>,

    // Task handles to keep processors alive
    _session_state_task: Arc<TaskHandle>,
    _scroll_sync_task: Arc<TaskHandle>,
    _config_save_debouncer_task: Arc<TaskHandle>,
    _workspace_history_task: Arc<TaskHandle>,
    _treeview_sync_task: Arc<TaskHandle>,
    _tracked_files_sync_task: Arc<TaskHandle>,
    _variables_filter_bridge_task: Arc<TaskHandle>,
    _selected_variables_snapshot_task: Arc<TaskHandle>,
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
        let (workspace_history_sender, workspace_history_receiver) =
            futures::channel::mpsc::unbounded::<shared::WorkspaceHistory>();

        // Create config_loaded_flag early so workspace_history_task can use it
        let config_loaded_flag = Mutable::new(false);

        let _workspace_history_task = {
            let mut update_stream = workspace_history_receiver.fuse();
            let mut config_loaded_stream = config_loaded_flag.signal().to_stream().fuse();
            let mut ready = false;
            Arc::new(Task::start_droppable(async move {
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
                            loaded = config_loaded_stream.next() => {
                                if let Some(is_loaded) = loaded {
                                    ready = is_loaded;
                                }
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
                                        if ready {
                                            if let Err(e) = CurrentPlatform::send_message(UpMsg::UpdateWorkspaceHistory(pending.clone())).await {
                                                zoon::println!("ERROR: Failed to send UpdateWorkspaceHistory: {e}");
                                            }
                                        }
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
                                if ready {
                                    if let Err(e) = CurrentPlatform::send_message(UpMsg::UpdateWorkspaceHistory(pending.clone())).await {
                                        zoon::println!("ERROR: Failed to send UpdateWorkspaceHistory (debounced): {e}");
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }))
        };

        let (session_state_sender, session_state_receiver) =
            futures::channel::mpsc::unbounded::<SessionState>();
        let (save_sender, mut save_receiver) = futures::channel::mpsc::unbounded::<()>();
        let (timeline_restore_sender, timeline_restore_receiver) =
            futures::channel::mpsc::unbounded::<TimelineState>();

        let timeline_state = Mutable::new(TimelineState::default());

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

        let theme = Mutable::new(config.ui.theme);
        {
            let initial_novyui_theme = match config.ui.theme {
                SharedTheme::Light => theme::Theme::Light,
                SharedTheme::Dark => theme::Theme::Dark,
            };
            theme::init_theme(Some(initial_novyui_theme), None);
        }

        let initial_name_width = match config.workspace.dock_mode {
            DockMode::Right => name_column_width_right_state.get_cloned(),
            DockMode::Bottom => name_column_width_bottom_state.get_cloned(),
        };
        let variables_name_column_width = Mutable::new(initial_name_width);

        let initial_value_width = match config.workspace.dock_mode {
            DockMode::Right => value_column_width_right_state.get_cloned(),
            DockMode::Bottom => value_column_width_bottom_state.get_cloned(),
        };
        let variables_value_column_width = Mutable::new(initial_value_width);

        let dock_mode = Mutable::new(config.workspace.dock_mode.clone());

        let files_panel_width_right = Mutable::new(
            config
                .workspace
                .docked_right_dimensions
                .files_and_scopes_panel_width as f32,
        );
        let files_panel_height_right = Mutable::new(
            config
                .workspace
                .docked_right_dimensions
                .files_and_scopes_panel_height as f32,
        );

        let files_panel_width_bottom = Mutable::new(
            config
                .workspace
                .docked_bottom_dimensions
                .files_and_scopes_panel_width as f32,
        );

        let files_panel_height_bottom = Mutable::new(
            config
                .workspace
                .docked_bottom_dimensions
                .files_and_scopes_panel_height as f32,
        );

        let variables_panel_width = Mutable::new(DEFAULT_PANEL_WIDTH);
        let timeline_panel_height = Mutable::new(DEFAULT_TIMELINE_HEIGHT);

        let session_state = Mutable::new(SessionState {
            opened_files: Vec::new(),
            expanded_scopes: Vec::new(),
            selected_scope_id: None,
            variables_search_filter: config.workspace.variables_search_filter.clone(),
            file_picker_scroll_position: config.workspace.load_files_scroll_position,
            file_picker_expanded_directories: config
                .workspace
                .load_files_expanded_directories
                .clone(),
        });
        let _session_state_task = {
            let state = session_state.clone();
            let save_sender_for_session = save_sender.clone();
            let mut session_stream = session_state_receiver.fuse();
            Arc::new(Task::start_droppable(async move {
                while let Some(new_session) = session_stream.next().await {
                    state.set_neq(new_session);
                    let _ = save_sender_for_session.unbounded_send(());
                }
            }))
        };

        let toast_dismiss_ms = Mutable::new(config.ui.toast_dismiss_ms as u32);

        // Clone the sender for struct return since it will be moved into Task
        let save_sender_for_struct = save_sender.clone();

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
            save_sender.clone(),
            connection,
            connection_message_actor.clone(),
        );

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
        let _treeview_sync_task = {
            let expanded_scopes_for_sync = files_expanded_scopes.clone();
            let selected_scope_for_sync = files_selected_scope.clone();
            let session_sender_for_treeview = session_state_sender.clone();
            let session_state_for_treeview = session_state.clone();
            Arc::new(Task::start_droppable(async move {
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
                                let current_session = session_state_for_treeview.signal_cloned().to_stream().next().await.unwrap_or_default();
                                let updated_session = SessionState {
                                    expanded_scopes: expanded_set.iter().cloned().collect(),
                                    ..current_session
                                };
                                let _ = session_sender_for_treeview.unbounded_send(updated_session);
                            }
                        }
                        selected = selected_stream.next() => {
                            if let Some(selected_vec) = selected {
                                let scope_sel = selected_vec.iter().find(|id| id.starts_with("scope_")).cloned();
                                let current_session = session_state_for_treeview.signal_cloned().to_stream().next().await.unwrap_or_default();
                                let updated_session = SessionState {
                                    selected_scope_id: scope_sel,
                                    ..current_session
                                };
                                let _ = session_sender_for_treeview.unbounded_send(updated_session);
                            }
                        }
                    }
                }
            }))
        };

        // Create sync task for TrackedFiles to update opened_files
        let _tracked_files_sync_task = {
            let session_state_for_files = session_state.clone();
            let files_signal = tracked_files.files_vec_signal.clone();
            let session_sender_for_files = session_state_sender.clone();
            let files_expanded_for_sync = files_expanded_scopes.clone();
            let files_selected_for_sync = files_selected_scope.clone();

            Arc::new(Task::start_droppable(async move {
                // Force initial sync with current value
                let initial_files = files_signal.get_cloned();
                if !initial_files.is_empty() {
                    let file_paths: Vec<CanonicalPathPayload> = initial_files
                        .iter()
                        .map(|tracked_file| {
                            CanonicalPathPayload::new(tracked_file.canonical_path.clone())
                        })
                        .collect();

                    let mut current_session = session_state_for_files
                        .signal_cloned()
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

                    let _ = session_sender_for_files.unbounded_send(current_session);
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
                    let mut current_session = session_state_for_files
                        .signal_cloned()
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
                    let _ = session_sender_for_files.unbounded_send(current_session);
                }
            }))
        };

        // Bridge variables search filter between SelectedVariables domain and SessionState
        let _variables_filter_bridge_task = {
            let session_state_for_bridge = session_state.clone();
            let session_sender_for_bridge = session_state_sender.clone();
            let selected_variables_for_bridge = selected_variables.clone();

            Arc::new(Task::start_droppable(async move {
                let mut session_state_stream =
                    session_state_for_bridge.signal_cloned().to_stream().fuse();
                let mut filter_signal_stream = selected_variables_for_bridge.search_filter.signal_cloned().to_stream().fuse();

                let mut current_session = session_state_stream.next().await.unwrap_or_default();
                let mut current_filter = current_session.variables_search_filter.clone();

                loop {
                    select! {
                        session = session_state_stream.next() => {
                            if let Some(session_state) = session {
                                if current_filter != session_state.variables_search_filter {
                                    current_filter = session_state.variables_search_filter.clone();
                                    selected_variables_for_bridge.set_search_filter(current_filter.clone());
                                }
                                current_session = session_state;
                            } else {
                                break;
                            }
                        }
                        filter = filter_signal_stream.next() => {
                            if let Some(filter_text) = filter {
                                if current_filter == filter_text {
                                    continue;
                                }
                                current_filter = filter_text.clone();
                                current_session.variables_search_filter = filter_text;
                                let _ = session_sender_for_bridge.unbounded_send(current_session.clone());
                            } else {
                                break;
                            }
                        }
                    }
                }
            }))
        };

        // Track SelectedVariables changes to trigger config saves with latest snapshot
        let selected_variables_snapshot = Mutable::new(Vec::<shared::SelectedVariable>::new());
        let _selected_variables_snapshot_task = {
            let state = selected_variables_snapshot.clone();
            let selected_variables_for_snapshot = selected_variables.clone();
            let save_sender_for_snapshot = save_sender.clone();

            Arc::new(Task::start_droppable(async move {
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
                        let _ = save_sender_for_snapshot.unbounded_send(());
                    }
                }
            }))
        };

        // File picker changes now trigger config save through FilePickerDomain
        // Use nested Task pattern for debouncing
        let _config_save_debouncer_task = {
            let theme_clone = theme.clone();
            let dock_mode_clone = dock_mode.clone();
            let session_clone = session_state.clone();
            let toast_clone = toast_dismiss_ms.clone();
            let timeline_state_clone = timeline_state.clone();
            let file_picker_domain_clone = file_picker_domain.clone();
            let selected_variables_snapshot_clone = selected_variables_snapshot.clone();
            let config_loaded_flag_for_saver = config_loaded_flag.clone();
            let files_width_right_clone = files_panel_width_right.clone();
            let files_height_right_clone = files_panel_height_right.clone();
            let files_width_bottom_clone = files_panel_width_bottom.clone();
            let files_height_bottom_clone = files_panel_height_bottom.clone();
            let name_column_width_bottom_state_clone = name_column_width_bottom_state.clone();
            let name_column_width_right_state_clone = name_column_width_right_state.clone();
            let value_column_width_bottom_state_clone = value_column_width_bottom_state.clone();
            let value_column_width_right_state_clone = value_column_width_right_state.clone();
            let plugins_state_clone = plugins_state.clone();
            let workspace_history_state_clone_for_save = workspace_history_state.clone();

            Arc::new(Task::start_droppable(async move {
                let mut save_stream = save_receiver.fuse();
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
                        result = save_stream.next() => {
                            if let Some(()) = result {
                                // Debounce loop - wait for quiet period, cancelling if new request arrives
                                loop {
                                    select! {
                                        // New save request cancels timer
                                        result = save_stream.next() => {
                                            if let Some(()) = result {
                                                continue; // Restart timer
                                            }
                                        }
                                        // Timer completes - do the save
                                        _ = zoon::Timer::sleep(300).fuse() => {
                                            if config_ready && crate::platform::server_is_ready() {
                                                if let Some(shared_config) = compose_shared_app_config(
                                                    &theme_clone,
                                                    &dock_mode_clone,
                                                    &session_clone,
                                                    &toast_clone,
                                                    &file_picker_domain_clone,
                                                    &selected_variables_snapshot_clone,
                                                    &files_width_right_clone,
                                                    &files_height_right_clone,
                                                    &files_width_bottom_clone,
                                                    &files_height_bottom_clone,
                                                    &name_column_width_bottom_state_clone,
                                                    &name_column_width_right_state_clone,
                                                    &value_column_width_bottom_state_clone,
                                                    &value_column_width_right_state_clone,
                                                    &timeline_state_clone,
                                                    &workspace_history_state_clone_for_save,
                                                    &plugins_state_clone,
                                                ) {
                                                    if let Err(e) = CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config)).await {
                                                        zoon::println!("ERROR: Failed to send SaveConfig: {e}");
                                                    }
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
            }))
        };

        // Complex bridge pattern removed - using direct FilePickerDomain events

        let _scroll_sync_task = {
            let scroll_position_sync = file_picker_domain.scroll_position.clone();
            let session_scroll_sync = session_state.clone();
            let session_sender_for_scroll = session_state_sender.clone();

            Arc::new(Task::start_droppable(async move {
                let mut scroll_stream = scroll_position_sync.signal().to_stream();

                while let Some(scroll_position) = scroll_stream.next().await {
                    // Create updated session with new scroll position
                    let current_session = session_scroll_sync
                        .signal_cloned()
                        .to_stream()
                        .next()
                        .await
                        .unwrap_or_default();
                    let updated_session = SessionState {
                        file_picker_scroll_position: scroll_position,
                        ..current_session
                    };
                    let _ = session_sender_for_scroll.unbounded_send(updated_session);
                }
            }))
        };

        let error_display = crate::error_display::ErrorDisplay::new();

        Self {
            theme,
            dock_mode,
            files_panel_width_right,
            files_panel_height_right,
            files_panel_width_bottom,
            files_panel_height_bottom,
            variables_panel_width,
            timeline_panel_height,
            variables_name_column_width,
            variables_value_column_width,
            session_state,
            toast_dismiss_ms,
            plugins_state,
            workspace_history_state,
            workspace_history_sender,

            file_picker_domain,

            loaded_selected_variables: config.workspace.selected_variables.clone(),

            timeline_restore_sender,
            timeline_restore_receiver: std::rc::Rc::new(std::cell::RefCell::new(Some(timeline_restore_receiver))),
            timeline_state,
            session_state_sender,
            save_sender: save_sender_for_struct,

            error_display,
            files_expanded_scopes,
            files_selected_scope,
            config_loaded_flag,
            dock_mode_state,
            name_column_width_bottom_state,
            name_column_width_right_state,
            value_column_width_bottom_state,
            value_column_width_right_state,
            selected_variables_snapshot,
            _session_state_task,
            _scroll_sync_task,
            _config_save_debouncer_task,
            _workspace_history_task,
            _treeview_sync_task,
            _tracked_files_sync_task,
            _variables_filter_bridge_task,
            _selected_variables_snapshot_task,
            _connection_message_actor: connection_message_actor,
        }
    }

    /// Mark that a workspace switch is in progress, pausing config saves
    /// until the next ConfigLoaded arrives.
    pub fn mark_workspace_switching(&self) {
        self.config_loaded_flag.set(false);
    }

    /// Request config to be saved (debounced internally)
    pub fn request_config_save(&self) {
        let _ = self.save_sender.unbounded_send(());
    }

    /// Update timeline state directly and trigger config save
    pub fn update_timeline_state(&self, new_state: TimelineState) {
        self.timeline_state.set(new_state);
        let _ = self.save_sender.unbounded_send(());
    }

    /// Toggle theme between light and dark
    pub fn toggle_theme(&self) {
        let current = self.theme.get();
        let new_theme = match current {
            SharedTheme::Light => SharedTheme::Dark,
            SharedTheme::Dark => SharedTheme::Light,
        };
        self.set_theme(new_theme);
        self.request_config_save();
    }

    /// Set theme to specific value (for config loading)
    pub fn set_theme(&self, theme: SharedTheme) {
        self.theme.set(theme);
        let novyui_theme = match theme {
            SharedTheme::Light => theme::Theme::Light,
            SharedTheme::Dark => theme::Theme::Dark,
        };
        theme::set_theme(novyui_theme);
    }

    /// Toggle dock mode between right and bottom
    pub fn toggle_dock_mode(&self) {
        let current = self.dock_mode.get();
        let new_mode = match current {
            DockMode::Right => DockMode::Bottom,
            DockMode::Bottom => DockMode::Right,
        };
        self.set_dock_mode(new_mode);
        self.request_config_save();
    }

    /// Set dock mode to specific value (for config loading)
    pub fn set_dock_mode(&self, mode: DockMode) {
        let current = self.dock_mode.get();
        if current != mode {
            // Snapshot current dock's column widths before switching
            match current {
                DockMode::Bottom => {
                    self.name_column_width_bottom_state
                        .set_neq(self.variables_name_column_width.get());
                    self.value_column_width_bottom_state
                        .set_neq(self.variables_value_column_width.get());
                }
                DockMode::Right => {
                    self.name_column_width_right_state
                        .set_neq(self.variables_name_column_width.get());
                    self.value_column_width_right_state
                        .set_neq(self.variables_value_column_width.get());
                }
            }
            // Switch dock mode
            self.dock_mode.set(mode);
            self.dock_mode_state.set(mode);
            // Restore new dock's column widths
            match mode {
                DockMode::Bottom => {
                    self.variables_name_column_width
                        .set_neq(self.name_column_width_bottom_state.get());
                    self.variables_value_column_width
                        .set_neq(self.value_column_width_bottom_state.get());
                }
                DockMode::Right => {
                    self.variables_name_column_width
                        .set_neq(self.name_column_width_right_state.get());
                    self.variables_value_column_width
                        .set_neq(self.value_column_width_right_state.get());
                }
            }
        }
    }

    /// Update config from loaded backend data
    pub fn update_from_loaded_config(&self, loaded_config: shared::AppConfig) {
        self.plugins_state.set(loaded_config.plugins.clone());
        // Update theme directly
        self.set_theme(loaded_config.ui.theme);

        // Update dock mode directly
        self.set_dock_mode(loaded_config.workspace.dock_mode);

        self.workspace_history_state
            .set_neq(loaded_config.global.workspace_history.clone());

        // Update expanded directories directly (bypass relay to avoid feedback loop)
        {
            let mut expanded = self
                .file_picker_domain
                .expanded_directories
                .lock_mut();
            for dir in &loaded_config.workspace.load_files_expanded_directories {
                expanded.insert(dir.clone());
            }
        }
        // Request directory contents for uncached directories via direct method
        {
            let cache = self.file_picker_domain.directory_cache.lock_ref();
            for dir in &loaded_config.workspace.load_files_expanded_directories {
                if !cache.contains_key(dir) {
                    self.file_picker_domain.expand_directory(dir.clone());
                }
            }
        }

        // Update scroll position directly (no debounce needed for restore)
        self.file_picker_domain
            .scroll_position
            .set_neq(loaded_config.workspace.load_files_scroll_position);
    }

    /// Restore all config state from loaded backend data (replaces config_loaded_actor)
    /// Called directly by ConnectionMessageActor when ConfigLoaded arrives.
    /// Phase 3: Direct state updates only - file loading happens in complete_initialization()
    pub fn restore_config(
        &self,
        loaded_config: shared::AppConfig,
        selected_variables: &crate::selected_variables::SelectedVariables,
    ) {
        // Update global state
        self.plugins_state.set(loaded_config.plugins.clone());
        self.workspace_history_state
            .set_neq(loaded_config.global.workspace_history.clone());
        self.dock_mode_state
            .set(loaded_config.workspace.dock_mode.clone());

        // Update dimension states for both dock modes
        let loaded_files_width_right = loaded_config
            .workspace
            .docked_right_dimensions
            .files_and_scopes_panel_width as f32;
        let loaded_files_height_right = loaded_config
            .workspace
            .docked_right_dimensions
            .files_and_scopes_panel_height as f32;
        let loaded_files_width_bottom = loaded_config
            .workspace
            .docked_bottom_dimensions
            .files_and_scopes_panel_width as f32;
        let loaded_files_height_bottom = loaded_config
            .workspace
            .docked_bottom_dimensions
            .files_and_scopes_panel_height as f32;

        self.files_panel_width_right.set_neq(loaded_files_width_right);
        self.files_panel_height_right.set_neq(loaded_files_height_right);
        self.files_panel_width_bottom.set_neq(loaded_files_width_bottom);
        self.files_panel_height_bottom.set_neq(loaded_files_height_bottom);

        // Update column width states
        let loaded_name_bottom = loaded_config
            .workspace
            .docked_bottom_dimensions
            .selected_variables_panel_name_column_width
            .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64) as f32;
        let loaded_name_right = loaded_config
            .workspace
            .docked_right_dimensions
            .selected_variables_panel_name_column_width
            .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64) as f32;
        self.name_column_width_bottom_state.set(loaded_name_bottom);
        self.name_column_width_right_state.set(loaded_name_right);

        let loaded_value_bottom = loaded_config
            .workspace
            .docked_bottom_dimensions
            .selected_variables_panel_value_column_width
            .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64) as f32;
        let loaded_value_right = loaded_config
            .workspace
            .docked_right_dimensions
            .selected_variables_panel_value_column_width
            .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64) as f32;
        self.value_column_width_bottom_state.set(loaded_value_bottom);
        self.value_column_width_right_state.set(loaded_value_right);

        // Set current dock mode's column widths directly
        let current_name_width = match loaded_config.workspace.dock_mode {
            DockMode::Right => loaded_name_right,
            DockMode::Bottom => loaded_name_bottom,
        };
        self.variables_name_column_width.set_neq(current_name_width);

        let current_value_width = match loaded_config.workspace.dock_mode {
            DockMode::Right => loaded_value_right,
            DockMode::Bottom => loaded_value_bottom,
        };
        self.variables_value_column_width.set_neq(current_value_width);

        // Restore timeline state
        let timeline_cfg = &loaded_config.workspace.timeline;
        let visible_start_ps = timeline_cfg.visible_range_start_ps;
        let mut visible_end_ps = timeline_cfg.visible_range_end_ps;
        if visible_end_ps <= visible_start_ps {
            visible_end_ps = visible_start_ps + 1;
        }

        let mut cursor_position_ps = timeline_cfg.cursor_position_ps;
        if cursor_position_ps < visible_start_ps || cursor_position_ps > visible_end_ps {
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

        self.update_timeline_state(timeline_state.clone());
        let _ = self.timeline_restore_sender.unbounded_send(timeline_state);

        // Update theme and dock mode directly
        self.set_theme(loaded_config.ui.theme);
        self.set_dock_mode(loaded_config.workspace.dock_mode);

        // Phase 3: Update FilePickerDomain expanded directories directly (no relay sends)
        // The actual directory loading requests will be sent in complete_initialization()
        {
            let mut expanded = self
                .file_picker_domain
                .expanded_directories
                .lock_mut();
            expanded.clear();
            for dir in &loaded_config.workspace.load_files_expanded_directories {
                expanded.insert(dir.clone());
            }
        }

        // Update scroll position directly
        self.file_picker_domain
            .scroll_position
            .set_neq(loaded_config.workspace.load_files_scroll_position);

        // Synchronize session state
        let _ = self.session_state_sender.unbounded_send(SessionState {
            opened_files: loaded_config.workspace.opened_files.clone(),
            expanded_scopes: loaded_config.workspace.expanded_scopes.clone(),
            selected_scope_id: loaded_config.workspace.selected_scope_id.clone(),
            variables_search_filter: loaded_config.workspace.variables_search_filter.clone(),
            file_picker_scroll_position: loaded_config.workspace.load_files_scroll_position,
            file_picker_expanded_directories: loaded_config
                .workspace
                .load_files_expanded_directories
                .clone(),
        });

        // Restore selected variables ONLY if there are files to load
        // (prevents orphan variables showing "Loading..." when files aren't loaded)
        if !loaded_config.workspace.opened_files.is_empty() {
            selected_variables.restore_variables(loaded_config.workspace.selected_variables.clone());
        } else {
            selected_variables.clear_selection();
        }

        // Restore expanded scopes and selected scope (no delay needed with direct calls)
        let expanded_set: indexmap::IndexSet<String> = loaded_config
            .workspace
            .expanded_scopes
            .iter()
            .cloned()
            .collect();
        self.files_expanded_scopes.set(expanded_set);

        if let Some(scope_id) = loaded_config.workspace.selected_scope_id.clone() {
            self.files_selected_scope.lock_mut().clear();
            self.files_selected_scope.lock_mut().push_cloned(scope_id);
        }

        self.config_loaded_flag.set(true);
    }

    /// Phase 4: Complete initialization with DIRECT backend calls (no relays!).
    /// Called after restore_config() has set all direct state.
    /// This method sends directory browse and file load requests directly to backend,
    /// bypassing the relay system to avoid race conditions.
    pub async fn complete_initialization(
        &self,
        connection: &std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        tracked_files: &crate::tracked_files::TrackedFiles,
        selected_variables: &crate::selected_variables::SelectedVariables,
        opened_files: Vec<CanonicalPathPayload>,
        expanded_directories: Vec<String>,
    ) {
        zoon::println!("[CONFIG] complete_initialization: DIRECT to backend (no relays)");
        zoon::println!("[CONFIG] complete_initialization: {} directories to check", expanded_directories.len());

        // Send directory browse requests DIRECTLY to backend (bypasses relay race condition)
        // Collect directories to browse first (release lock before awaiting)
        let dirs_to_browse: Vec<String> = {
            let cache = self.file_picker_domain.directory_cache.lock_ref();
            expanded_directories
                .iter()
                .filter(|dir| !cache.contains_key(*dir))
                .cloned()
                .collect()
        };

        zoon::println!("[CONFIG] complete_initialization: {} directories need browsing", dirs_to_browse.len());

        // Send all BrowseDirectory requests - use join_all to properly await all of them
        // (Don't use Task::start_droppable which would drop handles immediately)
        use futures::future::join_all;
        let connection_clone = connection.clone();
        let browse_futures: Vec<_> = dirs_to_browse
            .into_iter()
            .map(|dir| {
                let conn = connection_clone.clone();
                async move {
                    zoon::println!("[CONFIG] complete_initialization: sending BrowseDirectory {}", dir);
                    if let Err(e) = conn.send_up_msg(shared::UpMsg::BrowseDirectory(dir.clone())).await {
                        zoon::println!("ERROR: Failed to send BrowseDirectory for {}: {:?}", dir, e);
                    }
                }
            })
            .collect();

        join_all(browse_futures).await;

        // Send file load requests directly (TrackedFiles processes immediately)
        if opened_files.is_empty() {
            zoon::println!("[CONFIG] complete_initialization: no files to load, clearing files and variables");
            tracked_files.clear_all_files();
            selected_variables.clear_selection();
        } else {
            zoon::println!("[CONFIG] complete_initialization: loading {} files via method", opened_files.len());
            tracked_files.load_config_files(opened_files);
        }
    }

    pub fn record_workspace_selection(&self, path: &str) {
        if path.is_empty() {
            return;
        }
        let mut history = self.workspace_history_state.get_cloned();
        history.touch_path(path, shared::WORKSPACE_HISTORY_MAX_RECENTS);
        self.workspace_history_state.set_neq(history.clone());
        // Let the workspace_history_actor persist it after ConfigLoaded (simple and reliable).
        let _ = self.workspace_history_sender.unbounded_send(history);
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
        let _ = self.workspace_history_sender.unbounded_send(history);
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
        let _ = self.workspace_history_sender.unbounded_send(history);
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
        let _ = self.workspace_history_sender.unbounded_send(history);
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
        let _ = self.workspace_history_sender.unbounded_send(history);
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
        let _ = self.workspace_history_sender.unbounded_send(history);
    }

    /// Copy text to clipboard (direct method, replaces clipboard_copy_requested_relay.send())
    pub fn copy_to_clipboard(&self, text: String) {
        use wasm_bindgen_futures::spawn_local;
        use wasm_bindgen_futures::JsFuture;

        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();

                #[cfg(web_sys_unstable_apis)]
                {
                    let clipboard = navigator.clipboard();
                    if let Err(e) = JsFuture::from(clipboard.write_text(&text)).await {
                        zoon::println!("ERROR: Failed to copy to clipboard: {:?}", e);
                    }
                }

                #[cfg(not(web_sys_unstable_apis))]
                {
                    zoon::println!("Clipboard API not available (web_sys_unstable_apis not enabled)");
                }
            }
        });
    }
}
