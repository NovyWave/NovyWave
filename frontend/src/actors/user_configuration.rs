//! UserConfiguration domain for configuration management using Actor+Relay architecture
//!
//! Consolidated configuration management domain to replace global mutables with event-driven architecture.
//! Manages themes, layouts, preferences, and persistent settings.

use crate::actors::{Actor, ActorVec, ActorMap, Relay, relay};
use shared::{Theme, DockMode, DockedRightDimensions, DockedBottomDimensions, VarFormat, AppConfig, AppSection, UiSection, WorkspaceSection};
use zoon::{SignalVecExt, MutableVecExt};
use futures::StreamExt;
use std::collections::BTreeMap;

/// Domain-driven configuration management with Actor+Relay architecture.
/// 
/// Replaces configuration-related global mutables with cohesive event-driven state management.
/// Manages themes, panel layouts, file paths, and user preferences.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct UserConfiguration {
    /// Current application theme
    current_theme: Actor<Theme>,
    
    /// Panel dock mode (Right or Bottom)
    dock_mode: Actor<DockMode>,
    
    /// Panel dimensions for right docking
    docked_right_dimensions: Actor<DockedRightDimensions>,
    
    /// Panel dimensions for bottom docking
    docked_bottom_dimensions: Actor<DockedBottomDimensions>,
    
    /// Recently opened file paths
    recent_files: ActorVec<String>,
    
    /// Default variable format preferences
    default_formats: ActorMap<String, VarFormat>,
    
    /// Panel visibility settings
    panel_visibility: ActorMap<String, bool>,
    
    /// Configuration loading state
    is_config_loaded: Actor<bool>,
    
    /// Configuration saving state
    is_config_saving: Actor<bool>,
    
    // === USER CONFIGURATION EVENTS ===
    /// User changed theme in settings
    pub theme_changed_relay: Relay<Theme>,
    
    /// User switched dock mode
    pub dock_mode_changed_relay: Relay<DockMode>,
    
    /// User resized right panel
    pub right_panel_resized_relay: Relay<DockedRightDimensions>,
    
    /// User resized bottom panel
    pub bottom_panel_resized_relay: Relay<DockedBottomDimensions>,
    
    /// User toggled panel visibility
    pub panel_visibility_toggled_relay: Relay<(String, bool)>,
    
    /// User changed default format for variable type
    pub default_format_changed_relay: Relay<(String, VarFormat)>,
    
    /// User added file to recent files
    pub file_added_to_recent_relay: Relay<String>,
    
    /// User cleared recent files
    pub recent_files_cleared_relay: Relay<()>,
    
    // === SYSTEM CONFIGURATION EVENTS ===
    /// Configuration loaded from storage
    pub config_loaded_relay: Relay<AppConfig>,
    
    /// Configuration saved to storage
    pub config_saved_relay: Relay<()>,
    
    /// Configuration save requested
    pub config_save_requested_relay: Relay<()>,
    
    /// Configuration reset to defaults requested
    pub config_reset_requested_relay: Relay<()>,
    
    /// Configuration export requested
    pub config_export_requested_relay: Relay<String>,
    
    /// Configuration import requested
    pub config_import_requested_relay: Relay<String>,
}

