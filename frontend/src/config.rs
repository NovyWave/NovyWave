// New Unified Config System
// Based on ringrev_private patterns: Big struct with nested Mutable values and triggers

use zoon::*;
use serde::{Deserialize, Serialize};
use educe::Educe;
use shared::UpMsg;

// =============================================================================
// MAIN CONFIG STORE - Single Source of Truth with Reactive Fields
// =============================================================================

#[derive(Clone)]
pub struct ConfigStore {
    pub app: Mutable<AppSection>,
    pub ui: Mutable<UiSection>,
    pub session: Mutable<SessionSection>,
    pub workspace: Mutable<WorkspaceSection>,
    pub dialogs: Mutable<DialogSection>,
}

// =============================================================================
// APP SECTION - Version and Migration
// =============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSection {
    pub version: Mutable<String>,
    pub migration_strategy: Mutable<MigrationStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    None,
    Upgrade,
    Recreate,
}

// =============================================================================
// UI SECTION - Theme and Visual Preferences
// =============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct UiSection {
    pub theme: Mutable<Theme>,
    pub font_size: Mutable<f32>,
    pub show_tooltips: Mutable<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

// =============================================================================
// SESSION SECTION - Files and Search State
// =============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionSection {
    pub opened_files: MutableVec<String>,
    pub variables_search_filter: Mutable<String>,
    pub file_picker: Mutable<FilePickerSection>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FilePickerSection {
    pub current_directory: Mutable<Option<String>>,
    pub expanded_directories: MutableVec<String>,
    pub show_hidden_files: Mutable<bool>,
}

// =============================================================================
// WORKSPACE SECTION - Layout and Panel State
// =============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkspaceSection {
    pub dock_mode: Mutable<DockMode>,
    pub selected_scope_id: Mutable<Option<String>>,
    pub expanded_scopes: MutableVec<String>,
    pub panel_layouts: Mutable<PanelLayouts>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DockMode {
    Bottom,
    Right,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PanelLayouts {
    pub docked_to_bottom: Mutable<PanelDimensions>,
    pub docked_to_right: Mutable<PanelDimensions>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PanelDimensions {
    pub files_panel_width: Mutable<f32>,
    pub files_panel_height: Mutable<f32>,
    pub variables_panel_width: Mutable<f32>,
    pub timeline_panel_height: Mutable<f32>,
}

// =============================================================================
// DIALOG SECTION - Dialog and Modal State
// =============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct DialogSection {
    pub show_file_dialog: Mutable<bool>,
    pub show_settings_dialog: Mutable<bool>,
    pub show_about_dialog: Mutable<bool>,
    pub file_paths_input: Mutable<String>,
}

// =============================================================================
// GLOBAL CONFIG STORE INSTANCE
// =============================================================================

static CONFIG_STORE: Lazy<ConfigStore> = Lazy::new(|| ConfigStore::new());

// Global flag to track config loading status (for compatibility with existing code)
pub static CONFIG_LOADED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// =============================================================================
// DEFAULT IMPLEMENTATIONS (Mutable-based)
// =============================================================================

impl Default for ConfigStore {
    fn default() -> Self {
        Self {
            app: Mutable::new(AppSection::default()),
            ui: Mutable::new(UiSection::default()),
            session: Mutable::new(SessionSection::default()),
            workspace: Mutable::new(WorkspaceSection::default()),
            dialogs: Mutable::new(DialogSection::default()),
        }
    }
}

impl ConfigStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            version: Mutable::new("1.0.0".to_string()),
            migration_strategy: Mutable::new(MigrationStrategy::None),
        }
    }
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: Mutable::new(Theme::Dark),
            font_size: Mutable::new(14.0),
            show_tooltips: Mutable::new(true),
        }
    }
}

impl Default for SessionSection {
    fn default() -> Self {
        Self {
            opened_files: MutableVec::new(),
            variables_search_filter: Mutable::new(String::new()),
            file_picker: Mutable::new(FilePickerSection::default()),
        }
    }
}

