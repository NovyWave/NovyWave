use zoon::*;
use shared::{self, AppConfig as SharedAppConfig, DockMode, Theme as SharedTheme};
use crate::time_types::TimeNs;
use crate::dataflow::{Actor, relay, Relay};
use crate::platform::{Platform, CurrentPlatform};
use futures::{StreamExt, select};
use shared::UpMsg;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::str::FromStr;

// === SHARED TYPES FOR ACTORS ===

/// Config saver actor that watches all config signals and debounces saves
fn create_config_saver_actor(
    theme_actor: Actor<SharedTheme>,
    dock_mode_actor: Actor<DockMode>,
    panel_right_actor: Actor<PanelDimensions>,
    panel_bottom_actor: Actor<PanelDimensions>,
    timeline_actor: Actor<TimelineState>,
    session_actor: Actor<SessionState>,
    ui_actor: Actor<UiState>,
    dialogs_actor: Actor<DialogsData>,
) -> Actor<()> {
    Actor::new((), async move |_state| {
        let mut debounce_task: Option<TaskHandle> = None;
        zoon::println!("💾 ConfigSaver: Watching all config signals...");
        
        // Combine all config signals - trigger save when ANY change
        let config_change_signal = map_ref! {
            let theme = theme_actor.signal(),
            let dock_mode = dock_mode_actor.signal(), 
            let panel_right = panel_right_actor.signal(),
            let panel_bottom = panel_bottom_actor.signal(),
            let timeline = timeline_actor.signal(),
            let session = session_actor.signal(),
            let ui = ui_actor.signal(),
            let dialogs = dialogs_actor.signal() =>
            (theme.clone(), dock_mode.clone(), panel_right.clone(), panel_bottom.clone(), 
             timeline.clone(), session.clone(), ui.clone(), dialogs.clone())
        };
        
        config_change_signal.skip(1).for_each(move |(theme, dock_mode, panel_right, panel_bottom, timeline, session, ui, dialogs)| async move {
            // Cancel any pending save
            debounce_task = None;
            
            // Schedule new save with 1 second debounce
            let handle = Task::start_droppable(async move {
                Timer::sleep(1000).await;
                zoon::println!("💾 ConfigSaver: Executing debounced save");
                
                // Build config from current values
                let shared_config = shared::AppConfig {
                    app: shared::AppSection::default(),
                    workspace: shared::WorkspaceSection {
                        opened_files: session.opened_files,
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
                        dock_mode: dock_mode.to_string(),
                        expanded_scopes: Vec::new(),
                        load_files_expanded_directories: Vec::new(),
                        selected_scope_id: None,
                        load_files_scroll_position: session.file_picker_scroll_position,
                        variables_search_filter: session.variables_search_filter,
                        selected_variables: Vec::new(),
                        timeline_cursor_position_ns: timeline.cursor_position.nanos(),
                        timeline_visible_range_start_ns: Some(timeline.visible_range.start.nanos()),
                        timeline_visible_range_end_ns: Some(timeline.visible_range.end.nanos()),
                        timeline_zoom_level: timeline.zoom_level as f32,
                    },
                    ui: shared::UiSection {
                        theme,
                        toast_dismiss_ms: ui.toast_dismiss_ms as u64,
                    },
                };
                
                if let Err(e) = CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config)).await {
                    zoon::eprintln!("🚨 Failed to save config: {e}");
                }
            });
            
            debounce_task = Some(handle);
        }).await;
    })
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
    
    // Keep config saver actor alive
    _config_saver_actor: Actor<()>,
    
    // === EVENT RELAYS ===
    pub theme_button_clicked_relay: Relay,
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
}

impl AppConfig {
    /// Load configuration from backend using request-response pattern
    async fn load_config_from_backend() -> Result<SharedAppConfig, String> {
        use futures::channel::oneshot;
        use std::sync::{Arc, Mutex};
        
        // Create a oneshot channel for the response
        let (sender, receiver) = oneshot::channel::<SharedAppConfig>();
        let sender = Arc::new(Mutex::new(Some(sender)));
        
        // Create a temporary relay to capture the config response
        let (config_response_relay, mut config_response_stream) = relay::<SharedAppConfig>();
        
        // Set up response handler that will forward the config when received
        let sender_clone = sender.clone();
        let response_task = Task::start(async move {
            if let Some(config) = config_response_stream.next().await {
                if let Some(sender) = sender_clone.lock().unwrap_throw().take() {
                    let _ = sender.send(config);
                }
            }
        });
        
        // Temporarily hook into connection to capture ConfigLoaded response
        // We'll modify the connection handler to emit through our relay
        let original_handler = std::sync::Arc::new(std::sync::Mutex::new(None));
        
        // Send the load config request
        CurrentPlatform::send_message(UpMsg::LoadConfig).await
            .map_err(|e| format!("Failed to send LoadConfig message: {e}"))?;
        
        // Set up temporary config capture by patching the connection handler
        // This is a workaround since we can't easily modify the Connection directly
        CONFIG_LOAD_RESPONSE_RELAY.set(Some(config_response_relay));
        
        // Wait for response with timeout
        let config = match Timer::timeout(5000, receiver).await {
            Some(Ok(config)) => config,
            Some(Err(_)) => {
                CONFIG_LOAD_RESPONSE_RELAY.set(None);
                return Err("Config response channel closed".to_string());
            },
            None => {
                CONFIG_LOAD_RESPONSE_RELAY.set(None);
                return Err("Config load request timed out after 5 seconds".to_string());
            },
        };
        
        // Clean up
        CONFIG_LOAD_RESPONSE_RELAY.set(None);
        response_task.cancel();
        
        Ok(config)
    }
    
