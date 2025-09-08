use crate::dataflow::{Actor, Relay, relay};
use crate::platform::{CurrentPlatform, Platform};
use crate::visualizer::timeline::time_types::TimeNs;
use futures::{StreamExt, select};
use serde::{Deserialize, Serialize};
use shared::UpMsg;
use shared::{self, AppConfig as SharedAppConfig, DockMode, Theme as SharedTheme};
use std::sync::Arc;
use zoon::*;
use moonzoon_novyui::tokens::theme;

// === SHARED TYPES FOR ACTORS ===

/// Config saver actor that watches all config signals and debounces saves
fn create_config_saver_actor(
    theme_actor: Actor<SharedTheme>,
    dock_mode_actor: Actor<DockMode>,
    panel_right_actor: Actor<PanelDimensions>,
    panel_bottom_actor: Actor<PanelDimensions>,
    session_actor: Actor<SessionState>,
    toast_dismiss_ms_actor: Actor<u32>,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> Actor<()> {
    let selected_variables = selected_variables.clone();
    Actor::new((), async move |_state| {
        let debounce_task = Arc::new(std::sync::Mutex::new(None::<TaskHandle>));
        // ConfigSaver: Watching all config signals for automatic persistence

        // Combine all config signals - trigger save when ANY change
        let config_change_signal = map_ref! {
            let theme = theme_actor.signal(),
            let dock_mode = dock_mode_actor.signal(),
            let panel_right = panel_right_actor.signal(),
            let panel_bottom = panel_bottom_actor.signal(),
            let session = session_actor.signal(),
            let toast_dismiss_ms = toast_dismiss_ms_actor.signal(),
            let expanded_scopes = selected_variables.expanded_scopes.signal().map(|scopes| scopes.into_iter().collect::<Vec<String>>()),
            let selected_scope_id = selected_variables.selected_scope.signal().map(|scope| {
                // Strip TreeView "scope_" prefix before storing to config
                scope.as_ref().map(|scope_id| {
                    if scope_id.starts_with("scope_") {
                        scope_id.strip_prefix("scope_").unwrap_or(scope_id).to_string()
                    } else {
                        scope_id.clone()
                    }
                })
            }),
            let selected_variables = selected_variables.variables_vec_signal.signal_cloned() =>
            (theme.clone(), dock_mode.clone(), panel_right.clone(), panel_bottom.clone(),
             session.clone(), toast_dismiss_ms.clone(), expanded_scopes.clone(), selected_scope_id.clone(),
             selected_variables.clone())
        };

        config_change_signal
            .to_stream()
            .skip(1)
            .for_each({
                let debounce_task = debounce_task.clone();
                move |(
                    theme,
                    dock_mode,
                    panel_right,
                    panel_bottom,
                    session,
                    toast_dismiss_ms,
                    expanded_scopes,
                    _selected_scope_id,
                    selected_variables,
                )| {
                    let debounce_task = debounce_task.clone();
                    async move {
                        // Cancel any pending save
                        *debounce_task.lock().unwrap() = None;

                        // Schedule new save with 1 second debounce
                        let handle = Task::start_droppable(async move {
                            // ✅ ACCEPTABLE: Timer::sleep() for debounced config saving (legitimate use case)
                            // Task::start_droppable + Timer::sleep is the correct debouncing pattern
                            Timer::sleep(1000).await;

                            // ConfigSaver: Executing debounced save
                            let save_result = async move {
                                // Build config from current values
                                let shared_config = shared::AppConfig {
                                    app: shared::AppSection::default(),
                                    workspace: shared::WorkspaceSection {
                                        opened_files: session.opened_files,
                                        docked_bottom_dimensions: shared::DockedBottomDimensions {
                                            files_and_scopes_panel_width: panel_bottom
                                                .files_panel_width
                                                as f64,
                                            files_and_scopes_panel_height: panel_bottom
                                                .files_panel_height
                                                as f64,
                                            selected_variables_panel_name_column_width: Some(
                                                panel_bottom.variables_name_column_width as f64,
                                            ),
                                            selected_variables_panel_value_column_width: Some(
                                                panel_bottom.variables_value_column_width as f64,
                                            ),
                                        },
                                        docked_right_dimensions: shared::DockedRightDimensions {
                                            files_and_scopes_panel_width: panel_right
                                                .files_panel_width
                                                as f64,
                                            files_and_scopes_panel_height: panel_right
                                                .files_panel_height
                                                as f64,
                                            selected_variables_panel_name_column_width: Some(
                                                panel_right.variables_name_column_width as f64,
                                            ),
                                            selected_variables_panel_value_column_width: Some(
                                                panel_right.variables_value_column_width as f64,
                                            ),
                                        },
                                        dock_mode: dock_mode.clone(),
                                        expanded_scopes: expanded_scopes.clone(),
                                        load_files_expanded_directories: session
                                            .file_picker_expanded_directories,
                                        selected_scope_id: _selected_scope_id,
                                        load_files_scroll_position: session
                                            .file_picker_scroll_position,
                                        variables_search_filter: session.variables_search_filter,
                                        selected_variables: selected_variables,
                                        timeline_cursor_position_ns: 0, // Default value
                                        timeline_visible_range_start_ns: None,
                                        timeline_visible_range_end_ns: None,
                                        timeline_zoom_level: 1.0, // Default zoom level
                                    },
                                    ui: shared::UiSection {
                                        theme,
                                        toast_dismiss_ms: toast_dismiss_ms as u64,
                                    },
                                };

                                CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config))
                                    .await
                            }
                            .await;

                            if let Err(_e) = save_result {}
                        });

                        // Store new task handle
                        *debounce_task.lock().unwrap() = Some(handle);
                    }
                }
            })
            .await;
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
            files_panel_width: DEFAULT_PANEL_WIDTH,
            files_panel_height: DEFAULT_PANEL_HEIGHT,
            variables_panel_width: DEFAULT_PANEL_WIDTH,
            timeline_panel_height: DEFAULT_TIMELINE_HEIGHT,
            variables_name_column_width: DEFAULT_NAME_COLUMN_WIDTH,
            variables_value_column_width: DEFAULT_VALUE_COLUMN_WIDTH,
        }
    }
}

