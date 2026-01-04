use crate::platform::{CurrentPlatform, Platform};
use crate::visualizer::timeline::TimePs;
use futures::{select, FutureExt, StreamExt};
use moonzoon_novyui::tokens::theme;
use serde::{Deserialize, Serialize};
use shared::UpMsg;
use shared::{
    self, AppConfig as SharedAppConfig, CanonicalPathPayload, DockMode, Theme as SharedTheme,
};
use std::sync::Arc;
use zoon::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: TimePs,
    pub end: TimePs,
}

fn compose_shared_app_config(
    theme: &Mutable<SharedTheme>,
    dock_mode: &Mutable<DockMode>,
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
    files_expanded_scopes: &Mutable<indexmap::IndexSet<String>>,
    files_selected_scope: &MutableVec<String>,
    files_vec_signal: &Mutable<Vec<shared::TrackedFile>>,
    variables_search_filter: &Mutable<String>,
) -> Option<shared::AppConfig> {
    let theme = theme.get();
    let dock_mode = dock_mode.get_cloned();
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

    let opened_files: Vec<CanonicalPathPayload> = files_vec_signal
        .get_cloned()
        .iter()
        .map(|f| CanonicalPathPayload::new(f.canonical_path.clone()))
        .collect();
    let expanded_scopes: Vec<String> = files_expanded_scopes.get_cloned().into_iter().collect();
    let selected_scope_id = files_selected_scope.lock_ref().first().cloned();
    let variables_search_filter = variables_search_filter.get_cloned();

    Some(shared::AppConfig {
        app: shared::AppSection::default(),
        workspace: shared::WorkspaceSection {
            opened_files,
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
            expanded_scopes,
            load_files_expanded_directories: expanded_directories,
            selected_scope_id,
            load_files_scroll_position: scroll_position,
            variables_search_filter,
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

    _browse_task: Arc<TaskHandle>,
    _expansion_detection_task: Arc<TaskHandle>,
    _selection_detection_task: Arc<TaskHandle>,
}

impl FilePickerDomain {
    pub fn new(
        initial_expanded: indexmap::IndexSet<String>,
        initial_scroll: i32,
        connection: std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        _connection_message_actor: crate::app::ConnectionMessageActor,
    ) -> Self {
        let (browse_request_sender, mut browse_request_receiver) =
            futures::channel::mpsc::unbounded::<String>();

        let errors = Mutable::new(std::collections::HashMap::<String, String>::new());
        let errors_for_task = errors.clone();
        let connection_for_task = connection.clone();

        let _browse_task = Arc::new(Task::start_droppable(async move {
            while let Some(path) = browse_request_receiver.next().await {
                zoon::println!("frontend: BrowseDirectory {path}");
                if let Err(e) = connection_for_task
                    .send_up_msg(shared::UpMsg::BrowseDirectory(path.clone()))
                    .await
                {
                    zoon::println!("ERROR: BrowseDirectory failed for {path}: {:?}", e);
                    errors_for_task.lock_mut().insert(path, format!("{:?}", e));
                }
            }
        }));

        let expanded_directories = Mutable::new(initial_expanded);
        let directory_cache = Mutable::new(std::collections::HashMap::new());

        // Expansion detection: trigger directory browsing for newly expanded paths
        // Config save is handled by the pure signal debouncer observing source Mutables directly
        let _expansion_detection_task = {
            let expanded = expanded_directories.clone();
            let cache = directory_cache.clone();
            let browse_sender = browse_request_sender.clone();
            Arc::new(Task::start_droppable({
                let previous = Mutable::new(indexmap::IndexSet::<String>::new());
                expanded.signal_cloned().dedupe_cloned().for_each_sync(move |current| {
                    let prev = previous.get_cloned();
                    for path in current.difference(&prev) {
                        if !cache.get_cloned().contains_key(path) && crate::platform::server_is_ready() {
                            let _ = browse_sender.unbounded_send(path.clone());
                        }
                    }
                    previous.set_neq(current);
                })
            }))
        };

        let selected_files = MutableVec::new();
        let selected_files_vec_signal = zoon::Mutable::new(Vec::new());

        // Selection detection task: sync selected_files → selected_files_vec_signal
        let _selection_detection_task = {
            let selected = selected_files.clone();
            let vec_signal = selected_files_vec_signal.clone();
            let current_vec = std::cell::RefCell::new(Vec::<String>::new());
            Arc::new(Task::start_droppable(
                selected.signal_vec_cloned().for_each(move |diff| {
                    {
                        let mut v = current_vec.borrow_mut();
                        match diff {
                            VecDiff::Replace { values } => *v = values,
                            VecDiff::InsertAt { index, value } => v.insert(index, value),
                            VecDiff::UpdateAt { index, value } => v[index] = value,
                            VecDiff::RemoveAt { index } => { v.remove(index); }
                            VecDiff::Move { old_index, new_index } => {
                                let item = v.remove(old_index);
                                v.insert(new_index, item);
                            }
                            VecDiff::Push { value } => v.push(value),
                            VecDiff::Pop {} => { v.pop(); }
                            VecDiff::Clear {} => v.clear(),
                        }
                    }
                    vec_signal.set(current_vec.borrow().clone());
                    async {}
                }),
            ))
        };

        Self {
            expanded_directories,
            scroll_position: Mutable::new(initial_scroll),
            directory_cache,
            directory_errors: errors,
            selected_files,
            selected_files_vec_signal,
            _browse_task,
            _expansion_detection_task,
            _selection_detection_task,
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

    /// Expand a directory - updates state only; detection task handles browse/save
    pub fn expand_directory(&self, path: String) {
        let mut current = self.expanded_directories.get_cloned();
        if current.insert(path) {
            self.expanded_directories.set_neq(current);
        }
    }

    /// Collapse a directory - updates state only; detection task handles save
    pub fn collapse_directory(&self, path: String) {
        let mut current = self.expanded_directories.get_cloned();
        if current.shift_remove(&path) {
            self.expanded_directories.set_neq(current);
        }
    }

    /// Set scroll position - config save handled by pure signal debouncer
    pub fn set_scroll_position(&self, position: i32) {
        self.scroll_position.set_neq(position);
    }

    /// Select a file - updates state only; detection task handles vec_signal sync
    pub fn select_file(&self, file_path: String) {
        let mut files = self.selected_files.lock_mut();
        if !files.iter().any(|p| p == &file_path) {
            files.push_cloned(file_path);
        }
    }

    /// Deselect a file - updates state only; detection task handles vec_signal sync
    pub fn deselect_file(&self, file_path: String) {
        self.selected_files.lock_mut().retain(|f| f != &file_path);
    }

    /// Clear all file selections - updates state only; detection task handles vec_signal sync
    pub fn clear_file_selection(&self) {
        self.selected_files.lock_mut().clear();
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

    pub toast_dismiss_ms: Mutable<u32>,
    pub plugins_state: Mutable<shared::PluginsSection>,
    pub workspace_history_state: Mutable<shared::WorkspaceHistory>,
    pub workspace_history_sender: futures::channel::mpsc::UnboundedSender<shared::WorkspaceHistory>,

    // File picker domain
    pub file_picker_domain: FilePickerDomain,

    // Keep ConnectionMessageActor alive to prevent channel disconnection
    _connection_message_actor: crate::app::ConnectionMessageActor,

    pub loaded_selected_variables: Vec<shared::SelectedVariable>,

    pub timeline_state_to_restore: Mutable<Option<TimelineState>>,
    pub timeline_state: Mutable<TimelineState>,

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
    _config_save_debouncer_task: Arc<TaskHandle>,
    _workspace_history_task: Arc<TaskHandle>,
    _selected_variables_snapshot_task: Arc<TaskHandle>,
    _clipboard_task: Arc<TaskHandle>,
    clipboard_sender: futures::channel::mpsc::UnboundedSender<String>,
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
        files_selected_scope: zoon::MutableVec<String>,
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

        let timeline_state_to_restore = Mutable::new(None);
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

        let toast_dismiss_ms = Mutable::new(config.ui.toast_dismiss_ms as u32);


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
            connection,
            connection_message_actor.clone(),
        );

        // Initialize TreeView Mutables for Files & Scopes panel
        let files_expanded_scopes = zoon::Mutable::new(indexmap::IndexSet::from_iter(
            config.workspace.expanded_scopes.iter().cloned(),
        ));
        // files_selected_scope is passed in from app.rs (shared with SelectedVariables domain)

        // Track SelectedVariables changes - snapshot for config saves
        // Config save is handled automatically by the pure signal debouncer
        let selected_variables_snapshot = Mutable::new(Vec::<shared::SelectedVariable>::new());
        let _selected_variables_snapshot_task = {
            let state = selected_variables_snapshot.clone();
            let variables_mutable = selected_variables.variables_vec_actor.clone();

            Arc::new(Task::start_droppable(
                variables_mutable
                    .signal_cloned()
                    .dedupe_cloned()
                    .for_each_sync(move |vars| {
                        state.set_neq(vars.clone());
                    }),
            ))
        };

        // Pure signal observation for config saves - no channels needed
        // The debouncer observes all source Mutables directly (no intermediate SessionState)
        let _config_save_debouncer_task = {
            let theme_clone = theme.clone();
            let dock_mode_clone = dock_mode.clone();
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
            let files_expanded_scopes_clone = files_expanded_scopes.clone();
            let files_selected_scope_clone = files_selected_scope.clone();
            let files_vec_signal_clone = tracked_files.files_vec_signal.clone();
            let variables_search_filter_clone = selected_variables.search_filter.clone();

            // Clone references for the combined signal
            let theme_for_signal = theme.clone();
            let dock_mode_for_signal = dock_mode.clone();
            let toast_for_signal = toast_dismiss_ms.clone();
            let timeline_for_signal = timeline_state.clone();
            let files_width_right_for_signal = files_panel_width_right.clone();
            let files_height_right_for_signal = files_panel_height_right.clone();
            let files_width_bottom_for_signal = files_panel_width_bottom.clone();
            let files_height_bottom_for_signal = files_panel_height_bottom.clone();
            let name_col_bottom_for_signal = name_column_width_bottom_state.clone();
            let name_col_right_for_signal = name_column_width_right_state.clone();
            let value_col_bottom_for_signal = value_column_width_bottom_state.clone();
            let value_col_right_for_signal = value_column_width_right_state.clone();
            let plugins_for_signal = plugins_state.clone();
            let workspace_history_for_signal = workspace_history_state.clone();
            let selected_vars_for_signal = selected_variables_snapshot.clone();
            let files_expanded_for_signal = files_expanded_scopes.clone();
            let files_selected_for_signal = files_selected_scope.clone();
            let files_vec_for_signal = tracked_files.files_vec_signal.clone();
            let vars_filter_for_signal = selected_variables.search_filter.clone();
            let picker_scroll_for_signal = file_picker_domain.scroll_position.clone();
            let picker_expanded_for_signal = file_picker_domain.expanded_directories.clone();

            Arc::new(Task::start_droppable(async move {
                // Combine all config-relevant signals into one trigger signal
                let config_changed = map_ref! {
                    let _ = theme_for_signal.signal(),
                    let _ = dock_mode_for_signal.signal_cloned(),
                    let _ = toast_for_signal.signal(),
                    let _ = timeline_for_signal.signal_cloned(),
                    let _ = files_width_right_for_signal.signal(),
                    let _ = files_height_right_for_signal.signal(),
                    let _ = files_width_bottom_for_signal.signal(),
                    let _ = files_height_bottom_for_signal.signal(),
                    let _ = name_col_bottom_for_signal.signal(),
                    let _ = name_col_right_for_signal.signal(),
                    let _ = value_col_bottom_for_signal.signal(),
                    let _ = value_col_right_for_signal.signal(),
                    let _ = plugins_for_signal.signal_cloned(),
                    let _ = workspace_history_for_signal.signal_cloned(),
                    let _ = selected_vars_for_signal.signal_cloned(),
                    let _ = files_expanded_for_signal.signal_cloned(),
                    let _ = files_selected_for_signal.signal_vec_cloned().map(|_| ()).to_signal_cloned(),
                    let _ = files_vec_for_signal.signal_cloned(),
                    let _ = vars_filter_for_signal.signal_cloned(),
                    let _ = picker_scroll_for_signal.signal(),
                    let _ = picker_expanded_for_signal.signal_cloned()
                    => ()
                };

                let mut config_stream = config_changed.to_stream().fuse();
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
                        result = config_stream.next() => {
                            if result.is_some() {
                                // Debounce loop - wait for quiet period, cancelling if new change arrives
                                loop {
                                    select! {
                                        // New config change cancels timer
                                        result = config_stream.next() => {
                                            if result.is_some() {
                                                continue; // Restart timer
                                            }
                                        }
                                        // Timer completes - do the save
                                        _ = zoon::Timer::sleep(300).fuse() => {
                                            if config_ready && crate::platform::server_is_ready() {
                                                if let Some(shared_config) = compose_shared_app_config(
                                                    &theme_clone,
                                                    &dock_mode_clone,
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
                                                    &files_expanded_scopes_clone,
                                                    &files_selected_scope_clone,
                                                    &files_vec_signal_clone,
                                                    &variables_search_filter_clone,
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

        let error_display = crate::error_display::ErrorDisplay::new();

        // Clipboard task - processes clipboard write requests
        let (clipboard_sender, mut clipboard_receiver) =
            futures::channel::mpsc::unbounded::<String>();
        let _clipboard_task = Arc::new(Task::start_droppable(async move {
            while let Some(text) = clipboard_receiver.next().await {
                if let Some(window) = web_sys::window() {
                    let navigator = window.navigator();

                    #[cfg(web_sys_unstable_apis)]
                    {
                        let clipboard = navigator.clipboard();
                        if let Err(e) =
                            wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text)).await
                        {
                            zoon::println!("ERROR: Failed to copy to clipboard: {:?}", e);
                        }
                    }

                    #[cfg(not(web_sys_unstable_apis))]
                    {
                        let _ = navigator;
                        zoon::println!(
                            "Clipboard API not available (web_sys_unstable_apis not enabled)"
                        );
                    }
                }
            }
        }));

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
            toast_dismiss_ms,
            plugins_state,
            workspace_history_state,
            workspace_history_sender,

            file_picker_domain,

            loaded_selected_variables: config.workspace.selected_variables.clone(),

            timeline_state_to_restore,
            timeline_state,

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
            _config_save_debouncer_task,
            _workspace_history_task,
            _selected_variables_snapshot_task,
            _clipboard_task,
            clipboard_sender,
            _connection_message_actor: connection_message_actor,
        }
    }

    /// Mark that a workspace switch is in progress, pausing config saves
    /// until the next ConfigLoaded arrives.
    pub fn mark_workspace_switching(&self) {
        self.config_loaded_flag.set(false);
    }

    /// Update timeline state - config save handled by pure signal debouncer
    pub fn update_timeline_state(&self, new_state: TimelineState) {
        self.timeline_state.set(new_state);
    }

    /// Toggle theme between light and dark
    /// Config save is handled automatically by the pure signal debouncer
    pub fn toggle_theme(&self) {
        let current = self.theme.get();
        let new_theme = match current {
            SharedTheme::Light => SharedTheme::Dark,
            SharedTheme::Dark => SharedTheme::Light,
        };
        self.set_theme(new_theme);
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
    /// Config save is handled automatically by the pure signal debouncer
    pub fn toggle_dock_mode(&self) {
        let current = self.dock_mode.get();
        let new_mode = match current {
            DockMode::Right => DockMode::Bottom,
            DockMode::Bottom => DockMode::Right,
        };
        self.set_dock_mode(new_mode);
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
        _selected_variables: &crate::selected_variables::SelectedVariables,
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
        self.timeline_state_to_restore.set(Some(timeline_state));

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

        // NOTE: Variables, scopes, and config_loaded_flag are NOT restored here.
        // They must be restored AFTER files have started loading to prevent:
        // 1. Variables showing "Loading..." state when files don't exist yet
        // 2. Scope references to non-existent files
        // 3. Config saves capturing incomplete state
        // Caller must call complete_initialization() which handles these.
    }

    pub fn set_config_loaded(&self) {
        self.config_loaded_flag.set(true);
    }

    /// Phase 4: Complete initialization with DIRECT backend calls (no relays!).
    /// Called after restore_config() has set all direct state.
    /// This method:
    /// 1. Sends directory browse requests directly to backend
    /// 2. Loads files from the config
    /// 3. Restores variables, scopes, and search filter AFTER files start loading
    /// This ordering prevents variables/scopes from referencing files that don't exist yet.
    pub async fn complete_initialization(
        &self,
        connection: &std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        tracked_files: &crate::tracked_files::TrackedFiles,
        selected_variables: &crate::selected_variables::SelectedVariables,
        loaded_config: &shared::AppConfig,
    ) {
        let opened_files = loaded_config.workspace.opened_files.clone();
        let expanded_directories = loaded_config.workspace.load_files_expanded_directories.clone();

        zoon::println!("[CONFIG] complete_initialization: DIRECT to backend (no relays)");
        zoon::println!("[CONFIG] complete_initialization: {} directories to check", expanded_directories.len());

        // Send directory browse requests DIRECTLY to backend (bypasses relay race condition)
        // Fire-and-forget: don't await, let files start loading immediately (Issue #7 fix)
        let dirs_to_browse: Vec<String> = {
            let cache = self.file_picker_domain.directory_cache.lock_ref();
            expanded_directories
                .iter()
                .filter(|dir| !cache.contains_key(*dir))
                .cloned()
                .collect()
        };

        zoon::println!("[CONFIG] complete_initialization: {} directories need browsing", dirs_to_browse.len());

        // Fire-and-forget directory browses - don't block file loading
        for dir in dirs_to_browse {
            let conn = connection.clone();
            Task::start(async move {
                if let Err(e) = conn.send_up_msg(shared::UpMsg::BrowseDirectory(dir.clone())).await {
                    zoon::println!("ERROR: Failed to send BrowseDirectory for {}: {:?}", dir, e);
                }
            });
        }

        // Send file load requests directly (TrackedFiles processes immediately)
        if opened_files.is_empty() {
            zoon::println!("[CONFIG] complete_initialization: no files to load, clearing files and variables");
            tracked_files.clear_all_files();
            selected_variables.clear_selection();
        } else {
            zoon::println!("[CONFIG] complete_initialization: loading {} files via method", opened_files.len());
            tracked_files.load_config_files(opened_files);
        }

        // AFTER files have started loading, restore variables, scopes, and search filter
        // This prevents "orphan" references to non-existent files (Issue #3, #4, #5 fixes)
        selected_variables.set_search_filter(loaded_config.workspace.variables_search_filter.clone());

        if !loaded_config.workspace.opened_files.is_empty() {
            selected_variables.restore_variables(loaded_config.workspace.selected_variables.clone());
        }

        let expanded_set: indexmap::IndexSet<String> = loaded_config
            .workspace
            .expanded_scopes
            .iter()
            .cloned()
            .collect();
        self.files_expanded_scopes.set(expanded_set);

        if let Some(scope_id) = loaded_config.workspace.selected_scope_id.clone() {
            let mut scope_guard = self.files_selected_scope.lock_mut();
            scope_guard.clear();
            scope_guard.push_cloned(scope_id);
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

    /// Copy text to clipboard via channel (processed by stored clipboard task)
    pub fn copy_to_clipboard(&self, text: String) {
        let _ = self.clipboard_sender.unbounded_send(text);
    }
}

/// Handles workspace picker state persistence using pure signal observation.
/// Encapsulates all coordination logic (selection, expanded, scroll, visibility restore).
pub struct WorkspacePickerPersistence {
    _selection_observer: Arc<TaskHandle>,
    _expanded_observer: Arc<TaskHandle>,
    _scroll_observer: Arc<TaskHandle>,
    _visibility_observer: Arc<TaskHandle>,
}

impl WorkspacePickerPersistence {
    pub fn new(
        workspace_picker_domain: FilePickerDomain,
        workspace_picker_visible: Mutable<bool>,
        workspace_picker_target: Mutable<Option<String>>,
        config: AppConfig,
    ) -> Self {
        let restoring_flag = Mutable::new(false);

        // Selection observer - updates target and records workspace selection
        let _selection_observer = {
            let selected_signal = workspace_picker_domain.selected_files_vec_signal.clone();
            let config_clone = config.clone();
            let domain_clone = workspace_picker_domain.clone();
            let target_clone = workspace_picker_target.clone();
            Arc::new(Task::start_droppable(
                selected_signal.signal_cloned().dedupe_cloned().for_each_sync(move |selection| {
                    crate::app::emit_trace("workspace_picker_selection", format!("paths={selection:?}"));
                    if let Some(path) = selection.first().cloned() {
                        target_clone.set_neq(Some(path.clone()));
                        config_clone.record_workspace_selection(&path);

                        let expanded_vec: Vec<String> = domain_clone
                            .expanded_directories
                            .lock_ref()
                            .iter()
                            .cloned()
                            .collect();
                        crate::app::emit_trace(
                            "workspace_picker_selection",
                            format!("selection={selection:?} expanded_paths={expanded_vec:?}"),
                        );
                        Self::publish_snapshot(&config_clone, &domain_clone, Some(expanded_vec));
                    }
                }),
            ))
        };

        // Expanded observer - updates tree state in config
        let _expanded_observer = {
            let expanded_dirs = workspace_picker_domain.expanded_directories.clone();
            let config_clone = config.clone();
            let restoring_flag_clone = restoring_flag.clone();
            let visible_clone = workspace_picker_visible.clone();
            let is_first = std::cell::Cell::new(true);
            Arc::new(Task::start_droppable(
                expanded_dirs.signal_cloned().dedupe_cloned().for_each_sync(move |expanded_set| {
                    // Skip initial value to avoid persisting on startup
                    if is_first.get() {
                        is_first.set(false);
                        return;
                    }
                    let expanded_vec: Vec<String> = expanded_set.iter().cloned().collect();
                    let is_visible = visible_clone.get();
                    let restoring = restoring_flag_clone.get();
                    crate::app::emit_trace(
                        "workspace_history_expanded_actor",
                        format!("paths={expanded_vec:?} restoring={restoring} visible={is_visible}"),
                    );
                    // Ignore teardown-driven empty updates when dialog is no longer visible
                    if !is_visible && expanded_vec.is_empty() {
                        crate::app::emit_trace(
                            "workspace_history_expanded_actor",
                            "skip_empty_invisible".to_string(),
                        );
                        return;
                    }
                    if restoring {
                        crate::app::emit_trace(
                            "workspace_history_expanded_actor",
                            "skip_restoring".to_string(),
                        );
                        return;
                    }
                    config_clone.update_workspace_picker_tree_state(expanded_vec);
                }),
            ))
        };

        // Scroll observer - updates scroll position in config
        let _scroll_observer = {
            let domain_clone = workspace_picker_domain.clone();
            let config_clone = config.clone();
            let restoring_flag_clone = restoring_flag.clone();
            Arc::new(Task::start_droppable(
                domain_clone.scroll_position.signal().dedupe().for_each_sync(move |position| {
                    if restoring_flag_clone.get() {
                        return;
                    }
                    let scroll_value = position as f64;
                    crate::app::emit_trace(
                        "workspace_picker_scroll",
                        format!("scroll_top={scroll_value}"),
                    );
                    config_clone.update_workspace_picker_scroll(scroll_value);
                }),
            ))
        };

        // Visibility observer - restores state when visible, publishes snapshot when closing
        let _visibility_observer = {
            let history_state = config.workspace_history_state.clone();
            let domain_clone = workspace_picker_domain.clone();
            let visible_clone = workspace_picker_visible.clone();
            let target_clone = workspace_picker_target.clone();
            let config_clone = config.clone();
            let restoring_flag_clone = restoring_flag.clone();
            Arc::new(Task::start_droppable(
                visible_clone.signal().dedupe().for_each_sync(move |visible| {
                    if visible {
                        restoring_flag_clone.set(true);
                        let history = history_state.get_cloned();
                        crate::app::emit_trace(
                            "workspace_picker_restore",
                            format!("stage=apply history={:?}", history.picker_tree_state),
                        );
                        Self::apply_tree_state(&history, &domain_clone);
                        let applied_state: Vec<String> = domain_clone
                            .expanded_directories
                            .lock_ref()
                            .iter()
                            .cloned()
                            .collect();
                        // Sync applied expansions into history state
                        config_clone.update_workspace_picker_tree_state(applied_state.clone());
                        crate::app::emit_trace(
                            "workspace_picker_restore",
                            format!("stage=post_apply expanded_paths={applied_state:?}"),
                        );
                        target_clone.set_neq(None);
                        domain_clone.selected_files_vec_signal.set_neq(Vec::new());
                        domain_clone.clear_file_selection();
                        restoring_flag_clone.set(false);
                    } else {
                        // Dialog is closing: publish final snapshot
                        Self::publish_snapshot(&config_clone, &domain_clone, None);
                        restoring_flag_clone.set(false);
                    }
                }),
            ))
        };

        Self {
            _selection_observer,
            _expanded_observer,
            _scroll_observer,
            _visibility_observer,
        }
    }

    fn apply_tree_state(history: &shared::WorkspaceHistory, domain: &FilePickerDomain) {
        if let Some(tree_state) = history.picker_tree_state.as_ref() {
            let mut expanded_set = indexmap::IndexSet::new();
            for entry in &tree_state.expanded_paths {
                expanded_set.insert(entry.clone());
            }
            let scroll_clamped = tree_state.scroll_top.max(0.0).round();
            let scroll_value = scroll_clamped.clamp(0.0, i32::MAX as f64) as i32;

            domain.expanded_directories.set_neq(expanded_set);
            domain.scroll_position.set_neq(scroll_value);
        }
    }

    fn publish_snapshot(
        config: &AppConfig,
        domain: &FilePickerDomain,
        known_expanded: Option<Vec<String>>,
    ) {
        let expanded_vec = known_expanded.unwrap_or_else(|| {
            domain
                .expanded_directories
                .lock_ref()
                .iter()
                .cloned()
                .collect()
        });
        if expanded_vec.is_empty() {
            crate::app::emit_trace(
                "workspace_picker_snapshot",
                "skipped empty expanded_paths".to_string(),
            );
            return;
        }
        crate::app::emit_trace(
            "workspace_picker_snapshot",
            format!("expanded_paths={expanded_vec:?}"),
        );
        config.update_workspace_picker_tree_state(expanded_vec);
    }
}
