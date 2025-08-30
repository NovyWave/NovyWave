use zoon::*;
use shared::{self, AppConfig as SharedAppConfig, DockMode, Theme as SharedTheme};
use crate::time_types::TimeNs;
use crate::dataflow::{Actor, relay, Relay};
use crate::platform::{Platform, CurrentPlatform};
use futures::{StreamExt, select};
use shared::UpMsg;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

// === SHARED TYPES FOR ACTORS ===

/// Automatic config saver that watches all config signals and debounces saves
#[derive(Debug, Clone)]
struct ConfigSaver {
    _task_handle: Arc<TaskHandle>,
}

impl ConfigSaver {
    fn with_task_handle(task_handle: TaskHandle) -> Self {
        Self {
            _task_handle: Arc::new(task_handle),
        }
    }
}

impl ConfigSaver {
    /// Create new config saver that watches all provided actors
    pub fn new(
        theme_actor: Actor<SharedTheme>,
        dock_mode_actor: Actor<DockMode>,
        panel_right_actor: Actor<PanelDimensions>,
        panel_bottom_actor: Actor<PanelDimensions>,
        timeline_actor: Actor<TimelineState>,
        session_actor: Actor<SessionState>,
        ui_actor: Actor<UiState>,
        dialogs_actor: Actor<DialogsData>,
    ) -> Self {
        use futures::StreamExt;
        
        let task_handle = Task::start_droppable(async move {
            let mut debounce_task: Option<TaskHandle> = None;
            zoon::println!("💾 ConfigSaver: Starting to watch Actor signals...");
            
            // ✅ CACHE CURRENT VALUES PATTERN - Cache values as they flow through streams
            let mut cached_theme = SharedTheme::Light;
            let mut cached_dock_mode = DockMode::Right;
            let mut cached_panel_right = PanelDimensions::default();
            let mut cached_panel_bottom = PanelDimensions::default();
            let mut cached_timeline = TimelineState::default();
            let mut cached_session = SessionState::default();
            let mut cached_ui = UiState::default();
            let mut cached_dialogs = DialogsData::default();
            
            // Create signal streams
            let mut theme_stream = theme_actor.signal().to_stream().fuse();
            let mut dock_stream = dock_mode_actor.signal().to_stream().fuse();
            let mut panel_right_stream = panel_right_actor.signal().to_stream().fuse();
            let mut panel_bottom_stream = panel_bottom_actor.signal().to_stream().fuse();
            let mut timeline_stream = timeline_actor.signal().to_stream().fuse();
            let mut session_stream = session_actor.signal().to_stream().fuse();
            let mut ui_stream = ui_actor.signal().to_stream().fuse();
            let mut dialogs_stream = dialogs_actor.signal().to_stream().fuse();
            
            loop {
                select! {
                    result = theme_stream.next() => {
                        if let Some(theme) = result {
                            cached_theme = theme; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: Theme changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                    result = dock_stream.next() => {
                        if let Some(dock_mode) = result {
                            cached_dock_mode = dock_mode; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: Dock mode changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                    result = panel_right_stream.next() => {
                        if let Some(panel_right) = result {
                            cached_panel_right = panel_right; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: Right panel changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                    result = panel_bottom_stream.next() => {
                        if let Some(panel_bottom) = result {
                            cached_panel_bottom = panel_bottom; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: Bottom panel changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                    result = timeline_stream.next() => {
                        if let Some(timeline) = result {
                            cached_timeline = timeline; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: Timeline changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                    result = session_stream.next() => {
                        if let Some(session) = result {
                            cached_session = session; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: Session changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                    result = ui_stream.next() => {
                        if let Some(ui_state) = result {
                            cached_ui = ui_state; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: UI state changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                    result = dialogs_stream.next() => {
                        if let Some(dialogs) = result {
                            cached_dialogs = dialogs; // ✅ Cache current value
                            zoon::println!("🔄 ConfigSaver: Dialogs changed, scheduling save");
                            Self::schedule_debounced_save_with_cached_values(
                                &mut debounce_task,
                                cached_theme.clone(),
                                cached_dock_mode.clone(),
                                cached_panel_right.clone(),
                                cached_panel_bottom.clone(),
                                cached_timeline.clone(),
                                cached_session.clone(),
                                cached_ui.clone(),
                                cached_dialogs.clone(),
                            ).await;
                        }
                    }
                }
            }
        });
        
        Self::with_task_handle(task_handle)
    }
    
    /// Schedule a debounced save with cached values - cancels previous pending save
    async fn schedule_debounced_save_with_cached_values(
        debounce_task: &mut Option<TaskHandle>,
        theme: SharedTheme,
        dock_mode: DockMode,
        panel_right: PanelDimensions,
        panel_bottom: PanelDimensions,
        timeline: TimelineState,
        session: SessionState,
        ui: UiState,
        dialogs: DialogsData,
    ) {
        // Cancel any pending save
        *debounce_task = None;
        
        // Schedule new save with 1 second debounce using cached values
        let handle = Task::start_droppable(async move {
            Timer::sleep(1000).await;
            zoon::println!("💾 ConfigSaver: Executing debounced save with cached values");
            save_config_with_cached_values(
                theme, dock_mode, panel_right, panel_bottom, 
                timeline, session, ui, dialogs
            ).await;
        });
        
        *debounce_task = Some(handle);
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: TimeNs,
    pub end: TimeNs,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PanelDimensions {
    pub files_panel_width: f32,
    pub files_panel_height: f32,
    pub variables_panel_width: f32,
    pub timeline_panel_height: f32,
    pub variables_name_column_width: f32,
    pub variables_value_column_width: f32,
}

impl Default for PanelDimensions {
    fn default() -> Self {
        Self {
            files_panel_width: 300.0,
            files_panel_height: 300.0,
            variables_panel_width: 300.0,
            timeline_panel_height: 200.0,
            variables_name_column_width: 180.0,
            variables_value_column_width: 100.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimelineState {
    pub cursor_position: TimeNs,
    pub visible_range: TimeRange,
    pub zoom_level: f64,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            cursor_position: TimeNs::ZERO,
            visible_range: TimeRange {
                start: TimeNs::ZERO,
                end: TimeNs::from_nanos(100_000_000_000),
            },
            zoom_level: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionState {
    pub opened_files: Vec<String>,
    pub variables_search_filter: String,
    pub file_picker_scroll_position: i32,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            variables_search_filter: String::new(),
            file_picker_scroll_position: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UiState {
    pub theme: SharedTheme,
    pub toast_dismiss_ms: u32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: SharedTheme::Light,
            toast_dismiss_ms: 5000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DialogsData {
    pub show_file_dialog: bool,
}

impl Default for DialogsData {
    fn default() -> Self {
        Self {
            show_file_dialog: false,
        }
    }
}

// === MAIN CONFIG DOMAIN ===

/// Clean Actor+Relay domain for application configuration
/// Replaces the 1,221-line monstrosity with proper architecture
#[derive(Clone)]
pub struct AppConfig {
    // === ACTOR STATE ===
    pub theme_actor: Actor<SharedTheme>,
    pub dock_mode_actor: Actor<DockMode>, 
    pub panel_dimensions_right_actor: Actor<PanelDimensions>,
    pub panel_dimensions_bottom_actor: Actor<PanelDimensions>,
    pub timeline_state_actor: Actor<TimelineState>,
    pub session_state_actor: Actor<SessionState>,
    pub ui_state_actor: Actor<UiState>,
    pub toast_dismiss_ms_actor: Actor<u32>,
    pub dialogs_data_actor: Actor<DialogsData>,
    pub is_loaded_actor: Actor<bool>,
    
    // Keep ConfigSaver alive in struct
    config_saver: ConfigSaver,
    
    // === EVENT RELAYS ===
    pub theme_loaded_relay: Relay<SharedTheme>,
    pub theme_button_clicked_relay: Relay,
    pub dock_mode_loaded_relay: Relay<DockMode>,
    pub dock_mode_button_clicked_relay: Relay,
    pub panel_dimensions_right_changed_relay: Relay<PanelDimensions>,
    pub panel_dimensions_bottom_changed_relay: Relay<PanelDimensions>,
    pub panel_resized_relay: Relay<PanelDimensions>,
    pub timeline_state_changed_relay: Relay<TimelineState>,
    pub cursor_moved_relay: Relay<TimeNs>,
    pub zoom_changed_relay: Relay<f64>,
    pub session_state_changed_relay: Relay<SessionState>,
    pub ui_state_changed_relay: Relay<UiState>,
    pub toast_dismiss_ms_changed_relay: Relay<u32>,
    pub dialogs_data_changed_relay: Relay<DialogsData>,
    pub config_loaded_relay: Relay<SharedAppConfig>,
}

impl AppConfig {
    /// Create new config domain with Actor+Relay architecture  
    pub fn new() -> Self {
        // Create relays for all events
        let (theme_loaded_relay, theme_loaded_stream) = relay::<SharedTheme>();
        let (theme_button_clicked_relay, theme_button_clicked_stream) = relay();
        let (dock_mode_loaded_relay, dock_mode_loaded_stream) = relay::<DockMode>();
        let (dock_mode_button_clicked_relay, dock_mode_button_clicked_stream) = relay();
        let (panel_dimensions_right_changed_relay, panel_dimensions_right_changed_stream) = relay();
        let (panel_dimensions_bottom_changed_relay, panel_dimensions_bottom_changed_stream) = relay();
        let (panel_resized_relay, panel_resized_stream) = relay();
        let (timeline_state_changed_relay, timeline_state_changed_stream) = relay();
        let (cursor_moved_relay, cursor_moved_stream) = relay();
        let (zoom_changed_relay, zoom_changed_stream) = relay();
        let (session_state_changed_relay, session_state_changed_stream) = relay();
        let (ui_state_changed_relay, ui_state_changed_stream) = relay();
        let (toast_dismiss_ms_changed_relay, toast_dismiss_ms_changed_stream) = relay();
        let (dialogs_data_changed_relay, dialogs_data_changed_stream) = relay();
        let (config_loaded_relay, config_loaded_stream) = relay();

        // Create theme actor
        let theme_actor = Actor::new(SharedTheme::default(), async move |state| {
            let mut theme_stream = theme_loaded_stream.fuse();
            let mut theme_button_clicked_stream = theme_button_clicked_stream.fuse();
            
            loop {
                select! {
                    new_theme = theme_stream.next() => {
                        if let Some(new_theme) = new_theme {
                            state.set_neq(new_theme);
                        }
                    }
                    button_click = theme_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            zoon::println!("🎨 Theme Actor: Processing button click");
                            // ✅ Read and modify state directly
                            {
                                let mut theme = state.lock_mut();
                                let old_theme = *theme;
                                *theme = match *theme {
                                    SharedTheme::Light => SharedTheme::Dark,
                                    SharedTheme::Dark => SharedTheme::Light,
                                };
                                zoon::println!("🎨 Theme Actor: Toggling from {:?} to {:?}", old_theme, *theme);
                            }
                        }
                    }
                }
            }
        });

        // Create dock mode actor  
        let dock_mode_actor = Actor::new(DockMode::default(), async move |state| {
            let mut dock_mode_stream = dock_mode_loaded_stream.fuse();
            let mut dock_mode_button_clicked_stream = dock_mode_button_clicked_stream.fuse();
            
            loop {
                select! {
                    new_mode = dock_mode_stream.next() => {
                        if let Some(new_mode) = new_mode {
                            state.set_neq(new_mode);
                        }
                    }
                    button_click = dock_mode_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            zoon::println!("🚢 Dock Actor: Processing button click");
                            // ✅ Read and modify state directly
                            {
                                let mut dock_mode = state.lock_mut();
                                let old_mode = *dock_mode;
                                *dock_mode = match *dock_mode {
                                    DockMode::Right => DockMode::Bottom,
                                    DockMode::Bottom => DockMode::Right,
                                };
                                zoon::println!("🚢 Dock Actor: Toggling from {:?} to {:?}", old_mode, *dock_mode);
                            }
                        }
                    }
                }
            }
        });

        // Create panel dimensions actors
        let panel_dimensions_right_actor = Actor::new(PanelDimensions::default(), async move |state| {
            let mut right_stream = panel_dimensions_right_changed_stream.fuse();
            let mut resized_stream = panel_resized_stream.fuse();
            
            loop {
                select! {
                    new_dims = right_stream.next() => {
                        if let Some(dims) = new_dims {
                            state.set_neq(dims);
                        }
                    }
                    resized_dims = resized_stream.next() => {
                        if let Some(dims) = resized_dims {
                            // Handle panel resize events
                            state.set_neq(dims);
                        }
                    }
                }
            }
        });

        let panel_dimensions_bottom_actor = Actor::new(PanelDimensions::default(), async move |state| {
            let mut bottom_stream = panel_dimensions_bottom_changed_stream;
            while let Some(new_dims) = bottom_stream.next().await {
                state.set_neq(new_dims);
            }
        });

        // Create timeline state actor  
        let timeline_state_actor = Actor::new(TimelineState::default(), async move |state| {
            let mut timeline_stream = timeline_state_changed_stream.fuse();
            let mut cursor_stream = cursor_moved_stream.fuse();
            let mut zoom_stream = zoom_changed_stream.fuse();
            
            loop {
                select! {
                    new_state = timeline_stream.next() => {
                        if let Some(state_update) = new_state {
                            state.set_neq(state_update);
                        }
                    }
                    cursor_pos = cursor_stream.next() => {
                        if let Some(pos) = cursor_pos {
                            state.update_mut(|current| current.cursor_position = pos);
                        }
                    }
                    zoom_level = zoom_stream.next() => {
                        if let Some(level) = zoom_level {
                            state.update_mut(|current| current.zoom_level = level);
                        }
                    }
                }
            }
        });

        // Create session state actor
        let session_state_actor = Actor::new(SessionState::default(), async move |state| {
            let mut session_stream = session_state_changed_stream;
            while let Some(new_session) = session_stream.next().await {
                state.set_neq(new_session);
            }
        });

        // Create UI state actor
        let ui_state_actor = Actor::new(UiState::default(), async move |state| {
            let mut ui_stream = ui_state_changed_stream;
            while let Some(new_ui) = ui_stream.next().await {
                state.set_neq(new_ui);
            }
        });

        // Create toast dismiss ms actor
        let toast_dismiss_ms_actor = Actor::new(5000u32, async move |state| {
            let mut toast_stream = toast_dismiss_ms_changed_stream;
            while let Some(new_ms) = toast_stream.next().await {
                state.set_neq(new_ms);
            }
        });

        // Create dialogs data actor
        let dialogs_data_actor = Actor::new(DialogsData::default(), async move |state| {
            let mut dialogs_stream = dialogs_data_changed_stream;
            while let Some(new_dialogs) = dialogs_stream.next().await {
                state.set_neq(new_dialogs);
            }
        });

        // Create is_loaded actor
        let is_loaded_actor = Actor::new(false, async move |state| {
            let mut config_loaded_stream = config_loaded_stream;
            while let Some(_config) = config_loaded_stream.next().await {
                state.set_neq(true);
            }
        });

        // Create automatic config saver that watches all config changes
        zoon::println!("🔧 AppConfig: About to create ConfigSaver...");
        let config_saver = ConfigSaver::new(
            theme_actor.clone(),
            dock_mode_actor.clone(), 
            panel_dimensions_right_actor.clone(),
            panel_dimensions_bottom_actor.clone(),
            timeline_state_actor.clone(),
            session_state_actor.clone(),
            ui_state_actor.clone(),
            dialogs_data_actor.clone(),
        );
        zoon::println!("✅ AppConfig: ConfigSaver created successfully");
        
        // Keep ConfigSaver alive by storing it without underscore prefix
        // The variable name without underscore prevents Rust from dropping it immediately

        Self {
            theme_actor,
            dock_mode_actor,
            panel_dimensions_right_actor,
            panel_dimensions_bottom_actor,
            timeline_state_actor,
            session_state_actor,
            ui_state_actor,
            toast_dismiss_ms_actor,
            dialogs_data_actor,
            is_loaded_actor,
            
            config_saver,
            
            theme_loaded_relay,
            theme_button_clicked_relay,
            dock_mode_loaded_relay,
            dock_mode_button_clicked_relay,
            panel_dimensions_right_changed_relay,
            panel_dimensions_bottom_changed_relay,
            panel_resized_relay,
            timeline_state_changed_relay,
            cursor_moved_relay,
            zoom_changed_relay,
            session_state_changed_relay,
            ui_state_changed_relay,
            toast_dismiss_ms_changed_relay,
            dialogs_data_changed_relay,
            config_loaded_relay,
        }
    }
}

// === GLOBAL INSTANCE ===

static APP_CONFIG: Lazy<AppConfig> = Lazy::new(AppConfig::new);

/// Get the global config domain
pub fn app_config() -> &'static AppConfig {
    &APP_CONFIG
}






/// Combined workspace section signal
pub fn workspace_section_signal() -> impl Signal<Item = shared::WorkspaceSection> {
    map_ref! {
        let dock_mode = app_config().dock_mode_actor.signal(),
        let right_dims = app_config().panel_dimensions_right_actor.signal(),
        let bottom_dims = app_config().panel_dimensions_bottom_actor.signal(),
        let timeline = app_config().timeline_state_actor.signal() =>
        shared::WorkspaceSection {
            opened_files: Vec::new(), // TODO: get from opened_files domain when implemented
            docked_bottom_dimensions: shared::DockedBottomDimensions {
                files_and_scopes_panel_width: bottom_dims.files_panel_width as f64,
                files_and_scopes_panel_height: bottom_dims.files_panel_height as f64,
                selected_variables_panel_name_column_width: Some(bottom_dims.variables_name_column_width as f64),
                selected_variables_panel_value_column_width: Some(bottom_dims.variables_value_column_width as f64),
            },
            docked_right_dimensions: shared::DockedRightDimensions {
                files_and_scopes_panel_width: right_dims.files_panel_width as f64,
                files_and_scopes_panel_height: right_dims.files_panel_height as f64,
                selected_variables_panel_name_column_width: Some(right_dims.variables_name_column_width as f64),
                selected_variables_panel_value_column_width: Some(right_dims.variables_value_column_width as f64),
            },
            dock_mode: *dock_mode,
            expanded_scopes: Vec::new(), // TODO: get from expanded_scopes domain when implemented
            load_files_expanded_directories: Vec::new(),
            selected_scope_id: None,
            load_files_scroll_position: 0,
            variables_search_filter: String::new(),
            selected_variables: Vec::new(),
            timeline_cursor_position_ns: timeline.cursor_position.nanos(),
            timeline_visible_range_start_ns: Some(timeline.visible_range.start.nanos()),
            timeline_visible_range_end_ns: Some(timeline.visible_range.end.nanos()),
            timeline_zoom_level: timeline.zoom_level as f32,
        }
    }
}


// === BACKEND INTEGRATION ===

/// Save configuration to backend using cached values (Actor+Relay compliant)
async fn save_config_with_cached_values(
    theme: SharedTheme,
    dock_mode: DockMode,
    panel_right: PanelDimensions,
    panel_bottom: PanelDimensions,
    timeline: TimelineState,
    session: SessionState,
    ui: UiState,
    dialogs: DialogsData,
) {
    // ✅ Use cached values passed from ConfigSaver Actor loop
    let shared_config = shared::AppConfig {
        app: shared::AppSection::default(),
        workspace: shared::WorkspaceSection {
            opened_files: Vec::new(),
            docked_bottom_dimensions: shared::DockedBottomDimensions {
                files_and_scopes_panel_width: panel_bottom.files_panel_width as f64,
                files_and_scopes_panel_height: panel_bottom.files_panel_height as f64,
                selected_variables_panel_name_column_width: Some(panel_bottom.variables_name_column_width as f64),
                selected_variables_panel_value_column_width: Some(panel_bottom.variables_value_column_width as f64),
            },
            docked_right_dimensions: shared::DockedRightDimensions {
                files_and_scopes_panel_width: panel_right.files_panel_width as f64,
                files_and_scopes_panel_height: panel_right.files_panel_height as f64,
                selected_variables_panel_name_column_width: Some(panel_right.variables_name_column_width as f64),
                selected_variables_panel_value_column_width: Some(panel_right.variables_value_column_width as f64),
            },
            dock_mode, // ✅ From cached value
            expanded_scopes: Vec::new(),
            load_files_expanded_directories: Vec::new(),
            selected_scope_id: None,
            load_files_scroll_position: session.file_picker_scroll_position,
            variables_search_filter: String::new(),
            selected_variables: Vec::new(),
            timeline_cursor_position_ns: timeline.cursor_position.nanos(),
            timeline_visible_range_start_ns: Some(timeline.visible_range.start.nanos()),
            timeline_visible_range_end_ns: Some(timeline.visible_range.end.nanos()),
            timeline_zoom_level: timeline.zoom_level as f32,
        },
        ui: shared::UiSection {
            theme, // ✅ From cached value
            toast_dismiss_ms: 3000, // Use ui state or default
        },
    };
    
    let _ = CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config)).await;
}


// === CONFIG APPLICATION FUNCTIONS ===

/// Apply configuration from backend (main entry point)
pub fn apply_config(config: shared::AppConfig) {
    // Clone config for sending to config_loaded at the end
    let config_copy = config.clone();
    
    // Update all domain actors with the loaded config
    let domain = app_config();
    
    // Update theme
    domain.theme_loaded_relay.send(config.ui.theme.clone());
    
    // Update dock mode  
    domain.dock_mode_loaded_relay.send(config.workspace.dock_mode);
    
    // Update panel dimensions
    let right_dims = PanelDimensions {
        files_panel_width: config.workspace.docked_right_dimensions.files_and_scopes_panel_width as f32,
        files_panel_height: config.workspace.docked_right_dimensions.files_and_scopes_panel_height as f32,
        variables_panel_width: 300.0,
        timeline_panel_height: 200.0,
        variables_name_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_name_column_width.unwrap_or(180.0) as f32,
        variables_value_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_value_column_width.unwrap_or(100.0) as f32,
    };
    domain.panel_dimensions_right_changed_relay.send(right_dims);
    
    let bottom_dims = PanelDimensions {
        files_panel_width: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_width as f32,
        files_panel_height: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_height as f32,
        variables_panel_width: 300.0,
        timeline_panel_height: 200.0,
        variables_name_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_name_column_width.unwrap_or(180.0) as f32,
        variables_value_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_value_column_width.unwrap_or(100.0) as f32,
    };
    domain.panel_dimensions_bottom_changed_relay.send(bottom_dims);
    
    // Update timeline state
    let timeline_state = TimelineState {
        cursor_position: TimeNs::from_nanos(config.workspace.timeline_cursor_position_ns),
        visible_range: TimeRange {
            start: TimeNs::from_nanos(config.workspace.timeline_visible_range_start_ns.unwrap_or(0)),
            end: TimeNs::from_nanos(config.workspace.timeline_visible_range_end_ns.unwrap_or(100_000_000_000)),
        },
        zoom_level: config.workspace.timeline_zoom_level as f64,
    };
    domain.timeline_state_changed_relay.send(timeline_state);
    
    // Update session state (use workspace data since session is not in shared::AppConfig)
    let session_state = SessionState {
        opened_files: config.workspace.opened_files,
        variables_search_filter: config.workspace.variables_search_filter,
        file_picker_scroll_position: config.workspace.load_files_scroll_position,
    };
    domain.session_state_changed_relay.send(session_state);
    
    // Update UI state
    let ui_state = UiState {
        theme: config.ui.theme,
        toast_dismiss_ms: config.ui.toast_dismiss_ms as u32,
    };
    domain.ui_state_changed_relay.send(ui_state);
    domain.toast_dismiss_ms_changed_relay.send(config.ui.toast_dismiss_ms as u32);
    
    // Update dialogs state (default since not in shared::AppConfig)
    let dialogs_data = DialogsData {
        show_file_dialog: false,
    };
    domain.dialogs_data_changed_relay.send(dialogs_data);
    
    // Signal that config is loaded (triggers UI rendering)
    domain.config_loaded_relay.send(config_copy);
}

/// Load initial configuration from backend
pub async fn load_config() {
    let _ = CurrentPlatform::send_message(UpMsg::LoadConfig).await;
}