impl Default for FilePickerSection {
    fn default() -> Self {
        Self {
            current_directory: Mutable::new(None),
            expanded_directories: MutableVec::new(),
            show_hidden_files: Mutable::new(false),
        }
    }
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            dock_mode: Mutable::new(DockMode::Bottom),
            selected_scope_id: Mutable::new(None),
            expanded_scopes: MutableVec::new(),
            panel_layouts: Mutable::new(PanelLayouts::default()),
        }
    }
}

impl Default for PanelLayouts {
    fn default() -> Self {
        Self {
            docked_to_bottom: Mutable::new(PanelDimensions {
                files_panel_width: Mutable::new(1400.0),
                files_panel_height: Mutable::new(600.0),
                variables_panel_width: Mutable::new(300.0),
                timeline_panel_height: Mutable::new(200.0),
            }),
            docked_to_right: Mutable::new(PanelDimensions {
                files_panel_width: Mutable::new(400.0),
                files_panel_height: Mutable::new(300.0),
                variables_panel_width: Mutable::new(250.0),
                timeline_panel_height: Mutable::new(150.0),
            }),
        }
    }
}

impl Default for DialogSection {
    fn default() -> Self {
        Self {
            show_file_dialog: Mutable::new(false),
            show_settings_dialog: Mutable::new(false),
            show_about_dialog: Mutable::new(false),
            file_paths_input: Mutable::new(String::new()),
        }
    }
}

