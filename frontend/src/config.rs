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
use moonzoon_novyui::tokens::theme;

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
        let debounce_task = Arc::new(std::sync::Mutex::new(None::<TaskHandle>));
        // ConfigSaver: Watching all config signals for automatic persistence
        
        // Combine all config signals - trigger save when ANY change
        let config_change_signal = map_ref! {
            let theme = theme_actor.signal(),
            let dock_mode = dock_mode_actor.signal(), 
            let panel_right = panel_right_actor.signal(),
            let panel_bottom = panel_bottom_actor.signal(),
            let timeline = timeline_actor.signal(),
            let session = session_actor.signal(),
            let ui = ui_actor.signal(),
            let dialogs = dialogs_actor.signal(),
            let expanded_scopes = crate::state::EXPANDED_SCOPES_FOR_CONFIG.signal_cloned(),
            let selected_scope_id = crate::state::SELECTED_SCOPE_ID_FOR_CONFIG.signal_cloned() =>
            (theme.clone(), dock_mode.clone(), panel_right.clone(), panel_bottom.clone(), 
             timeline.clone(), session.clone(), ui.clone(), dialogs.clone(), expanded_scopes.clone(), selected_scope_id.clone())
        };
        
        config_change_signal.to_stream().skip(1).for_each({
            let debounce_task = debounce_task.clone();
            move |(theme, dock_mode, panel_right, panel_bottom, timeline, session, ui, _dialogs, _expanded_scopes, _selected_scope_id)| {
                let debounce_task = debounce_task.clone();
                async move {
                    // Cancel any pending save
                    *debounce_task.lock().unwrap() = None;
            
            // Schedule new save with 1 second debounce
            let handle = Task::start_droppable(async move {
                Timer::sleep(1000).await;
                // ConfigSaver: Executing debounced save
                
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
                        dock_mode: dock_mode.clone(),
                        expanded_scopes: crate::state::EXPANDED_SCOPES_FOR_CONFIG.get_cloned(),
                        load_files_expanded_directories: session.file_picker_expanded_directories,
                        selected_scope_id: crate::state::SELECTED_SCOPE_ID_FOR_CONFIG.get_cloned(),
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
            
            *debounce_task.lock().unwrap() = Some(handle);
                }
            }
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
            variables_name_column_width: 190.0,
            variables_value_column_width: 220.0,
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
    pub file_picker_expanded_directories: Vec<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            variables_search_filter: String::new(),
            file_picker_scroll_position: 0,
            file_picker_expanded_directories: Vec::new(),
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
    
    // === UI MUTABLES FOR DIRECT TREEVIEW CONNECTION ===
    pub file_picker_expanded_directories: Mutable<indexmap::IndexSet<String>>,
    pub file_picker_scroll_position: Mutable<i32>,
    
    // Keep config saver actor alive
    _config_saver_actor: Actor<()>,
    
    // === EVENT RELAYS ===
    pub theme_button_clicked_relay: Relay,
    pub dock_mode_button_clicked_relay: Relay,
    pub variables_filter_changed_relay: Relay<String>,
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
        // Use unified platform abstraction for request-response pattern
        crate::platform::CurrentPlatform::request_response(UpMsg::LoadConfig).await
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
        
        
        // Create relays for all events
        let (theme_button_clicked_relay, theme_button_clicked_stream) = relay();
        let (dock_mode_button_clicked_relay, dock_mode_button_clicked_stream) = relay();
        let (variables_filter_changed_relay, variables_filter_changed_stream) = relay();
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
        let (config_loaded_relay, config_loaded_stream) = relay::<SharedAppConfig>();

        // Clone relays for use in multiple Actors to avoid move issues
        let panel_dimensions_right_changed_relay_clone = panel_dimensions_right_changed_relay.clone();
        let panel_dimensions_bottom_changed_relay_clone = panel_dimensions_bottom_changed_relay.clone();
        let panel_resized_relay_clone = panel_resized_relay.clone();

        // Create theme actor with loaded config value
        let theme_actor = Actor::new(config.ui.theme, async move |state| {
            let mut theme_button_clicked_stream = theme_button_clicked_stream.fuse();
            
            // Initialize NovyUI theme system with current theme
            let initial_novyui_theme = match config.ui.theme {
                SharedTheme::Light => theme::Theme::Light,
                SharedTheme::Dark => theme::Theme::Dark,
            };
            theme::init_theme(Some(initial_novyui_theme), None);
            // Theme Actor: Initialized NovyUI theme system
            
            loop {
                select! {
                    button_click = theme_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            // Theme Actor: Processing button click
                            // ✅ Read and modify state directly
                            {
                                let mut theme = state.lock_mut();
                                let old_theme = *theme;
                                *theme = match *theme {
                                    SharedTheme::Light => SharedTheme::Dark,
                                    SharedTheme::Dark => SharedTheme::Light,
                                };
                                // Theme Actor: Toggling theme
                                
                                // Update NovyUI theme system immediately
                                let novyui_theme = match *theme {
                                    SharedTheme::Light => theme::Theme::Light,
                                    SharedTheme::Dark => theme::Theme::Dark,
                                };
                                theme::set_theme(novyui_theme);
                                zoon::println!("🎨 Theme Actor: Updated NovyUI theme to {:?}", novyui_theme);
                            }
                        }
                    }
                }
            }
        });

        // Create dock mode actor with loaded config value
        let dock_mode_actor = Actor::new(
            config.workspace.dock_mode.clone(), 
            {
                let panel_dimensions_right_changed_relay = panel_dimensions_right_changed_relay_clone.clone();
                let panel_dimensions_bottom_changed_relay = panel_dimensions_bottom_changed_relay_clone.clone();
                async move |state| {
            let mut dock_mode_button_clicked_stream = dock_mode_button_clicked_stream.fuse();
            
            loop {
                select! {
                    button_click = dock_mode_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            zoon::println!("🚢 Dock Actor: Processing button click - implementing proper dimension preservation");
                            
                            // Get current panel dimensions from DRAGGING SYSTEM BEFORE switching mode
                            let current_files_height = crate::dragging::files_panel_height_signal().to_stream().next().await.unwrap_or(300.0) as u32;
                            let current_name_width = crate::dragging::variables_name_column_width_signal().to_stream().next().await.unwrap_or(190.0) as u32;
                            let current_value_width = crate::dragging::variables_value_column_width_signal().to_stream().next().await.unwrap_or(220.0) as u32;
                            
                            zoon::println!("💾 DOCK SWITCH: Current dimensions before switch: height={}, name={}, value={}", 
                                current_files_height, current_name_width, current_value_width);
                            
                            // ✅ Read and modify dock mode
                            let (old_mode, new_mode) = {
                                let mut dock_mode = state.lock_mut();
                                let old_mode = *dock_mode;
                                *dock_mode = match *dock_mode {
                                    DockMode::Right => DockMode::Bottom,
                                    DockMode::Bottom => DockMode::Right,
                                };
                                let new_mode = *dock_mode;
                                (old_mode, new_mode)
                            };
                            
                            zoon::println!("🚢 Dock Actor: Switching from {:?} to {:?}", old_mode, new_mode);
                            
                            // 📁 CRITICAL: Save current mode's dimensions before switching
                            // ✅ FIX: Don't overwrite existing config values - only save current Actor values for ACTIVE dimensions
                            match old_mode {
                                DockMode::Right => {
                                    // Update Right dock dimensions - keep existing values, only update what's currently active  
                                    let current_dims = crate::config::app_config().panel_dimensions_right_actor.signal().to_stream().next().await.unwrap();
                                    let updated_dims = PanelDimensions {
                                        files_panel_width: current_dims.files_panel_width, // Keep existing
                                        files_panel_height: current_dims.files_panel_height, // Keep existing - don't overwrite with shared Actor
                                        variables_panel_width: current_dims.variables_panel_width, // Keep existing
                                        timeline_panel_height: current_dims.timeline_panel_height, // Keep existing
                                        variables_name_column_width: current_name_width as f32, // Update from Actor (this is actively used)
                                        variables_value_column_width: current_value_width as f32, // Update from Actor (this is actively used)
                                    };
                                    panel_dimensions_right_changed_relay.send(updated_dims);
                                    zoon::println!("💾 DOCK SWITCH: Preserved Right mode dimensions: height={} (kept), name={}, value={}", 
                                        current_dims.files_panel_height, current_name_width, current_value_width);
                                }
                                DockMode::Bottom => {
                                    // Update Bottom dock dimensions - keep existing values, only update what's currently active
                                    let current_dims = crate::config::app_config().panel_dimensions_bottom_actor.signal().to_stream().next().await.unwrap();
                                    let updated_dims = PanelDimensions {
                                        files_panel_width: current_dims.files_panel_width, // Keep existing 
                                        files_panel_height: current_dims.files_panel_height, // Keep existing - don't overwrite with shared Actor
                                        variables_panel_width: current_dims.variables_panel_width, // Keep existing
                                        timeline_panel_height: current_dims.timeline_panel_height, // Keep existing
                                        variables_name_column_width: current_name_width as f32, // Update from Actor (this is actively used)
                                        variables_value_column_width: current_value_width as f32, // Update from Actor (this is actively used)
                                    };
                                    panel_dimensions_bottom_changed_relay.send(updated_dims);
                                    zoon::println!("💾 DOCK SWITCH: Preserved Bottom mode dimensions: height={} (kept), name={}, value={}", 
                                        current_dims.files_panel_height, current_name_width, current_value_width);
                                }
                            }
                            
                            // 📂 CRITICAL: Load new mode's saved dimensions to Actors
                            Task::start({
                                let new_mode = new_mode;
                                async move {
                                    Timer::sleep(50).await; // Small delay to ensure config actors are updated
                                    
                                    match new_mode {
                                        DockMode::Right => {
                                            // Load Right dock dimensions and update Actors
                                            let right_config = crate::config::app_config().panel_dimensions_right_actor.signal().to_stream().next().await;
                                            if let Some(dims) = right_config {
                                                zoon::println!("📂 DOCK SWITCH: Loading Right mode dimensions: height={}, name={}, value={}", 
                                                    dims.files_panel_height, dims.variables_name_column_width, dims.variables_value_column_width);
                                                
                                                // Right mode dimensions are already loaded into the config actors - no need to force sync
                                            }
                                        }
                                        DockMode::Bottom => {
                                            // Load Bottom dock dimensions and update Actors
                                            let bottom_config = crate::config::app_config().panel_dimensions_bottom_actor.signal().to_stream().next().await;
                                            if let Some(dims) = bottom_config {
                                                zoon::println!("📂 DOCK SWITCH: Loading Bottom mode dimensions: height={}, name={}, value={}", 
                                                    dims.files_panel_height, dims.variables_name_column_width, dims.variables_value_column_width);
                                                
                                                // Bottom mode dimensions are already loaded into the config actors - no need to force sync
                                            }
                                        }
                                    }
                                }
                            });
                            
                            zoon::println!("✅ DOCK SWITCH: Complete - dimensions preserved independently for both modes");
                        }
                    }
                }
            }
            }
        });

        // Create panel dimensions actors with loaded config values        
        let panel_dimensions_right_actor = Actor::new(PanelDimensions {
            files_panel_width: config.workspace.docked_right_dimensions.files_and_scopes_panel_width as f32,
            files_panel_height: config.workspace.docked_right_dimensions.files_and_scopes_panel_height as f32,
            variables_panel_width: 300.0, // Default values for missing fields
            timeline_panel_height: 200.0,
            variables_name_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_name_column_width.unwrap_or(190.0) as f32,
            variables_value_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_value_column_width.unwrap_or(220.0) as f32,
        }, async move |state| {
            let mut right_stream = panel_dimensions_right_changed_relay_clone.subscribe().fuse();
            let mut resized_stream = panel_resized_relay_clone.subscribe().fuse();
            
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

        let panel_dimensions_bottom_actor = Actor::new(PanelDimensions {
            files_panel_width: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_width as f32,
            files_panel_height: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_height as f32,
            variables_panel_width: 300.0, // Default values for missing fields
            timeline_panel_height: 200.0,
            variables_name_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_name_column_width.unwrap_or(190.0) as f32,
            variables_value_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_value_column_width.unwrap_or(220.0) as f32,
        }, async move |state| {
            let mut bottom_stream = panel_dimensions_bottom_changed_relay_clone.subscribe().fuse();
            
            loop {
                select! {
                    new_dims = bottom_stream.next() => {
                        if let Some(dims) = new_dims {
                            state.set_neq(dims);
                        }
                    }
                }
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

        // Create session state actor with loaded config values
        let session_state_actor = Actor::new(SessionState {
            opened_files: config.workspace.opened_files,
            variables_search_filter: config.workspace.variables_search_filter,
            file_picker_scroll_position: config.workspace.load_files_scroll_position,
            file_picker_expanded_directories: config.workspace.load_files_expanded_directories.clone(),
        }, async move |state| {
            let mut session_stream = session_state_changed_stream.fuse();
            let mut variables_filter_stream = variables_filter_changed_stream.fuse();
            
            loop {
                select! {
                    session_change = session_stream.next() => {
                        if let Some(new_session) = session_change {
                            state.set_neq(new_session);
                        }
                    }
                    filter_change = variables_filter_stream.next() => {
                        if let Some(new_filter) = filter_change {
                            // Update just the variables_search_filter field
                            {
                                let mut session = state.lock_mut();
                                session.variables_search_filter = new_filter;
                            }
                        }
                    }
                }
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
        // AppConfig: Creating config saver actor
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
        // AppConfig: Config saver actor created successfully

        // Create file picker expanded directories mutable with loaded config
        let file_picker_expanded_directories = {
            let mut expanded_set = indexmap::IndexSet::new();
            for dir in &config.workspace.load_files_expanded_directories {
                expanded_set.insert(dir.clone());
            }
            Mutable::new(expanded_set)
        };

        // Load expanded scopes from config into EXPANDED_SCOPES
        {
            // Config: Loading expanded scopes from config
            // Config: Found expanded scopes in config
            
            let mut expanded_scopes = crate::state::EXPANDED_SCOPES.lock_mut();
            expanded_scopes.clear();
            for scope in &config.workspace.expanded_scopes {
                // Distinguish between file-level and scope-level expansion
                let scope_id = if scope.is_empty() {
                    zoon::println!("⚠️ Config: Skipping empty scope ID");
                    continue; // Skip empty scope IDs
                } else if scope.contains('|') {
                    // Nested scope - add "scope_" prefix  
                    let prefixed = format!("scope_{}", scope);
                    // Config: Loading nested scope with prefix
                    prefixed
                } else {
                    // File-level expansion - use path directly (no prefix)
                    // Config: Loading file-level expansion
                    scope.clone()
                };
                expanded_scopes.insert(scope_id);
            }
            // Config: Loaded expanded scopes from config into EXPANDED_SCOPES
            // Config: Final EXPANDED_SCOPES contents loaded
        }

        // Load selected scope ID from config into SELECTED_SCOPE_ID
        if let Some(selected_scope) = &config.workspace.selected_scope_id {
            // Config: Loading selected scope from config
            
            // Apply same prefix logic as expanded_scopes for consistency
            let scope_id = if selected_scope.contains('|') {
                // Nested scope - add "scope_" prefix
                let prefixed = format!("scope_{}", selected_scope);
                // Config: Loading selected nested scope with prefix
                prefixed
            } else {
                // File-level selection - use path directly (no prefix)
                // Config: Loading selected file-level scope
                selected_scope.clone()
            };
            
            crate::state::SELECTED_SCOPE_ID.set_neq(Some(scope_id.clone()));
            // Config: Loaded selected scope ID into SELECTED_SCOPE_ID
        } else {
            // Config: No selected scope ID in config, leaving SELECTED_SCOPE_ID as None
        }

        // Create file picker scroll position mutable with loaded config
        let file_picker_scroll_position = Mutable::new(config.workspace.load_files_scroll_position);

        // Set up sync from mutable changes to session state (for config saving)
        let file_picker_sync = file_picker_expanded_directories.clone();
        let session_sync = session_state_actor.clone();
        let session_changed_relay = session_state_changed_relay.clone();
        Task::start(async move {
            file_picker_sync.signal_cloned().for_each(move |expanded_set| {
                let session_sync = session_sync.clone();
                let session_changed_relay = session_changed_relay.clone();
                async move {
                    // Get current session state and update expanded directories
                    if let Some(mut session_state) = session_sync.signal().to_stream().next().await {
                        session_state.file_picker_expanded_directories = expanded_set.iter().cloned().collect();
                        
                        // Trigger session state change to save config
                        session_changed_relay.send(session_state);
                        // File picker directories synced to session state
                    }
                }
            }).await;
        });

        // Set up sync for scroll position changes to session state
        let scroll_position_sync = file_picker_scroll_position.clone();
        let session_scroll_sync = session_state_actor.clone();
        let session_scroll_changed_relay = session_state_changed_relay.clone();
        Task::start(async move {
            scroll_position_sync.signal().for_each(move |scroll_position| {
                let session_scroll_sync = session_scroll_sync.clone();
                let session_scroll_changed_relay = session_scroll_changed_relay.clone();
                async move {
                    // Get current session state and update scroll position
                    if let Some(mut session_state) = session_scroll_sync.signal().to_stream().next().await {
                        session_state.file_picker_scroll_position = scroll_position;
                        
                        // Trigger session state change to save config
                        session_scroll_changed_relay.send(session_state);
                        // File picker scroll position synced to session state
                    }
                }
            }).await;
        });

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
            
            file_picker_expanded_directories,
            file_picker_scroll_position,
            
            _config_saver_actor: config_saver_actor,
            
            theme_button_clicked_relay,
            dock_mode_button_clicked_relay,
            variables_filter_changed_relay,
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

pub static APP_CONFIG: std::sync::OnceLock<AppConfig> = std::sync::OnceLock::new();


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
        let timeline = app_config().timeline_state_actor.signal(),
        let session = app_config().session_state_actor.signal() =>
        shared::WorkspaceSection {
            opened_files: session.opened_files.clone(),
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
            expanded_scopes: crate::state::EXPANDED_SCOPES_FOR_CONFIG.get_cloned(),
            load_files_expanded_directories: session.file_picker_expanded_directories.clone(),
            selected_scope_id: None,
            load_files_scroll_position: session.file_picker_scroll_position,
            variables_search_filter: session.variables_search_filter.clone(),
            selected_variables: Vec::new(),
            timeline_cursor_position_ns: timeline.cursor_position.nanos(),
            timeline_visible_range_start_ns: Some(timeline.visible_range.start.nanos()),
            timeline_visible_range_end_ns: Some(timeline.visible_range.end.nanos()),
            timeline_zoom_level: timeline.zoom_level as f32,
        }
    }
}


// === BACKEND INTEGRATION ===