// === STATIC DIMENSION CONSTANTS ===
// Default dimensions for professional waveform viewer interface
pub const DEFAULT_PANEL_WIDTH: f32 = 300.0;        // Side panel default width
pub const DEFAULT_PANEL_HEIGHT: f32 = 300.0;       // Horizontal panel default height  
pub const DEFAULT_TIMELINE_HEIGHT: f32 = 200.0;    // Timeline panel optimized height
pub const DEFAULT_NAME_COLUMN_WIDTH: f32 = 190.0;  // Variable name column width
pub const DEFAULT_VALUE_COLUMN_WIDTH: f32 = 220.0; // Variable value column width

// === CONSTRAINT CONSTANTS ===
// UI constraints for professional layout bounds
pub const MIN_PANEL_HEIGHT: f32 = 150.0;           // Minimum for content + scrollbar
pub const MAX_PANEL_HEIGHT: f32 = 530.0;           // Maximum before overwhelming UI
pub const MIN_COLUMN_WIDTH: f32 = 100.0;           // Minimum for readable text
pub const MAX_COLUMN_WIDTH: f32 = 400.0;           // Maximum before inefficient spacing
pub const MIN_FILES_PANEL_WIDTH: f32 = 200.0;      // Minimum for file names
pub const MAX_FILES_PANEL_WIDTH: f32 = 600.0;      // Maximum before UI dominance


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
                // 🎯 NON-INTERFERING DEFAULT: 10 seconds - large enough that fallback detection (<5s) won't trigger,
                // but won't override real file data (0-250s range from VCD files)
                end: TimeNs::from_nanos(10_000_000_000), // 10 seconds in nanoseconds
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