#[allow(dead_code)]
impl UserConfiguration {
    /// Create a new UserConfiguration domain with event processors
    pub async fn new() -> Self {
        // Create relays for user configuration events
        let (theme_changed_relay, theme_changed_stream) = relay::<Theme>();
        let (dock_mode_changed_relay, dock_mode_changed_stream) = relay::<DockMode>();
        let (right_panel_resized_relay, right_panel_resized_stream) = relay::<DockedRightDimensions>();
        let (bottom_panel_resized_relay, bottom_panel_resized_stream) = relay::<DockedBottomDimensions>();
        let (panel_visibility_toggled_relay, panel_visibility_toggled_stream) = relay::<(String, bool)>();
        let (default_format_changed_relay, default_format_changed_stream) = relay::<(String, VarFormat)>();
        let (file_added_to_recent_relay, file_added_to_recent_stream) = relay::<String>();
        let (recent_files_cleared_relay, recent_files_cleared_stream) = relay::<()>();
        
        // Create relays for system configuration events
        let (config_loaded_relay, config_loaded_stream) = relay::<AppConfig>();
        let (config_saved_relay, config_saved_stream) = relay::<()>();
        let (config_save_requested_relay, config_save_requested_stream) = relay::<()>();
        let (config_reset_requested_relay, _config_reset_requested_stream) = relay::<()>();
        let (config_export_requested_relay, _config_export_requested_stream) = relay::<String>();
        let (config_import_requested_relay, _config_import_requested_stream) = relay::<String>();
        
        // Create theme actor with event handling
        let current_theme = Actor::new(Theme::Dark, async move |state| {
            let mut theme_changed = theme_changed_stream;
            
            while let Some(theme) = theme_changed.next().await {
                state.set(theme);
            }
        });
        
        // Create dock mode actor
        let dock_mode = Actor::new(DockMode::Right, async move |state| {
            let mut dock_mode_changed = dock_mode_changed_stream;
            
            while let Some(mode) = dock_mode_changed.next().await {
                state.set(mode);
            }
        });
        
        // Create panel dimensions actors
        let docked_right_dimensions = Actor::new(
            DockedRightDimensions::default(),
            async move |state| {
                let mut right_panel_resized = right_panel_resized_stream;
                
                while let Some(dimensions) = right_panel_resized.next().await {
                    state.set(dimensions);
                }
            }
        );
        
        let docked_bottom_dimensions = Actor::new(
            DockedBottomDimensions::default(),
            async move |state| {
                let mut bottom_panel_resized = bottom_panel_resized_stream;
                
                while let Some(dimensions) = bottom_panel_resized.next().await {
                    state.set(dimensions);
                }
            }
        );
        
        // Create recent files actor
        let recent_files = ActorVec::new(vec![], async move |files| {
            let mut file_added_to_recent = file_added_to_recent_stream;
            let mut recent_files_cleared = recent_files_cleared_stream;
            
            loop {
                if let Some(file_path) = file_added_to_recent.next().await {
                    // Add file to recent list (remove duplicates and limit to 10)
                    files.update_mut(|files| {
                        // Remove if already present
                        if let Some(pos) = files.iter().position(|f| f == &file_path) {
                            files.remove(pos);
                        }
                        // Add to front
                        files.push_cloned(file_path);
                        // Limit to 10 recent files
                        if files.len() > 10 {
                            files.truncate(10);
                        }
                    });
                } else if let Some(()) = recent_files_cleared.next().await {
                    files.lock_mut().clear();
                } else {
                    break;
                }
            }
        });
        
        // Create default formats map
        let default_formats = ActorMap::new(BTreeMap::new(), async move |map| {
            let mut default_format_changed = default_format_changed_stream;
            
            while let Some((var_type, format)) = default_format_changed.next().await {
                map.lock_mut().insert_cloned(var_type, format);
            }
        });
        
        // Create panel visibility map
        let panel_visibility = ActorMap::new(BTreeMap::new(), async move |map| {
            let mut panel_visibility_toggled = panel_visibility_toggled_stream;
            
            while let Some((panel_name, is_visible)) = panel_visibility_toggled.next().await {
                map.lock_mut().insert_cloned(panel_name, is_visible);
            }
        });
        
        // Create configuration state actors
        let is_config_loaded = Actor::new(false, async move |state| {
            let mut config_loaded = config_loaded_stream;
            
            while let Some(_config) = config_loaded.next().await {
                state.set(true);
            }
        });
        
        let is_config_saving = Actor::new(false, async move |state| {
            let mut config_save_requested = config_save_requested_stream;
            let mut config_saved = config_saved_stream;
            
            loop {
                if let Some(()) = config_save_requested.next().await {
                    state.set(true);
                } else if let Some(()) = config_saved.next().await {
                    state.set(false);
                } else {
                    break;
                }
            }
        });
        
        Self {
            current_theme,
            dock_mode,
            docked_right_dimensions,
            docked_bottom_dimensions,
            recent_files,
            default_formats,
            panel_visibility,
            is_config_loaded,
            is_config_saving,
            
            theme_changed_relay,
            dock_mode_changed_relay,
            right_panel_resized_relay,
            bottom_panel_resized_relay,
            panel_visibility_toggled_relay,
            default_format_changed_relay,
            file_added_to_recent_relay,
            recent_files_cleared_relay,
            
            config_loaded_relay,
            config_saved_relay,
            config_save_requested_relay,
            config_reset_requested_relay,
            config_export_requested_relay,
            config_import_requested_relay,
        }
    }
    
    // === REACTIVE SIGNAL ACCESS ===
    
    /// Get reactive signal for current theme
    pub fn current_theme_signal(&self) -> impl zoon::Signal<Item = Theme> {
        self.current_theme.signal()
    }
    
    /// Get reactive signal for dock mode
    pub fn dock_mode_signal(&self) -> impl zoon::Signal<Item = DockMode> {
        self.dock_mode.signal()
    }
    
    /// Get reactive signal for right panel dimensions
    pub fn docked_right_dimensions_signal(&self) -> impl zoon::Signal<Item = DockedRightDimensions> {
        self.docked_right_dimensions.signal()
    }
    
    /// Get reactive signal for bottom panel dimensions
    pub fn docked_bottom_dimensions_signal(&self) -> impl zoon::Signal<Item = DockedBottomDimensions> {
        self.docked_bottom_dimensions.signal()
    }
    
