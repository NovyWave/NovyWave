// New Unified Config System
// Based on ringrev_private patterns: Big struct with nested Mutable values and triggers

use zoon::*;
use serde::{Deserialize, Serialize};
use shared::UpMsg;
pub use shared::{Theme, DockMode}; // Re-export for frontend usage

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
    pub toast_dismiss_ms: Mutable<u64>,
}

// Theme and DockMode enums now imported from shared crate for type safety

// DockMode enum now imported from shared crate for type safety

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
    pub scroll_position: Mutable<i32>,
}

// =============================================================================
// WORKSPACE SECTION - Layout and Panel State
// =============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkspaceSection {
    pub dock_mode: Mutable<DockMode>,
    pub selected_scope_id: Mutable<Option<String>>,
    pub expanded_scopes: MutableVec<String>,
    pub load_files_expanded_directories: MutableVec<String>,
    pub panel_layouts: Mutable<PanelLayouts>,
}

// DockMode enum now imported from shared crate for type safety

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
            toast_dismiss_ms: Mutable::new(10000), // Default 10 seconds
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
            scroll_position: Mutable::new(0),
        }
    }
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            dock_mode: Mutable::new(DockMode::Bottom),
            selected_scope_id: Mutable::new(None),
            expanded_scopes: MutableVec::new(),
            load_files_expanded_directories: MutableVec::new(),
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
    #[serde(default = "default_toast_dismiss_ms")]
    pub toast_dismiss_ms: u64,
}

fn default_toast_dismiss_ms() -> u64 {
    10000 // Default 10 seconds
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
    pub scroll_position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableWorkspaceSection {
    pub dock_mode: DockMode,
    pub selected_scope_id: Option<String>,
    pub expanded_scopes: Vec<String>,
    pub load_files_expanded_directories: Vec<String>,
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
                toast_dismiss_ms: self.ui.lock_ref().toast_dismiss_ms.get(),
            },
            session: SerializableSessionSection {
                opened_files: self.session.lock_ref().opened_files.lock_ref().to_vec(),
                variables_search_filter: self.session.lock_ref().variables_search_filter.get_cloned(),
                file_picker: SerializableFilePickerSection {
                    current_directory: self.session.lock_ref().file_picker.lock_ref().current_directory.get_cloned(),
                    expanded_directories: self.session.lock_ref().file_picker.lock_ref().expanded_directories.lock_ref().to_vec(),
                    show_hidden_files: self.session.lock_ref().file_picker.lock_ref().show_hidden_files.get(),
                    scroll_position: self.session.lock_ref().file_picker.lock_ref().scroll_position.get(),
                },
            },
            workspace: SerializableWorkspaceSection {
                dock_mode: self.workspace.lock_ref().dock_mode.get_cloned(),
                selected_scope_id: self.workspace.lock_ref().selected_scope_id.get_cloned(),
                expanded_scopes: self.workspace.lock_ref().expanded_scopes.lock_ref().to_vec(),
                load_files_expanded_directories: self.workspace.lock_ref().load_files_expanded_directories.lock_ref().to_vec(),
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
        self.ui.lock_mut().toast_dismiss_ms.set(config.ui.toast_dismiss_ms);

        // Load session section
        self.session.lock_mut().opened_files.lock_mut().replace_cloned(config.session.opened_files);
        self.session.lock_mut().variables_search_filter.set(config.session.variables_search_filter);
        
        {
            let session_ref = self.session.lock_ref();
            let file_picker = session_ref.file_picker.lock_ref();
            file_picker.current_directory.set(config.session.file_picker.current_directory);
            file_picker.expanded_directories.lock_mut().replace_cloned(config.session.file_picker.expanded_directories);
            file_picker.show_hidden_files.set(config.session.file_picker.show_hidden_files);
            file_picker.scroll_position.set(config.session.file_picker.scroll_position);
        }

        // Load workspace section
        self.workspace.lock_mut().dock_mode.set(config.workspace.dock_mode);
        self.workspace.lock_mut().selected_scope_id.set(config.workspace.selected_scope_id);
        self.workspace.lock_mut().expanded_scopes.lock_mut().replace_cloned(config.workspace.expanded_scopes);
        self.workspace.lock_mut().load_files_expanded_directories.lock_mut().replace_cloned(config.workspace.load_files_expanded_directories);

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
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    save_config_to_backend();
            } else {
                }
        }).await
    });
    
    // Dock mode changes - get signal and drop lock immediately  
    let dock_mode_signal = {
        let workspace = config_store().workspace.lock_ref();
        workspace.dock_mode.signal_cloned()
    };
    Task::start(async move {
        dock_mode_signal.for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    save_config_to_backend();
            } else {
                }
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
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    save_config_to_backend();
            } else {
                }
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
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    save_config_to_backend();
            } else {
                }
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
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    save_config_to_backend();
            } else {
                }
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
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    save_config_to_backend();
            } else {
                }
        }).await
    });
}