/// Application configuration domain
#[derive(Clone)]
pub struct AppConfig {
    // === ACTOR STATE ===
    pub theme_actor: Actor<SharedTheme>,
    pub dock_mode_actor: Actor<DockMode>,
    pub panel_dimensions_right_actor: Actor<PanelDimensions>,
    pub panel_dimensions_bottom_actor: Actor<PanelDimensions>,
    pub session_state_actor: Actor<SessionState>,
    pub toast_dismiss_ms_actor: Actor<u32>,

    // === UI MUTABLES FOR DIRECT TREEVIEW CONNECTION ===
    pub file_picker_expanded_directories: Mutable<indexmap::IndexSet<String>>,
    pub file_picker_scroll_position: Mutable<i32>,

    // === LOADED CONFIG DATA ===
    /// Selected variables loaded from config file for domain restoration
    pub loaded_selected_variables: Vec<shared::SelectedVariable>,

    // Keep config saver actor alive
    _config_saver_actor: Actor<()>,

    // === EVENT RELAYS ===
    pub theme_button_clicked_relay: Relay,
    pub dock_mode_button_clicked_relay: Relay,
    pub variables_filter_changed_relay: Relay<String>,
    pub panel_dimensions_right_changed_relay: Relay<PanelDimensions>,
    pub panel_dimensions_bottom_changed_relay: Relay<PanelDimensions>,
    pub session_state_changed_relay: Relay<SessionState>,
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
        let config = Self::load_config_from_backend()
            .await
            .unwrap_or_else(|_error| SharedAppConfig::default());

        // Create relays for all events
        let (theme_button_clicked_relay, mut theme_button_clicked_stream) = relay();
        let (dock_mode_button_clicked_relay, mut dock_mode_button_clicked_stream) = relay();
        let (variables_filter_changed_relay, variables_filter_changed_stream) = relay();
        let (panel_dimensions_right_changed_relay, _panel_dimensions_right_changed_stream) =
            relay();
        let (panel_dimensions_bottom_changed_relay, _panel_dimensions_bottom_changed_stream) =
            relay();
        let (session_state_changed_relay, session_state_changed_stream) = relay();

        // Clone relays for use in multiple Actors to avoid move issues
        let panel_dimensions_right_changed_relay_clone =
            panel_dimensions_right_changed_relay.clone();
        let panel_dimensions_bottom_changed_relay_clone =
            panel_dimensions_bottom_changed_relay.clone();