// =============================================================================
// SERIALIZATION HELPERS - Convert between Mutable and serializable types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableConfig {
    pub app: SerializableAppSection,
    pub ui: SerializableUiSection,
    pub session: SerializableSessionSection,
    pub workspace: SerializableWorkspaceSection,
    pub dialogs: SerializableDialogSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableAppSection {
    pub version: String,
    pub migration_strategy: MigrationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableUiSection {
    pub theme: Theme,
    pub font_size: f32,
    pub show_tooltips: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSessionSection {
    pub opened_files: Vec<String>,
    pub variables_search_filter: String,
    pub file_picker: SerializableFilePickerSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFilePickerSection {
    pub current_directory: Option<String>,
    pub expanded_directories: Vec<String>,
    pub show_hidden_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableWorkspaceSection {
    pub dock_mode: DockMode,
    pub selected_scope_id: Option<String>,
    pub expanded_scopes: Vec<String>,
    pub panel_layouts: SerializablePanelLayouts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializablePanelLayouts {
    pub docked_to_bottom: SerializablePanelDimensions,
    pub docked_to_right: SerializablePanelDimensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializablePanelDimensions {
    pub files_panel_width: f32,
    pub files_panel_height: f32,
    pub variables_panel_width: f32,
    pub timeline_panel_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableDialogSection {
    pub show_file_dialog: bool,
    pub show_settings_dialog: bool,
    pub show_about_dialog: bool,
    pub file_paths_input: String,
}

impl ConfigStore {
    pub fn to_serializable(&self) -> SerializableConfig {
        SerializableConfig {
            app: SerializableAppSection {
                version: self.app.lock_ref().version.get_cloned(),
                migration_strategy: self.app.lock_ref().migration_strategy.get_cloned(),
            },
            ui: SerializableUiSection {
                theme: self.ui.lock_ref().theme.get_cloned(),
                font_size: self.ui.lock_ref().font_size.get(),
                show_tooltips: self.ui.lock_ref().show_tooltips.get(),
            },
            session: SerializableSessionSection {
                opened_files: self.session.lock_ref().opened_files.lock_ref().to_vec(),
                variables_search_filter: self.session.lock_ref().variables_search_filter.get_cloned(),
                file_picker: SerializableFilePickerSection {
                    current_directory: self.session.lock_ref().file_picker.lock_ref().current_directory.get_cloned(),
                    expanded_directories: self.session.lock_ref().file_picker.lock_ref().expanded_directories.lock_ref().to_vec(),
                    show_hidden_files: self.session.lock_ref().file_picker.lock_ref().show_hidden_files.get(),
                },
            },
            workspace: SerializableWorkspaceSection {
                dock_mode: self.workspace.lock_ref().dock_mode.get_cloned(),
                selected_scope_id: self.workspace.lock_ref().selected_scope_id.get_cloned(),
                expanded_scopes: self.workspace.lock_ref().expanded_scopes.lock_ref().to_vec(),
                panel_layouts: SerializablePanelLayouts {
                    docked_to_bottom: SerializablePanelDimensions {
                        files_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().files_panel_width.get(),
                        files_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().files_panel_height.get(),
                        variables_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().variables_panel_width.get(),
                        timeline_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().timeline_panel_height.get(),
                    },
                    docked_to_right: SerializablePanelDimensions {
                        files_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().files_panel_width.get(),
                        files_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().files_panel_height.get(),
                        variables_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().variables_panel_width.get(),
                        timeline_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().timeline_panel_height.get(),
                    },
                },
            },
            dialogs: SerializableDialogSection {
                show_file_dialog: self.dialogs.lock_ref().show_file_dialog.get(),
                show_settings_dialog: self.dialogs.lock_ref().show_settings_dialog.get(),
                show_about_dialog: self.dialogs.lock_ref().show_about_dialog.get(),
                file_paths_input: self.dialogs.lock_ref().file_paths_input.get_cloned(),
            },
        }
    }

    pub fn load_from_serializable(&self, config: SerializableConfig) {
        // Load app section
        self.app.lock_mut().version.set(config.app.version);
        self.app.lock_mut().migration_strategy.set(config.app.migration_strategy);

        // Load UI section
        self.ui.lock_mut().theme.set(config.ui.theme);
        self.ui.lock_mut().font_size.set(config.ui.font_size);
        self.ui.lock_mut().show_tooltips.set(config.ui.show_tooltips);

        // Load session section
        self.session.lock_mut().opened_files.lock_mut().replace_cloned(config.session.opened_files);
        self.session.lock_mut().variables_search_filter.set(config.session.variables_search_filter);
        
        {
            let session_ref = self.session.lock_ref();
            let file_picker = session_ref.file_picker.lock_ref();
            file_picker.current_directory.set(config.session.file_picker.current_directory);
            file_picker.expanded_directories.lock_mut().replace_cloned(config.session.file_picker.expanded_directories);
            file_picker.show_hidden_files.set(config.session.file_picker.show_hidden_files);
        }

        // Load workspace section
        self.workspace.lock_mut().dock_mode.set(config.workspace.dock_mode);
        self.workspace.lock_mut().selected_scope_id.set(config.workspace.selected_scope_id);
        self.workspace.lock_mut().expanded_scopes.lock_mut().replace_cloned(config.workspace.expanded_scopes);

        {
            let workspace_ref = self.workspace.lock_ref();
            let panel_layouts = workspace_ref.panel_layouts.lock_ref();
            
            {
                let bottom_dims = panel_layouts.docked_to_bottom.lock_ref();
                bottom_dims.files_panel_width.set(config.workspace.panel_layouts.docked_to_bottom.files_panel_width);
                bottom_dims.files_panel_height.set(config.workspace.panel_layouts.docked_to_bottom.files_panel_height);
                bottom_dims.variables_panel_width.set(config.workspace.panel_layouts.docked_to_bottom.variables_panel_width);
                bottom_dims.timeline_panel_height.set(config.workspace.panel_layouts.docked_to_bottom.timeline_panel_height);
            }

            {
                let right_dims = panel_layouts.docked_to_right.lock_ref();
                right_dims.files_panel_width.set(config.workspace.panel_layouts.docked_to_right.files_panel_width);
                right_dims.files_panel_height.set(config.workspace.panel_layouts.docked_to_right.files_panel_height);
                right_dims.variables_panel_width.set(config.workspace.panel_layouts.docked_to_right.variables_panel_width);
                right_dims.timeline_panel_height.set(config.workspace.panel_layouts.docked_to_right.timeline_panel_height);
            }
        }

        // Load dialogs section
        self.dialogs.lock_mut().show_file_dialog.set(config.dialogs.show_file_dialog);
        self.dialogs.lock_mut().show_settings_dialog.set(config.dialogs.show_settings_dialog);
        self.dialogs.lock_mut().show_about_dialog.set(config.dialogs.show_about_dialog);
        self.dialogs.lock_mut().file_paths_input.set(config.dialogs.file_paths_input);
    }
}

// =============================================================================
// TRIGGERS MODULE - Reactive Config Persistence
// =============================================================================

pub fn create_config_triggers() {
    store_config_on_any_change();
}

fn store_config_on_any_change() {
    // UI theme changes - get signal and drop lock immediately
    let theme_signal = {
        let ui = config_store().ui.lock_ref();
        ui.theme.signal_cloned()
    };
    Task::start(async move {
        theme_signal.for_each_sync(|_| {
            zoon::println!("Theme changed, saving config");
            save_config_to_backend();
        }).await
    });
    
    // Dock mode changes - get signal and drop lock immediately  
    let dock_mode_signal = {
        let workspace = config_store().workspace.lock_ref();
        workspace.dock_mode.signal_cloned()
    };
    Task::start(async move {
        dock_mode_signal.for_each_sync(|_| {
            zoon::println!("Dock mode changed, saving config");
            save_config_to_backend();
        }).await
    });
    
    // Panel dimension changes - get signals and drop locks immediately
    let bottom_width_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let bottom = layouts.docked_to_bottom.lock_ref();
        bottom.files_panel_width.signal()
    };
    Task::start(async move {
        bottom_width_signal.for_each_sync(|_| {
            zoon::println!("Bottom files panel width changed, saving config");
            save_config_to_backend();
        }).await
    });
    
    let bottom_height_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let bottom = layouts.docked_to_bottom.lock_ref();
        bottom.files_panel_height.signal()
    };
    Task::start(async move {
        bottom_height_signal.for_each_sync(|_| {
            zoon::println!("Bottom files panel height changed, saving config");
            save_config_to_backend();
        }).await
    });
    
    let right_width_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let right = layouts.docked_to_right.lock_ref();
        right.files_panel_width.signal()
    };
    Task::start(async move {
        right_width_signal.for_each_sync(|_| {
            zoon::println!("Right files panel width changed, saving config");
            save_config_to_backend();
        }).await
    });
    
    let right_height_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let right = layouts.docked_to_right.lock_ref();
        right.files_panel_height.signal()
    };
    Task::start(async move {
        right_height_signal.for_each_sync(|_| {
            zoon::println!("Right files panel height changed, saving config");
            save_config_to_backend();
        }).await
    });
}

fn save_config_to_backend() {
    use crate::connection::send_up_msg;
    
    zoon::println!("save_config_to_backend called!");
    let serializable_config = config_store().to_serializable();
    zoon::println!("Theme in config: {:?}", serializable_config.ui.theme);
    
    // Convert to shared::AppConfig format for backend compatibility
    let app_config = shared::AppConfig {
        app: shared::AppSection {
            version: serializable_config.app.version,
        },
        ui: shared::UiSection {
            theme: match serializable_config.ui.theme {
                Theme::Dark => "dark".to_string(),
                Theme::Light => "light".to_string(),
            },
        },
        workspace: shared::WorkspaceSection {
            opened_files: serializable_config.session.opened_files,
            dock_mode: match serializable_config.workspace.dock_mode {
                DockMode::Bottom => "bottom".to_string(),
                DockMode::Right => "right".to_string(),
            },
            expanded_scopes: serializable_config.workspace.expanded_scopes,
            selected_scope_id: serializable_config.workspace.selected_scope_id,
            docked_to_bottom: shared::DockedToBottomLayout {
                files_panel_width: serializable_config.workspace.panel_layouts.docked_to_bottom.files_panel_width as f64,
                files_panel_height: serializable_config.workspace.panel_layouts.docked_to_bottom.files_panel_height as f64,
            },
            docked_to_right: shared::DockedToRightLayout {
                files_panel_width: serializable_config.workspace.panel_layouts.docked_to_right.files_panel_width as f64,
                files_panel_height: serializable_config.workspace.panel_layouts.docked_to_right.files_panel_height as f64,
            },
        },
    };

    send_up_msg(UpMsg::SaveConfig(app_config));
}

// =============================================================================
// PUBLIC API - Global Store Access
// =============================================================================

pub fn config_store() -> &'static ConfigStore {
    &CONFIG_STORE
}

// =============================================================================
// BRIDGE FUNCTIONS - Compatibility with existing state.rs globals
// =============================================================================

// Bridge functions for gradual migration from old state.rs system
pub fn save_scope_selection() {
    // This is now handled automatically by the reactive triggers
    // The new system auto-saves when config changes
}

pub fn save_panel_layout() {
    // This is now handled automatically by the reactive triggers
    // The new system auto-saves when config changes
}

pub fn save_current_config() {
    // This is now handled automatically by the reactive triggers
    // The new system auto-saves when config changes
}

pub fn save_file_list() {
    // Simple approach: read directly from legacy FILE_PATHS global
    use crate::state::FILE_PATHS;
    
    let file_paths: Vec<String> = FILE_PATHS.lock_ref().values().cloned().collect();
    config_store().session.lock_mut().opened_files.lock_mut().replace_cloned(file_paths);
    
    // Manually trigger config save since MutableVec reactive signals are complex
    save_config_to_backend();
}

pub fn switch_dock_mode_preserving_dimensions(new_is_docked_to_bottom: bool) {
    // Convert boolean to DockMode enum and update config store
    let dock_mode = if new_is_docked_to_bottom {
        DockMode::Bottom
    } else {
        DockMode::Right
    };
    
    config_store().workspace.lock_mut().dock_mode.set_neq(dock_mode);
}

pub fn apply_config(config: shared::AppConfig) {
    // Load config from backend into the new ConfigStore
    let serializable_config = SerializableConfig {
        app: SerializableAppSection {
            version: config.app.version,
            migration_strategy: MigrationStrategy::None,  // Default since shared doesn't have this field
        },
        ui: SerializableUiSection {
            theme: match config.ui.theme.as_str() {
                "light" => Theme::Light,
                _ => Theme::Dark,
            },
            font_size: 14.0,
            show_tooltips: true,
        },
        session: SerializableSessionSection {
            opened_files: config.workspace.opened_files,
            variables_search_filter: String::new(),
            file_picker: SerializableFilePickerSection {
                current_directory: None,
                expanded_directories: Vec::new(),
                show_hidden_files: false,
            },
        },
        workspace: SerializableWorkspaceSection {
            dock_mode: match config.workspace.dock_mode.as_str() {
                "right" => DockMode::Right,
                _ => DockMode::Bottom,
            },
            selected_scope_id: config.workspace.selected_scope_id,
            expanded_scopes: config.workspace.expanded_scopes,
            panel_layouts: SerializablePanelLayouts {
                docked_to_bottom: SerializablePanelDimensions {
                    files_panel_width: config.workspace.docked_to_bottom.files_panel_width as f32,
                    files_panel_height: config.workspace.docked_to_bottom.files_panel_height as f32,
                    variables_panel_width: 300.0,
                    timeline_panel_height: 200.0,
                },
                docked_to_right: SerializablePanelDimensions {
                    files_panel_width: config.workspace.docked_to_right.files_panel_width as f32,
                    files_panel_height: config.workspace.docked_to_right.files_panel_height as f32,
                    variables_panel_width: 250.0,
                    timeline_panel_height: 150.0,
                },
            },
        },
        dialogs: SerializableDialogSection {
            show_file_dialog: false,
            show_settings_dialog: false,
            show_about_dialog: false,
            file_paths_input: String::new(),
        },
    };

    config_store().load_from_serializable(serializable_config);
    
    // Manual sync of expanded_scopes from config to signal (Vec<String> to HashSet<String>)
    sync_expanded_scopes_from_config();
    
    // Manual sync of opened_files from config to legacy globals
    sync_opened_files_from_config();
    
    // Set config loaded flag
    CONFIG_LOADED.set_neq(true);
}

// =============================================================================
// HELPER FUNCTIONS - Common config operations
// =============================================================================

pub fn current_theme() -> impl Signal<Item = Theme> {
    config_store().ui.signal_ref(|ui| ui.theme.signal_cloned()).flatten()
}

// Manual sync function to convert expanded_scopes from Vec<String> to HashSet<String>
fn sync_expanded_scopes_from_config() {
    use crate::state::EXPANDED_SCOPES;
    
    let expanded_vec = config_store().workspace.lock_ref().expanded_scopes.lock_ref().to_vec();
    
    // Clear existing and insert all items from config
    let mut expanded_scopes = EXPANDED_SCOPES.lock_mut();
    expanded_scopes.clear();
    for scope_id in expanded_vec {
        expanded_scopes.insert(scope_id);
    }
}

// Manual sync function to restore opened_files from config to legacy globals and reload files
fn sync_opened_files_from_config() {
    use crate::state::FILE_PATHS;
    use crate::send_up_msg;
    
    let opened_files = config_store().session.lock_ref().opened_files.lock_ref().to_vec();
    
    // Clear existing FILE_PATHS
    FILE_PATHS.lock_mut().clear();
    
    // Restore each file path and reload the file
    for file_path in opened_files {
        // Generate file ID and store in FILE_PATHS (same pattern as file loading)
        let file_id = shared::generate_file_id(&file_path);
        FILE_PATHS.lock_mut().insert(file_id, file_path.clone());
        
        // Reload the file
        send_up_msg(shared::UpMsg::LoadWaveformFile(file_path));
    }
}

pub fn current_dock_mode() -> impl Signal<Item = DockMode> {
    config_store().workspace.signal_ref(|ws| ws.dock_mode.signal_cloned()).flatten()
}

pub fn is_docked_to_bottom() -> impl Signal<Item = bool> {
    current_dock_mode().map(|mode| matches!(mode, DockMode::Bottom))
}

pub fn panel_dimensions_signal() -> impl Signal<Item = PanelDimensions> {
    map_ref! {
        let dock_mode = current_dock_mode(),
        let layouts = config_store().workspace.signal_ref(|ws| ws.panel_layouts.signal_cloned()).flatten() =>
        match dock_mode {
            DockMode::Bottom => layouts.docked_to_bottom.get_cloned(),
            DockMode::Right => layouts.docked_to_right.get_cloned(),
        }
    }
}

// =============================================================================
// STATE SYNC HELPERS - Bridge between ConfigStore and state.rs globals  
// =============================================================================

// Create tasks that sync config changes to old state.rs globals
pub fn sync_config_to_globals() {
    use crate::state::*;

    // Sync dock mode
    Task::start(async {
        current_dock_mode().for_each_sync(|dock_mode| {
            IS_DOCKED_TO_BOTTOM.set_neq(matches!(dock_mode, DockMode::Bottom));
        }).await
    });

    // Sync panel dimensions based on current dock mode
    Task::start(async {
        panel_dimensions_signal().for_each_sync(|dimensions| {
            FILES_PANEL_WIDTH.set_neq(dimensions.files_panel_width.get() as u32);
            FILES_PANEL_HEIGHT.set_neq(dimensions.files_panel_height.get() as u32);
        }).await
    });

    // Sync selected scope
    Task::start(async {
        config_store().workspace.signal_ref(|ws| ws.selected_scope_id.signal_cloned()).flatten()
            .for_each_sync(|scope_id| {
                SELECTED_SCOPE_ID.set_neq(scope_id);
            }).await
    });

    // Sync expanded scopes (convert between Vec and HashSet)  
    // Note: Manual sync is used since MutableVec signal handling is complex

    // Sync variables search filter
    Task::start(async {
        config_store().session.signal_ref(|s| s.variables_search_filter.signal_cloned()).flatten()
            .for_each_sync(|filter| {
                VARIABLES_SEARCH_FILTER.set_neq(filter);
            }).await
    });

    // Sync dialog states
    Task::start(async {
        config_store().dialogs.signal_ref(|d| d.show_file_dialog.signal()).flatten()
            .for_each_sync(|show| {
                SHOW_FILE_DIALOG.set_neq(show);
            }).await
    });

    Task::start(async {
        config_store().dialogs.signal_ref(|d| d.file_paths_input.signal_cloned()).flatten()
            .for_each_sync(|input| {
                FILE_PATHS_INPUT.set_neq(input);
            }).await
    });
}

// =============================================================================
// REVERSE SYNC - Update ConfigStore when state.rs globals change
// =============================================================================

pub fn sync_globals_to_config() {
    use crate::state::*;

    // Sync panel dimensions back to config when UI updates them
    Task::start(async {
        FILES_PANEL_WIDTH.signal().for_each_sync(|width| {
            zoon::println!("FILES_PANEL_WIDTH changed to: {}", width);
            let dock_mode = config_store().workspace.lock_ref().dock_mode.get_cloned();
            let workspace_ref = config_store().workspace.lock_ref();
            let layouts = workspace_ref.panel_layouts.lock_ref();
            
            match dock_mode {
                DockMode::Bottom => {
                    zoon::println!("Updating bottom layout files_panel_width to: {}", width);
                    layouts.docked_to_bottom.lock_ref().files_panel_width.set_neq(width as f32);
                }
                DockMode::Right => {
                    zoon::println!("Updating right layout files_panel_width to: {}", width);
                    layouts.docked_to_right.lock_ref().files_panel_width.set_neq(width as f32);
                }
            }
        }).await
    });

    Task::start(async {
        FILES_PANEL_HEIGHT.signal().for_each_sync(|height| {
            let dock_mode = config_store().workspace.lock_ref().dock_mode.get_cloned();
            let workspace_ref = config_store().workspace.lock_ref();
            let layouts = workspace_ref.panel_layouts.lock_ref();
            
            match dock_mode {
                DockMode::Bottom => {
                    layouts.docked_to_bottom.lock_ref().files_panel_height.set_neq(height as f32);
                }
                DockMode::Right => {
                    layouts.docked_to_right.lock_ref().files_panel_height.set_neq(height as f32);
                }
            }
        }).await
    });

    // Sync expanded scopes back to config (convert HashSet to Vec)
    Task::start(async {
        EXPANDED_SCOPES.signal_ref(|expanded_set| {
            expanded_set.clone()
        }).for_each_sync(|expanded_set| {
            let expanded_vec: Vec<String> = expanded_set.into_iter().collect();
            config_store().workspace.lock_ref().expanded_scopes.lock_mut().replace_cloned(expanded_vec);
            // Manually trigger config save since MutableVec reactive signals are complex
            save_config_to_backend();
        }).await
    });

    // Sync selected scope back to config
    Task::start(async {
        SELECTED_SCOPE_ID.signal_cloned().for_each_sync(|scope_id| {
            config_store().workspace.lock_mut().selected_scope_id.set_neq(scope_id);
            // Manually trigger config save for scope selection changes
            save_config_to_backend();
        }).await
    });

    // Sync variables search filter back to config
    Task::start(async {
        VARIABLES_SEARCH_FILTER.signal_cloned().for_each_sync(|filter| {
            config_store().session.lock_mut().variables_search_filter.set_neq(filter);
        }).await
    });

    // Sync dialog states back to config
    Task::start(async {
        SHOW_FILE_DIALOG.signal().for_each_sync(|show| {
            config_store().dialogs.lock_mut().show_file_dialog.set_neq(show);
        }).await
    });

    Task::start(async {
        FILE_PATHS_INPUT.signal_cloned().for_each_sync(|input| {
            config_store().dialogs.lock_mut().file_paths_input.set_neq(input);
        }).await
    });
}

// =============================================================================
// THEME SYNCHRONIZATION - Keep NovyUI theme in sync with ConfigStore
// =============================================================================

pub fn sync_theme_to_novyui() {
    Task::start(async {
        current_theme().for_each_sync(|config_theme| {
            // Convert config theme to NovyUI theme and update
            let novyui_theme = match config_theme {
                Theme::Light => moonzoon_novyui::tokens::theme::Theme::Light,
                Theme::Dark => moonzoon_novyui::tokens::theme::Theme::Dark,
            };
            
            // Update NovyUI theme without triggering the callback
            // (to avoid circular updates since our callback updates the config)
            moonzoon_novyui::tokens::theme::set_theme_without_callback(novyui_theme);
        }).await
    })
}