/// Backend-compatible panel dimensions with only the 2 fields that exist in shared schema
/// This ensures serde only serializes the fields that exist in backend
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackendPanelDimensions {
    files_panel_width: f64,
    files_panel_height: f64,
    // NOTE: variables_panel_width and timeline_panel_height are frontend-only
    // They are NOT included here to prevent serialization corruption
}

impl From<SerializablePanelDimensions> for BackendPanelDimensions {
    fn from(dimensions: SerializablePanelDimensions) -> Self {
        Self {
            files_panel_width: dimensions.files_panel_width as f64,
            files_panel_height: dimensions.files_panel_height as f64,
        }
    }
}

fn save_config_to_backend() {
    use crate::connection::send_up_msg;
    
    // Convert to serializable format using existing infrastructure
    let serializable_config = config_store().to_serializable();
    
    // Convert panel dimensions using declarative type conversion
    let backend_docked_to_bottom = BackendPanelDimensions::from(serializable_config.workspace.panel_layouts.docked_to_bottom);
    let backend_docked_to_right = BackendPanelDimensions::from(serializable_config.workspace.panel_layouts.docked_to_right);
    
    // Build shared::AppConfig using backend-compatible data
    let app_config = shared::AppConfig {
        app: shared::AppSection {
            version: serializable_config.app.version,
        },
        ui: shared::UiSection {
            theme: serializable_config.ui.theme,
            toast_dismiss_ms: serializable_config.ui.toast_dismiss_ms,
        },
        workspace: shared::WorkspaceSection {
            opened_files: serializable_config.session.opened_files,
            panel_dimensions_bottom: shared::PanelDimensions::new(
                backend_docked_to_bottom.files_panel_width,
                backend_docked_to_bottom.files_panel_height
            ),
            panel_dimensions_right: shared::PanelDimensions::new(
                backend_docked_to_right.files_panel_width,
                backend_docked_to_right.files_panel_height
            ),
            dock_mode: serializable_config.workspace.dock_mode,
            expanded_scopes: serializable_config.workspace.expanded_scopes,
            load_files_expanded_directories: serializable_config.workspace.load_files_expanded_directories,
            selected_scope_id: serializable_config.workspace.selected_scope_id,
            load_files_scroll_position: serializable_config.session.file_picker.scroll_position,
            variables_search_filter: serializable_config.session.variables_search_filter,
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

#[allow(dead_code)]
pub fn save_current_config() {
    // This is now handled automatically by the reactive triggers
    // The new system auto-saves when config changes
}

pub fn save_file_list() {
    // Enhanced approach: use TRACKED_FILES system instead of legacy FILE_PATHS
    use crate::state::{TRACKED_FILES, get_all_tracked_file_paths};
    
    // Get all tracked file paths (preserves order and includes all file states)
    let file_paths = get_all_tracked_file_paths();
    config_store().session.lock_mut().opened_files.lock_mut().replace_cloned(file_paths);
    
    // Also maintain legacy FILE_PATHS for backward compatibility during transition
    use crate::state::FILE_PATHS;
    let tracked_files = TRACKED_FILES.lock_ref();
    let mut legacy_file_paths = FILE_PATHS.lock_mut();
    legacy_file_paths.clear();
    
    for tracked_file in tracked_files.iter() {
        legacy_file_paths.insert(tracked_file.id.clone(), tracked_file.path.clone());
    }
    
    // Manually trigger config save since MutableVec reactive signals are complex
    if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
        save_config_to_backend();
    }
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
    // NOTE: Set CONFIG_LOADED at the END to prevent race condition with lazy initialization
    let serializable_config = SerializableConfig {
        app: SerializableAppSection {
            version: config.app.version,
            migration_strategy: MigrationStrategy::None,  // Default since shared doesn't have this field
        },
        ui: SerializableUiSection {
            theme: config.ui.theme,
            font_size: 14.0,
            show_tooltips: true,
            toast_dismiss_ms: config.ui.toast_dismiss_ms,
        },
        session: SerializableSessionSection {
            opened_files: config.workspace.opened_files,
            variables_search_filter: config.workspace.variables_search_filter,
            file_picker: SerializableFilePickerSection {
                current_directory: None,
                expanded_directories: config.workspace.load_files_expanded_directories.clone(),
                show_hidden_files: false,
                scroll_position: config.workspace.load_files_scroll_position,
            },
        },
        workspace: SerializableWorkspaceSection {
            dock_mode: config.workspace.dock_mode,
            selected_scope_id: config.workspace.selected_scope_id,
            expanded_scopes: config.workspace.expanded_scopes,
            load_files_expanded_directories: config.workspace.load_files_expanded_directories,
            panel_layouts: SerializablePanelLayouts {
                docked_to_bottom: SerializablePanelDimensions {
                    files_panel_width: config.workspace.panel_dimensions_bottom.width as f32,
                    files_panel_height: config.workspace.panel_dimensions_bottom.height as f32,
                    // Backend schema doesn't include these fields - use frontend defaults to prevent corruption
                    variables_panel_width: 300.0,  // Frontend-only default
                    timeline_panel_height: 200.0,  // Frontend-only default
                },
                docked_to_right: SerializablePanelDimensions {
                    files_panel_width: config.workspace.panel_dimensions_right.width as f32,
                    files_panel_height: config.workspace.panel_dimensions_right.height as f32,
                    // Backend schema doesn't include these fields - use frontend defaults to prevent corruption
                    variables_panel_width: 250.0,  // Frontend-only default
                    timeline_panel_height: 150.0,  // Frontend-only default
                },
            },
        },
        dialogs: SerializableDialogSection {
            show_file_dialog: false,  // Don't auto-open via config (use session state instead)
            show_settings_dialog: false,
            show_about_dialog: false,
            file_paths_input: String::new(),
        },
    };

    config_store().load_from_serializable(serializable_config);
    
    // Manual sync of expanded_scopes from config to signal (Vec<String> to HashSet<String>)
    sync_expanded_scopes_from_config();
    
    // Manual sync of load_files_expanded_directories from config to signal (Vec<String> to HashSet<String>)
    sync_load_files_expanded_directories_from_config();
    
    // Manual sync of opened_files from config to legacy globals
    sync_opened_files_from_config();
    
    // Manual sync of file picker current directory from config to legacy globals
    sync_file_picker_current_directory_from_config();
    
    // Manual sync of scroll position from config to legacy globals
    sync_load_files_scroll_position_from_config();
    
    // Set config loaded flag
    CONFIG_LOADED.set_neq(true);
    
    // Mark initialization complete to allow reactive config saves
    crate::CONFIG_INITIALIZATION_COMPLETE.set_neq(true);
    
    // Note: sync_globals_to_config() is called later in main.rs after CONFIG_LOADED signal
    // to ensure proper timing when UI components are fully initialized
}

// =============================================================================
// HELPER FUNCTIONS - Common config operations
// =============================================================================

pub fn current_theme() -> impl Signal<Item = Theme> {
    config_store().ui.signal_ref(|ui| ui.theme.signal_cloned()).flatten()
}

/// Get current toast dismiss time
pub fn current_toast_dismiss_ms() -> u64 {
    config_store().ui.lock_ref().toast_dismiss_ms.get()
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

// Manual sync function to convert load_files_expanded_directories from Vec<String> to IndexSet<String>
fn sync_load_files_expanded_directories_from_config() {
    use crate::state::FILE_PICKER_EXPANDED;
    use indexmap::IndexSet;
    
    let expanded_vec = config_store().workspace.lock_ref().load_files_expanded_directories.lock_ref().to_vec();
    
    // In WASM, trust the backend-validated directories (no filesystem access)
    let new_expanded_set: IndexSet<String> = expanded_vec.into_iter().collect();
    
    
    // Apply the complete set atomically to prevent reactive race conditions
    *FILE_PICKER_EXPANDED.lock_mut() = new_expanded_set;
}

// Enhanced sync function to restore opened_files from config using TRACKED_FILES system
fn sync_opened_files_from_config() {
    use crate::state::{init_tracked_files_from_config, FILE_PATHS};
    use crate::send_up_msg;
    
    let opened_files = config_store().session.lock_ref().opened_files.lock_ref().to_vec();
    
    // Initialize TRACKED_FILES system with config file paths
    init_tracked_files_from_config(opened_files.clone());
    
    // Also maintain legacy FILE_PATHS for backward compatibility during transition
    FILE_PATHS.lock_mut().clear();
    
    // Restore each file path and reload the file
    for file_path in opened_files {
        // Generate file ID and store in FILE_PATHS (same pattern as file loading)
        let file_id = shared::generate_file_id(&file_path);
        FILE_PATHS.lock_mut().insert(file_id, file_path.clone());
        
        // Add to TRACKED_FILES system with loading state
        crate::state::add_tracked_file(file_path.clone(), shared::FileState::Loading(shared::LoadingStatus::Starting));
        
        // Reload the file
        send_up_msg(shared::UpMsg::LoadWaveformFile(file_path));
    }
}

// Manual sync function to restore file picker current directory from config 
fn sync_file_picker_current_directory_from_config() {
    use crate::state::CURRENT_DIRECTORY;
    
    let current_dir = config_store().session.lock_ref().file_picker.lock_ref().current_directory.get_cloned();
    
    // Restore current directory if it exists in config
    if let Some(directory) = current_dir {
        // Validate directory exists before restoring
        if std::path::Path::new(&directory).is_dir() {
            CURRENT_DIRECTORY.set_neq(directory);
        } else {
                // Clear invalid directory from config
            config_store().session.lock_ref().file_picker.lock_ref().current_directory.set_neq(None);
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }
    }
}

// Manual sync function to restore scroll position from config to legacy globals
fn sync_load_files_scroll_position_from_config() {
    use crate::state::LOAD_FILES_SCROLL_POSITION;
    
    let saved_scroll_position = config_store().session.lock_ref().file_picker.lock_ref().scroll_position.get();
    
    // Restore the scroll position to both persistent globals to prevent viewport lazy initialization with 0
    LOAD_FILES_SCROLL_POSITION.set_neq(saved_scroll_position);
    crate::LOAD_FILES_VIEWPORT_Y.set_neq(saved_scroll_position);
}

pub fn current_dock_mode() -> impl Signal<Item = DockMode> {
    config_store().workspace.signal_ref(|ws| ws.dock_mode.signal_cloned()).flatten()
}

#[allow(dead_code)]
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

    // Sync load files scroll position
    Task::start(async {
        config_store().session.signal_ref(|s| s.file_picker.signal_ref(|fp| fp.scroll_position.signal()).flatten()).flatten()
            .for_each_sync(|scroll_pos| {
                // Only sync during runtime, not during initialization
                if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    LOAD_FILES_SCROLL_POSITION.set_neq(scroll_pos);
                }
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
                    let dock_mode = config_store().workspace.lock_ref().dock_mode.get_cloned();
            let workspace_ref = config_store().workspace.lock_ref();
            let layouts = workspace_ref.panel_layouts.lock_ref();
            
            match dock_mode {
                DockMode::Bottom => {
                                layouts.docked_to_bottom.lock_ref().files_panel_width.set_neq(width as f32);
                }
                DockMode::Right => {
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
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            } else {
                }
        }).await
    });

    // Sync load files expanded directories back to config (convert IndexSet to Vec)
    Task::start(async {
        FILE_PICKER_EXPANDED.signal_ref(|expanded_set| {
            expanded_set.clone()
        }).for_each_sync(|expanded_set| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                let expanded_vec: Vec<String> = expanded_set.into_iter().collect();
                    config_store().workspace.lock_ref().load_files_expanded_directories.lock_mut().replace_cloned(expanded_vec);
                // Manually trigger config save since MutableVec reactive signals are complex
                save_config_to_backend();
            } else {
                }
        }).await
    });

    // Sync selected scope back to config
    Task::start(async {
        SELECTED_SCOPE_ID.signal_cloned().for_each_sync(|scope_id| {
            config_store().workspace.lock_mut().selected_scope_id.set_neq(scope_id);
            // Manually trigger config save for scope selection changes
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            } else {
                }
        }).await
    });

    // Sync file picker current directory back to config
    Task::start(async {
        CURRENT_DIRECTORY.signal_cloned().for_each_sync(|current_dir| {
            // Only save non-empty directories
            let dir_to_save = if current_dir.is_empty() { None } else { Some(current_dir) };
            config_store().session.lock_ref().file_picker.lock_ref().current_directory.set_neq(dir_to_save);
            // Manually trigger config save for current directory changes
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            } else {
                }
        }).await
    });

    // Sync variables search filter back to config
    Task::start(async {
        VARIABLES_SEARCH_FILTER.signal_cloned().for_each_sync(|filter| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().session.lock_mut().variables_search_filter.set_neq(filter);
                // Manually trigger config save for search filter changes
                save_config_to_backend();
            } else {
            }
        }).await
    });

    // Sync load files scroll position back to config
    Task::start(async {
        LOAD_FILES_SCROLL_POSITION.signal().for_each_sync(|scroll_pos| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                    // Validate scroll position is within bounds [0, 10000]
                let validated_pos = scroll_pos.max(0).min(10000);
                config_store().session.lock_ref().file_picker.lock_ref().scroll_position.set_neq(validated_pos);
                // Manually trigger config save for scroll position changes
                save_config_to_backend();
            } else {
                }
        }).await
    });

    // Sync viewport scroll changes back to persistent scroll position
    Task::start(async {
        LOAD_FILES_VIEWPORT_Y.signal().for_each_sync(|viewport_y| {
                // Only sync during runtime, not during initialization
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                // Update the persistent scroll position when user scrolls the viewport
                // This ensures manual scrolling is also saved
                LOAD_FILES_SCROLL_POSITION.set_neq(viewport_y);
            } else {
                }
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