    /// Create new config domain with Actor+Relay architecture  
    pub async fn new() -> Self {
        // Load app config from backend using request-response pattern
        let config = Self::load_config_from_backend().await
            .unwrap_or_else(|error| {
                zoon::eprintln!("⚠️ Failed to load config from backend: {error}");
                zoon::println!("🔧 Using default configuration");
                SharedAppConfig::default()
            });
        
        zoon::println!("✅ Config loaded: dock_mode={:?}", config.workspace.dock_mode);
        
        // Create relays for all events
        let (theme_button_clicked_relay, theme_button_clicked_stream) = relay();
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

        // Create theme actor with loaded config value
        let theme_actor = Actor::new(config.ui.theme, async move |state| {
            let mut theme_button_clicked_stream = theme_button_clicked_stream.fuse();
            
            loop {
                select! {
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

        // Create dock mode actor with loaded config value
        let dock_mode_actor = Actor::new(
            DockMode::from_str(&config.workspace.dock_mode).unwrap_or(DockMode::Right), 
            async move |state| {
            let mut dock_mode_button_clicked_stream = dock_mode_button_clicked_stream.fuse();
            
            loop {
                select! {
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

        // Create panel dimensions actors with loaded config values
        let panel_dimensions_right_actor = Actor::new(config.workspace.panel_dimensions_right, async move |state| {
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

        let panel_dimensions_bottom_actor = Actor::new(config.workspace.panel_dimensions_bottom, async move |state| {
            let mut bottom_stream = panel_dimensions_bottom_changed_stream;
            while let Some(new_dims) = bottom_stream.next().await {
                state.set_neq(new_dims);
            }
        });

        // Create timeline state actor (using defaults for now - can be added to config later)
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

        // Create toast dismiss ms actor with loaded config value
        let toast_dismiss_ms_actor = Actor::new(config.ui.toast_dismiss_ms as u32, async move |state| {
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

        // Create automatic config saver actor that watches all config changes
        zoon::println!("🔧 AppConfig: Creating config saver actor...");
        let config_saver_actor = create_config_saver_actor(
            theme_actor.clone(),
            dock_mode_actor.clone(), 
            panel_dimensions_right_actor.clone(),
            panel_dimensions_bottom_actor.clone(),
            timeline_state_actor.clone(),
            session_state_actor.clone(),
            ui_state_actor.clone(),
            dialogs_data_actor.clone(),
        );
        zoon::println!("✅ AppConfig: Config saver actor created successfully");

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
            
            _config_saver_actor: config_saver_actor,
            
            theme_button_clicked_relay,
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
        }
    }
}

// === GLOBAL INSTANCE ===

static APP_CONFIG: std::sync::OnceLock<AppConfig> = std::sync::OnceLock::new();

/// Temporary relay for capturing config load responses during initialization
static CONFIG_LOAD_RESPONSE_RELAY: Lazy<Mutable<Option<Relay<SharedAppConfig>>>> = 
    Lazy::new(|| Mutable::new(None));

/// Initialize the global AppConfig instance
pub async fn init_app_config() -> Result<(), &'static str> {
    let config = AppConfig::new().await;
    APP_CONFIG.set(config).map_err(|_| "AppConfig already initialized")
}

/// Called by connection handler to forward config load responses during initialization
pub fn forward_config_load_response(config: SharedAppConfig) {
    if let Some(relay) = CONFIG_LOAD_RESPONSE_RELAY.get_cloned().flatten() {
        relay.send(config);
    }
}

/// Get the global config domain
pub fn app_config() -> &'static AppConfig {
    APP_CONFIG.get().expect_throw("AppConfig not initialized - call init_app_config() first")
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