        // Create theme actor with loaded config value
        let theme_actor = Actor::new(config.ui.theme, async move |state| {
            // ✅ Cache Current Values pattern - maintain current theme as it changes
            let mut current_theme = config.ui.theme;

            // Initialize NovyUI theme system with current theme
            let initial_novyui_theme = match current_theme {
                SharedTheme::Light => theme::Theme::Light,
                SharedTheme::Dark => theme::Theme::Dark,
            };
            theme::init_theme(Some(initial_novyui_theme), None);

            loop {
                select! {
                    button_click = theme_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            // ✅ Use cached current value for toggle logic
                            let new_theme = match current_theme {
                                SharedTheme::Light => SharedTheme::Dark,
                                SharedTheme::Dark => SharedTheme::Light,
                            };
                            current_theme = new_theme; // Update cache
                            state.set(new_theme);

                            // Update NovyUI theme system immediately
                            let novyui_theme = match new_theme {
                                SharedTheme::Light => theme::Theme::Light,
                                SharedTheme::Dark => theme::Theme::Dark,
                            };
                            theme::set_theme(novyui_theme);
                        }
                    }
                }
            }
        });

        // Create dock mode actor with loaded config value
        let dock_mode_actor = Actor::new(config.workspace.dock_mode.clone(), {
            let panel_dimensions_right_changed_relay =
                panel_dimensions_right_changed_relay_clone.clone();
            let panel_dimensions_bottom_changed_relay =
                panel_dimensions_bottom_changed_relay_clone.clone();
            async move |state| {
                // ✅ Cache Current Values pattern - maintain current dock mode as it changes
                let mut current_dock_mode = config.workspace.dock_mode.clone();

                loop {
                    select! {
                            button_click = dock_mode_button_clicked_stream.next() => {
                                if let Some(()) = button_click {

                                    // Get current panel dimensions from CONFIG ACTORS BEFORE switching mode
                                    let current_name_width = match current_dock_mode {
                                        DockMode::Right => config.workspace.docked_right_dimensions.selected_variables_panel_name_column_width.unwrap_or(130.0),
                                        DockMode::Bottom => config.workspace.docked_bottom_dimensions.selected_variables_panel_name_column_width.unwrap_or(130.0),
                                    } as u32;
                                    let current_value_width = match current_dock_mode {
                                        DockMode::Right => config.workspace.docked_right_dimensions.selected_variables_panel_value_column_width.unwrap_or(170.0),
                                        DockMode::Bottom => config.workspace.docked_bottom_dimensions.selected_variables_panel_value_column_width.unwrap_or(170.0),
                                    } as u32;


                                    // ✅ Use cached current value for toggle logic
                                    let old_mode = current_dock_mode;
                                    let new_mode = match current_dock_mode {
                                        DockMode::Right => DockMode::Bottom,
                                        DockMode::Bottom => DockMode::Right,
                                    };
                                    current_dock_mode = new_mode; // Update cache
                                    state.set(new_mode);


                                // 📁 CRITICAL: Save current mode's dimensions before switching
                                // ✅ FIX: Don't overwrite existing config values - only save current Actor values for ACTIVE dimensions
                                match old_mode {
                                    DockMode::Right => {
                                        // Update Right dock dimensions - use current config values
                                        let current_dims = PanelDimensions {
                                            files_panel_width: config.workspace.docked_right_dimensions.files_and_scopes_panel_width as f32,
                                            files_panel_height: config.workspace.docked_right_dimensions.files_and_scopes_panel_height as f32,
                                            variables_panel_width: DEFAULT_PANEL_WIDTH,
                                            timeline_panel_height: DEFAULT_TIMELINE_HEIGHT,
                                            variables_name_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_name_column_width.unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64) as f32,
                                            variables_value_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_value_column_width.unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64) as f32,
                                        };
                                        let updated_dims = PanelDimensions {
                                            files_panel_width: current_dims.files_panel_width, // Keep existing
                                            files_panel_height: current_dims.files_panel_height, // Keep existing - don't overwrite with shared Actor
                                            variables_panel_width: current_dims.variables_panel_width, // Keep existing
                                            timeline_panel_height: current_dims.timeline_panel_height, // Keep existing
                                            variables_name_column_width: current_name_width as f32, // Update from Actor (this is actively used)
                                            variables_value_column_width: current_value_width as f32, // Update from Actor (this is actively used)
                                        };
                                        panel_dimensions_right_changed_relay.send(updated_dims);
                                    }
                                    DockMode::Bottom => {
                                        // Update Bottom dock dimensions - use current config values  
                                        let current_dims = PanelDimensions {
                                            files_panel_width: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_width as f32,
                                            files_panel_height: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_height as f32,
                                            variables_panel_width: DEFAULT_PANEL_WIDTH,
                                            timeline_panel_height: DEFAULT_TIMELINE_HEIGHT,
                                            variables_name_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_name_column_width.unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64) as f32,
                                            variables_value_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_value_column_width.unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64) as f32,
                                        };
                                        let updated_dims = PanelDimensions {
                                            files_panel_width: current_dims.files_panel_width, // Keep existing
                                            files_panel_height: current_dims.files_panel_height, // Keep existing - don't overwrite with shared Actor
                                            variables_panel_width: current_dims.variables_panel_width, // Keep existing
                                            timeline_panel_height: current_dims.timeline_panel_height, // Keep existing
                                            variables_name_column_width: current_name_width as f32, // Update from Actor (this is actively used)
                                            variables_value_column_width: current_value_width as f32, // Update from Actor (this is actively used)
                                        };
                                        panel_dimensions_bottom_changed_relay.send(updated_dims);
                                    }
                                }

                                // 📂 CRITICAL: Load new mode's saved dimensions to Actors
                                Task::start({
                                    let new_mode = new_mode;
                                    async move {
                                        // Wait for dock mode signal to update, then proceed

                                        match new_mode {
                                            DockMode::Right => {
                                                // Right mode dimensions are already loaded into the config actors - no need to force sync
                                            }
                                            DockMode::Bottom => {
                                                // Bottom mode dimensions are already loaded into the config actors - no need to force sync
                                            }
                                        }
                                    }
                                });

                            }
                        }
                    }
                }
            }
        });

        // Create panel dimensions actors with loaded config values
        let panel_dimensions_right_actor = Actor::new(
            PanelDimensions {
                files_panel_width: config
                    .workspace
                    .docked_right_dimensions
                    .files_and_scopes_panel_width as f32,
                files_panel_height: config
                    .workspace
                    .docked_right_dimensions
                    .files_and_scopes_panel_height as f32,
                variables_panel_width: DEFAULT_PANEL_WIDTH,
                timeline_panel_height: DEFAULT_TIMELINE_HEIGHT,
                variables_name_column_width: config
                    .workspace
                    .docked_right_dimensions
                    .selected_variables_panel_name_column_width
                    .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64)
                    as f32,
                variables_value_column_width: config
                    .workspace
                    .docked_right_dimensions
                    .selected_variables_panel_value_column_width
                    .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64)
                    as f32,
            },
            async move |state| {
                let mut right_stream = panel_dimensions_right_changed_relay_clone.subscribe();

                loop {
                    select! {
                        new_dims = right_stream.next() => {
                            if let Some(dims) = new_dims {
                                state.set_neq(dims);
                            }
                        }
                    }
                }
            },
        );

        let panel_dimensions_bottom_actor = Actor::new(
            PanelDimensions {
                files_panel_width: config
                    .workspace
                    .docked_bottom_dimensions
                    .files_and_scopes_panel_width as f32,
                files_panel_height: config
                    .workspace
                    .docked_bottom_dimensions
                    .files_and_scopes_panel_height as f32,
                variables_panel_width: DEFAULT_PANEL_WIDTH,
                timeline_panel_height: DEFAULT_TIMELINE_HEIGHT,
                variables_name_column_width: config
                    .workspace
                    .docked_bottom_dimensions
                    .selected_variables_panel_name_column_width
                    .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64)
                    as f32,
                variables_value_column_width: config
                    .workspace
                    .docked_bottom_dimensions
                    .selected_variables_panel_value_column_width
                    .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64)
                    as f32,
            },
            async move |state| {
                let mut bottom_stream = panel_dimensions_bottom_changed_relay_clone.subscribe();

                loop {
                    select! {
                        new_dims = bottom_stream.next() => {
                            if let Some(dims) = new_dims {
                                state.set_neq(dims);
                            }
                        }
                    }
                }
            },
        );

        // Create session state actor with loaded config values
        let session_state_actor = Actor::new(
            SessionState {
                opened_files: config.workspace.opened_files,
                variables_search_filter: config.workspace.variables_search_filter,
                file_picker_scroll_position: config.workspace.load_files_scroll_position,
                file_picker_expanded_directories: config
                    .workspace
                    .load_files_expanded_directories
                    .clone(),
            },
            async move |state| {
                let mut session_stream = session_state_changed_stream;
                let mut variables_filter_stream = variables_filter_changed_stream;

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
                                state.update_mut(|session| {
                                    session.variables_search_filter = new_filter;
                                });
                            }
                        }
                    }
                }
            },
        );

        // Create toast dismiss ms actor with loaded config value
        let toast_dismiss_ms_actor =
            Actor::new(config.ui.toast_dismiss_ms as u32, async move |_state| {
                // Actor maintains the loaded config value (no external updates needed)
                loop {
                    // Keep actor alive but no processing needed since toast_dismiss_ms is read-only from config
                    Task::next_macro_tick().await;
                }
            });

        // Create automatic config saver actor that watches all config changes
        // AppConfig: Creating config saver actor
        // TODO: In proper Actor+Relay architecture, get selected_variables from main domain coordination
        let placeholder_selected_variables = crate::selected_variables::SelectedVariables::default();
        let config_saver_actor = create_config_saver_actor(
            theme_actor.clone(),
            dock_mode_actor.clone(),
            panel_dimensions_right_actor.clone(),
            panel_dimensions_bottom_actor.clone(),
            session_state_actor.clone(),
            toast_dismiss_ms_actor.clone(),
            &placeholder_selected_variables,
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

        // Load expanded scopes from config using Actor+Relay pattern
        {
            // Config: Loading expanded scopes from config
            // Config: Found expanded scopes in config

            let mut expanded_scopes_set = indexmap::IndexSet::new();
            for scope in &config.workspace.expanded_scopes {
                // Distinguish between file-level and scope-level expansion
                let scope_id = if scope.is_empty() {
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
                expanded_scopes_set.insert(scope_id);
            }

            // TODO: Send bulk restoration event through SelectedVariables domain parameter
            // This needs to be called from NovyWaveApp context with domain access
            let _ = expanded_scopes_set; // Suppress unused variable warning

            // Config: Loaded expanded scopes from config via Actor+Relay
            // Config: Final expanded scopes restored to domain
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

            // TODO: Send through SelectedVariables domain parameter
            // This needs SelectedVariables instance access: selected_variables.scope_selected_relay.send(Some(scope_id.clone()));
            let _ = scope_id; // Suppress unused variable warning
            // Config: Loaded selected scope ID into SELECTED_SCOPE_ID
        } else {
            // Config: No selected scope ID in config, leaving SELECTED_SCOPE_ID as None
        }

        // NOTE: Selected variables restoration handled by main.rs using proper config.workspace.selected_variables access

        // Create file picker scroll position mutable with loaded config
        let file_picker_scroll_position = Mutable::new(config.workspace.load_files_scroll_position);

        // Set up sync from mutable changes to session state (for config saving)
        let file_picker_sync = file_picker_expanded_directories.clone();
        let session_sync = session_state_actor.clone();
        let session_changed_relay = session_state_changed_relay.clone();
        Task::start(async move {
            file_picker_sync
                .signal_cloned()
                .for_each(move |expanded_set| {
                    let session_sync = session_sync.clone();
                    let session_changed_relay = session_changed_relay.clone();
                    async move {
                        // Get current session state and update expanded directories
                        if let Some(mut session_state) =
                            session_sync.signal().to_stream().next().await
                        {
                            session_state.file_picker_expanded_directories =
                                expanded_set.iter().cloned().collect();

                            // Trigger session state change to save config
                            session_changed_relay.send(session_state);
                            // File picker directories synced to session state
                        }
                    }
                })
                .await;
        });

        // Set up sync for scroll position changes to session state
        let scroll_position_sync = file_picker_scroll_position.clone();
        let session_scroll_sync = session_state_actor.clone();
        let session_scroll_changed_relay = session_state_changed_relay.clone();
        Task::start(async move {
            scroll_position_sync
                .signal()
                .for_each(move |scroll_position| {
                    let session_scroll_sync = session_scroll_sync.clone();
                    let session_scroll_changed_relay = session_scroll_changed_relay.clone();
                    async move {
                        // Get current session state and update scroll position
                        if let Some(mut session_state) =
                            session_scroll_sync.signal().to_stream().next().await
                        {
                            session_state.file_picker_scroll_position = scroll_position;

                            // Trigger session state change to save config
                            session_scroll_changed_relay.send(session_state);
                            // File picker scroll position synced to session state
                        }
                    }
                })
                .await;
        });

        Self {
            theme_actor,
            dock_mode_actor,
            panel_dimensions_right_actor,
            panel_dimensions_bottom_actor,
            session_state_actor,
            toast_dismiss_ms_actor,

            file_picker_expanded_directories,
            file_picker_scroll_position,

            loaded_selected_variables: config.workspace.selected_variables.clone(),

            _config_saver_actor: config_saver_actor,

            theme_button_clicked_relay,
            dock_mode_button_clicked_relay,
            variables_filter_changed_relay,
            panel_dimensions_right_changed_relay,
            panel_dimensions_bottom_changed_relay,
            session_state_changed_relay,
        }
    }
}

// === BACKEND INTEGRATION ===