    /// Get reactive signal for recent files
    pub fn recent_files_signal(&self) -> impl zoon::Signal<Item = Vec<String>> {
        self.recent_files.signal_vec().to_signal_cloned()
    }
    
    /// Get reactive signal for default formats
    pub fn default_formats_signal(&self) -> impl zoon::Signal<Item = BTreeMap<String, VarFormat>> {
        use zoon::SignalExt;
        self.default_formats.entries_signal_vec().to_signal_cloned().map(|entries| {
            entries.into_iter().collect()
        })
    }
    
    /// Get reactive signal for panel visibility
    pub fn panel_visibility_signal(&self) -> impl zoon::Signal<Item = BTreeMap<String, bool>> {
        use zoon::SignalExt;
        self.panel_visibility.entries_signal_vec().to_signal_cloned().map(|entries| {
            entries.into_iter().collect()
        })
    }
    
    /// Get reactive signal for configuration loaded state
    pub fn is_config_loaded_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.is_config_loaded.signal()
    }
    
    /// Get reactive signal for configuration saving state
    pub fn is_config_saving_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.is_config_saving.signal()
    }
    
    /// Get signal for specific panel visibility
    pub fn is_panel_visible_signal(&self, panel_name: String) -> impl zoon::Signal<Item = bool> {
        use zoon::SignalExt;
        self.panel_visibility.value_signal(panel_name).map(|opt_value| {
            opt_value.unwrap_or(true) // Default to visible
        })
    }
    
    /// Get signal for default format of variable type
    pub fn default_format_for_type_signal(&self, var_type: String) -> impl zoon::Signal<Item = VarFormat> {
        use zoon::SignalExt;
        self.default_formats.value_signal(var_type).map(|opt_value| {
            opt_value.unwrap_or(VarFormat::Binary) // Default format
        })
    }
    
    /// Check if config is currently being saved
    pub fn is_config_dirty_signal(&self) -> impl zoon::Signal<Item = bool> {
        // Simplified - can be enhanced with change tracking
        use zoon::SignalExt;
        self.is_config_saving.signal().map(|saving| !saving)
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

#[allow(dead_code)]
impl UserConfiguration {
    /// Apply loaded configuration to all actors
    fn apply_loaded_config(
        config: &AppConfig,
        theme_handle: &zoon::Mutable<Theme>,
        dock_handle: &zoon::Mutable<DockMode>,
        // Add other handles as needed
    ) {
        // Apply theme from UI section
        theme_handle.set(config.ui.theme.clone());
        
        // Apply dock mode from workspace section
        dock_handle.set(config.workspace.dock_mode.clone());
        
        // Additional config application can be added here
    }
    
    /// Create default configuration
    fn create_default_config() -> AppConfig {
        AppConfig {
            app: AppSection {
                version: "1.0.0".to_string(),
            },
            ui: UiSection {
                theme: Theme::Dark,
                toast_dismiss_ms: 10000,
            },
            workspace: WorkspaceSection {
                opened_files: vec![],
                dock_mode: DockMode::Right,
                expanded_scopes: vec![],
                load_files_expanded_directories: vec![],
                selected_scope_id: None,
                docked_bottom_dimensions: DockedBottomDimensions::default(),
                docked_right_dimensions: DockedRightDimensions::default(),
                load_files_scroll_position: 0,
                variables_search_filter: String::new(),
                selected_variables: vec![],
                timeline_cursor_position_ns: 0,
                timeline_zoom_level: 1.0,
                timeline_visible_range_start_ns: None,
                timeline_visible_range_end_ns: None,
            },
        }
    }
    
    /// Export configuration to JSON string
    fn export_config_to_json(config: &AppConfig) -> Result<String, String> {
        serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))
    }
    
    /// Import configuration from JSON string
    fn import_config_from_json(json: &str) -> Result<AppConfig, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse config JSON: {}", e))
    }
}

// === CONVENIENCE FUNCTIONS FOR UI INTEGRATION ===

/// Global UserConfiguration instance
static _USER_CONFIGURATION_INSTANCE: std::sync::OnceLock<UserConfiguration> = std::sync::OnceLock::new();

/// Initialize the UserConfiguration domain (call once on app startup)
#[allow(dead_code)]
pub async fn initialize_user_configuration() -> UserConfiguration {
    let user_configuration = UserConfiguration::new().await;
    if let Err(_) = _USER_CONFIGURATION_INSTANCE.set(user_configuration.clone()) {
        zoon::eprintln!("⚠️ UserConfiguration already initialized - ignoring duplicate initialization");
    }
    user_configuration
}

/// Get the global UserConfiguration instance
#[allow(dead_code)]
pub fn get_user_configuration() -> Option<UserConfiguration> {
    _USER_CONFIGURATION_INSTANCE.get().map(|config| config.clone())
}