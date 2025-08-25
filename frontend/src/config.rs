// New Unified Config System
// Based on ringrev_private patterns: Big struct with nested Mutable values and triggers

use zoon::*;
use serde::{Deserialize, Serialize};
use shared::UpMsg;
pub use shared::{Theme, DockMode}; // Re-export for frontend usage
use crate::CONFIG_INITIALIZATION_COMPLETE;

// Reactive triggers module
pub mod triggers;

// Timeline validation constants
const MIN_VALID_RANGE: f32 = 1e-6;       // 1 microsecond minimum range
const SAFE_FALLBACK_START: f32 = 0.0;    // Safe fallback start time
const SAFE_FALLBACK_END: f32 = 100.0;    // Safe fallback end time

/// Validate timeline values from config to prevent NaN propagation
fn validate_timeline_values(cursor: f64, zoom: f32, start: f32, end: f32) -> (f64, f32, f32, f32) {
    // Validate cursor position
    let safe_cursor = if cursor.is_finite() && cursor >= 0.0 { cursor } else { 50.0 };
    
    // Validate zoom level
    let safe_zoom = if zoom.is_finite() && zoom >= 1.0 && zoom <= 1e9 { zoom } else { 1.0 };
    
    // Validate range
    let (safe_start, safe_end) = if start.is_finite() && end.is_finite() && start < end && (end - start) >= MIN_VALID_RANGE {
        (start, end)
    } else {
        crate::debug_utils::debug_critical(&format!("CONFIG: Invalid timeline range ({}, {}), using fallback", start, end));
        (SAFE_FALLBACK_START, SAFE_FALLBACK_END)
    };
    
    (safe_cursor, safe_zoom, safe_start, safe_end)
}

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
    pub selected_variables: MutableVec<shared::SelectedVariable>,
    pub timeline_cursor_position: Mutable<f64>,
    pub timeline_zoom_level: Mutable<f32>,
    pub timeline_visible_range_start: Mutable<f32>,
    pub timeline_visible_range_end: Mutable<f32>,
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
    pub variables_name_column_width: Mutable<f32>,
    pub variables_value_column_width: Mutable<f32>,
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
pub static CONFIG_LOADED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false)); // Start false, set true after config loads

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
            selected_variables: MutableVec::new(),
            timeline_cursor_position: Mutable::new(10.0),
            timeline_zoom_level: Mutable::new(1.0),
            timeline_visible_range_start: Mutable::new(0.0),
            timeline_visible_range_end: Mutable::new(100.0),
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
                variables_name_column_width: Mutable::new(180.0),
                variables_value_column_width: Mutable::new(100.0),
            }),
            docked_to_right: Mutable::new(PanelDimensions {
                files_panel_width: Mutable::new(400.0),
                files_panel_height: Mutable::new(300.0),
                variables_panel_width: Mutable::new(250.0),
                timeline_panel_height: Mutable::new(150.0),
                variables_name_column_width: Mutable::new(180.0),
                variables_value_column_width: Mutable::new(100.0),
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
    pub selected_variables: Vec<shared::SelectedVariable>,
    pub timeline_cursor_position: f64,
    pub timeline_zoom_level: f32,
    pub timeline_visible_range_start: f32,
    pub timeline_visible_range_end: f32,
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
    pub variables_name_column_width: f32,
    pub variables_value_column_width: f32,
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
                selected_variables: self.workspace.lock_ref().selected_variables.lock_ref().to_vec(),
                panel_layouts: SerializablePanelLayouts {
                    docked_to_bottom: SerializablePanelDimensions {
                        files_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().files_panel_width.get(),
                        files_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().files_panel_height.get(),
                        variables_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().variables_panel_width.get(),
                        timeline_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().timeline_panel_height.get(),
                        variables_name_column_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().variables_name_column_width.get(),
                        variables_value_column_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_bottom.lock_ref().variables_value_column_width.get(),
                    },
                    docked_to_right: SerializablePanelDimensions {
                        files_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().files_panel_width.get(),
                        files_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().files_panel_height.get(),
                        variables_panel_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().variables_panel_width.get(),
                        timeline_panel_height: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().timeline_panel_height.get(),
                        variables_name_column_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().variables_name_column_width.get(),
                        variables_value_column_width: self.workspace.lock_ref().panel_layouts.lock_ref().docked_to_right.lock_ref().variables_value_column_width.get(),
                    },
                },
                timeline_cursor_position: self.workspace.lock_ref().timeline_cursor_position.get(),
                timeline_zoom_level: self.workspace.lock_ref().timeline_zoom_level.get(),
                timeline_visible_range_start: self.workspace.lock_ref().timeline_visible_range_start.get(),
                timeline_visible_range_end: self.workspace.lock_ref().timeline_visible_range_end.get(),
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
        self.workspace.lock_mut().selected_variables.lock_mut().replace_cloned(config.workspace.selected_variables);
        
        // Validate timeline values before setting to prevent NaN propagation
        let (safe_cursor, safe_zoom, safe_start, safe_end) = validate_timeline_values(
            config.workspace.timeline_cursor_position,
            config.workspace.timeline_zoom_level,
            config.workspace.timeline_visible_range_start,
            config.workspace.timeline_visible_range_end
        );
        
        self.workspace.lock_mut().timeline_cursor_position.set(safe_cursor);
        self.workspace.lock_mut().timeline_zoom_level.set(safe_zoom);
        self.workspace.lock_mut().timeline_visible_range_start.set(safe_start);
        self.workspace.lock_mut().timeline_visible_range_end.set(safe_end);

        {
            let workspace_ref = self.workspace.lock_ref();
            let panel_layouts = workspace_ref.panel_layouts.lock_ref();
            
            {
                let bottom_dims = panel_layouts.docked_to_bottom.lock_ref();
                bottom_dims.files_panel_width.set(config.workspace.panel_layouts.docked_to_bottom.files_panel_width);
                bottom_dims.files_panel_height.set(config.workspace.panel_layouts.docked_to_bottom.files_panel_height);
                bottom_dims.variables_panel_width.set(config.workspace.panel_layouts.docked_to_bottom.variables_panel_width);
                bottom_dims.timeline_panel_height.set(config.workspace.panel_layouts.docked_to_bottom.timeline_panel_height);
                bottom_dims.variables_name_column_width.set(config.workspace.panel_layouts.docked_to_bottom.variables_name_column_width);
                bottom_dims.variables_value_column_width.set(config.workspace.panel_layouts.docked_to_bottom.variables_value_column_width);
            }

            {
                let right_dims = panel_layouts.docked_to_right.lock_ref();
                right_dims.files_panel_width.set(config.workspace.panel_layouts.docked_to_right.files_panel_width);
                right_dims.files_panel_height.set(config.workspace.panel_layouts.docked_to_right.files_panel_height);
                right_dims.variables_panel_width.set(config.workspace.panel_layouts.docked_to_right.variables_panel_width);
                right_dims.timeline_panel_height.set(config.workspace.panel_layouts.docked_to_right.timeline_panel_height);
                right_dims.variables_name_column_width.set(config.workspace.panel_layouts.docked_to_right.variables_name_column_width);
                right_dims.variables_value_column_width.set(config.workspace.panel_layouts.docked_to_right.variables_value_column_width);
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
    
    // Variables column width changes - bottom dock mode
    let bottom_name_column_width_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let bottom = layouts.docked_to_bottom.lock_ref();
        bottom.variables_name_column_width.signal()
    };
    Task::start(async move {
        bottom_name_column_width_signal.for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    let bottom_value_column_width_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let bottom = layouts.docked_to_bottom.lock_ref();
        bottom.variables_value_column_width.signal()
    };
    Task::start(async move {
        bottom_value_column_width_signal.for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    // Variables column width changes - right dock mode
    let right_name_column_width_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let right = layouts.docked_to_right.lock_ref();
        right.variables_name_column_width.signal()
    };
    Task::start(async move {
        right_name_column_width_signal.for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    let right_value_column_width_signal = {
        let workspace = config_store().workspace.lock_ref();
        let layouts = workspace.panel_layouts.lock_ref();
        let right = layouts.docked_to_right.lock_ref();
        right.variables_value_column_width.signal()
    };
    Task::start(async move {
        right_value_column_width_signal.for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    // Timeline cursor position changes - TRUE DEBOUNCING with droppable tasks
    let timeline_cursor_position_signal = crate::state::TIMELINE_CURSOR_POSITION.signal();
    Task::start(async move {
        let debounce_task: Mutable<Option<TaskHandle>> = Mutable::new(None);
        
        timeline_cursor_position_signal
            .dedupe() // Skip duplicate values  
            .for_each_sync(move |_| {
                // Drop any existing debounce task (true cancellation)
                debounce_task.set(None);
                
                // Start a new droppable debounce timer
                let new_handle = Task::start_droppable(async {
                    Timer::sleep(1000).await; // 1 second of inactivity
                    if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                        save_config_to_backend();
                    }
                });
                
                debounce_task.set(Some(new_handle));
            })
            .await;
    });
    
    // Timeline zoom level changes - DISABLED to prevent backend flooding during smooth operations  
    // Zoom changes will be saved when other config changes occur
    // COMMENTED OUT - This signal may also cause flooding:
    // let timeline_zoom_level_signal = crate::state::TIMELINE_ZOOM_LEVEL.signal();
    
    // Timeline visible range changes - DISABLED to prevent backend flooding
    // Range changes are too frequent during smooth operations and don't need immediate persistence
    // They will be saved when other config changes occur (cursor position, zoom, etc.)
    
    // COMMENTED OUT - These signals were causing the backend flooding:
    // let timeline_visible_range_start_signal = crate::state::TIMELINE_VISIBLE_RANGE_START.signal();
    // let timeline_visible_range_end_signal = crate::state::TIMELINE_VISIBLE_RANGE_END.signal();
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

// Global debouncing for config saves to prevent backend flooding
static SAVE_CONFIG_PENDING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

pub fn save_config_to_backend() {
    // CRITICAL: Debounce all config saves to prevent backend flooding
    if !SAVE_CONFIG_PENDING.get() {
        SAVE_CONFIG_PENDING.set_neq(true);
        Task::start(async {
            Timer::sleep(1000).await; // AGGRESSIVE: 1 second debounce to prevent flooding
            save_config_immediately(); // Execute save FIRST
            SAVE_CONFIG_PENDING.set_neq(false); // Clear flag AFTER save completes
        });
    }
}

fn save_config_immediately() {
    use crate::platform::{Platform, CurrentPlatform};
    
    // Convert to serializable format using existing infrastructure
    let serializable_config = config_store().to_serializable();
    
    // Extract panel layouts first to avoid borrow checker issues
    let bottom_layout = serializable_config.workspace.panel_layouts.docked_to_bottom;
    let right_layout = serializable_config.workspace.panel_layouts.docked_to_right;
    
    // Convert panel dimensions using declarative type conversion
    let backend_docked_to_bottom = BackendPanelDimensions::from(bottom_layout.clone());
    let backend_docked_to_right = BackendPanelDimensions::from(right_layout.clone());
    
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
            docked_bottom_dimensions: shared::DockedBottomDimensions {
                files_and_scopes_panel_width: backend_docked_to_bottom.files_panel_width,
                files_and_scopes_panel_height: backend_docked_to_bottom.files_panel_height,
                selected_variables_panel_name_column_width: Some(bottom_layout.variables_name_column_width as f64),
                selected_variables_panel_value_column_width: Some(bottom_layout.variables_value_column_width as f64),
            },
            docked_right_dimensions: shared::DockedRightDimensions {
                files_and_scopes_panel_width: backend_docked_to_right.files_panel_width,
                files_and_scopes_panel_height: backend_docked_to_right.files_panel_height,
                selected_variables_panel_name_column_width: Some(right_layout.variables_name_column_width as f64),
                selected_variables_panel_value_column_width: Some(right_layout.variables_value_column_width as f64),
            },
            dock_mode: serializable_config.workspace.dock_mode,
            expanded_scopes: serializable_config.workspace.expanded_scopes,
            load_files_expanded_directories: serializable_config.workspace.load_files_expanded_directories,
            selected_scope_id: serializable_config.workspace.selected_scope_id,
            load_files_scroll_position: serializable_config.session.file_picker.scroll_position,
            variables_search_filter: serializable_config.session.variables_search_filter,
            selected_variables: serializable_config.workspace.selected_variables,
            timeline_cursor_position: {
                let pos = crate::state::TIMELINE_CURSOR_POSITION.get();
                if pos.is_finite() && pos >= 0.0 { pos } else { 10.0 } // Default to 10.0s if invalid
            },
            timeline_zoom_level: {
                let zoom = crate::state::TIMELINE_ZOOM_LEVEL.get();
                if zoom.is_finite() && zoom >= 1.0 { zoom } else { 1.0 } // Default to 1.0 if invalid
            },
            timeline_visible_range_start: {
                let start = crate::state::TIMELINE_VISIBLE_RANGE_START.get();
                if start.is_finite() && start >= 0.0 {
                    Some(start)
                } else {
                    None  // Send None instead of Some(NaN) to prevent JSON deserialization error
                }
            },
            timeline_visible_range_end: {
                let end = crate::state::TIMELINE_VISIBLE_RANGE_END.get();
                if end.is_finite() && end > 0.0 {
                    Some(end)
                } else {
                    None  // Send None instead of Some(NaN) to prevent JSON deserialization error
                }
            },
        },
    };

    // Use platform abstraction instead of direct connection
    Task::start(async move {
        if let Err(e) = CurrentPlatform::send_message(UpMsg::SaveConfig(app_config)).await {
            zoon::println!("ERROR: Failed to save config via platform: {}", e);
        }
    });
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
    if CONFIG_INITIALIZATION_COMPLETE.get() {
        save_config_to_backend();
    }
}

pub fn save_panel_layout() {
    if CONFIG_INITIALIZATION_COMPLETE.get() {
        save_config_to_backend();
    }
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
            selected_variables: config.workspace.selected_variables,
            timeline_cursor_position: config.workspace.timeline_cursor_position,
            timeline_zoom_level: config.workspace.timeline_zoom_level,
            timeline_visible_range_start: config.workspace.timeline_visible_range_start.unwrap_or(0.0),
            timeline_visible_range_end: config.workspace.timeline_visible_range_end.unwrap_or(100.0),
            panel_layouts: SerializablePanelLayouts {
                docked_to_bottom: SerializablePanelDimensions {
                    files_panel_width: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_width as f32,
                    files_panel_height: config.workspace.docked_bottom_dimensions.files_and_scopes_panel_height as f32,
                    // Backend schema doesn't include these fields - use frontend defaults to prevent corruption
                    variables_panel_width: 300.0,  // Frontend-only default
                    timeline_panel_height: 200.0,  // Frontend-only default
                    variables_name_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_name_column_width.unwrap_or(180.0) as f32,
                    variables_value_column_width: config.workspace.docked_bottom_dimensions.selected_variables_panel_value_column_width.unwrap_or(100.0) as f32,
                },
                docked_to_right: SerializablePanelDimensions {
                    files_panel_width: config.workspace.docked_right_dimensions.files_and_scopes_panel_width as f32,
                    files_panel_height: config.workspace.docked_right_dimensions.files_and_scopes_panel_height as f32,
                    // Backend schema doesn't include these fields - use frontend defaults to prevent corruption
                    variables_panel_width: 250.0,  // Frontend-only default
                    timeline_panel_height: 150.0,  // Frontend-only default
                    variables_name_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_name_column_width.unwrap_or(180.0) as f32,
                    variables_value_column_width: config.workspace.docked_right_dimensions.selected_variables_panel_value_column_width.unwrap_or(100.0) as f32,
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
    
    // One-shot config initialization - runs once after config loads
    triggers::setup_one_time_config_sync();
    
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

// populate_globals_from_config() function removed - replaced by reactive triggers
// All global state is now synced automatically through triggers::setup_reactive_config_system()

// =============================================================================
// SIGNALS - Helper functions for reactive config values
// =============================================================================













// =============================================================================
// REVERSE SYNC - Update ConfigStore when state.rs globals change
// =============================================================================

/// Set up reactive config persistence - observes global state and automatically saves config
pub fn setup_reactive_config_persistence() {
    use crate::state::*;

    // Initialize derived signals first
    crate::state::init_config_derived_signals();

    // Observe OPENED_FILES_FOR_CONFIG and update config store
    Task::start(async {
        OPENED_FILES_FOR_CONFIG.signal_cloned().for_each(|file_paths| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().session.lock_mut().opened_files.lock_mut().replace_cloned(file_paths);
                save_config_to_backend();
            }
        }).await
    });

    // Observe EXPANDED_SCOPES_FOR_CONFIG and update config store
    Task::start(async {
        EXPANDED_SCOPES_FOR_CONFIG.signal_cloned().for_each(|expanded_scopes| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().expanded_scopes.lock_mut().replace_cloned(expanded_scopes);
                save_config_to_backend();
            }
        }).await
    });

    // Observe LOAD_FILES_EXPANDED_DIRECTORIES_FOR_CONFIG and update config store
    Task::start(async {
        LOAD_FILES_EXPANDED_DIRECTORIES_FOR_CONFIG.signal_cloned().for_each(|expanded_dirs| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().load_files_expanded_directories.lock_mut().replace_cloned(expanded_dirs);
                save_config_to_backend();
            }
        }).await
    });

    // Observe SELECTED_VARIABLES_FOR_CONFIG and update config store
    Task::start(async {
        SELECTED_VARIABLES_FOR_CONFIG.signal_cloned().for_each(|selected_vars| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().selected_variables.lock_mut().replace_cloned(selected_vars);
                save_config_to_backend();
            }
        }).await
    });

    // Observe DOCK_MODE_FOR_CONFIG and update config store
    Task::start(async {
        DOCK_MODE_FOR_CONFIG.signal_cloned().for_each(|dock_mode| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().dock_mode.set_neq(dock_mode);
                save_config_to_backend();
            }
        }).await
    });

    // Observe SELECTED_SCOPE_ID and update config store
    Task::start(async {
        SELECTED_SCOPE_ID.signal_cloned().for_each(|scope_id| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().selected_scope_id.set_neq(scope_id);
                save_config_to_backend();
            }
        }).await
    });

    // Observe VARIABLES_SEARCH_FILTER and update config store
    Task::start(async {
        VARIABLES_SEARCH_FILTER.signal_cloned().for_each(|filter| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().session.lock_mut().variables_search_filter.set_neq(filter);
                save_config_to_backend();
            }
        }).await
    });

    // Observe CURRENT_DIRECTORY and update config store
    Task::start(async {
        CURRENT_DIRECTORY.signal_cloned().for_each(|current_dir| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let dir_to_save = if current_dir.is_empty() { None } else { Some(current_dir) };
                config_store().session.lock_ref().file_picker.lock_ref().current_directory.set_neq(dir_to_save);
                save_config_to_backend();
            }
        }).await
    });

    // Observe LOAD_FILES_SCROLL_POSITION and update config store
    Task::start(async {
        LOAD_FILES_SCROLL_POSITION.signal().for_each(|scroll_pos| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().session.lock_ref().file_picker.lock_ref().scroll_position.set_neq(scroll_pos);
                save_config_to_backend();
            }
        }).await
    });

    // Observe timeline state and update config store
    Task::start(async {
        TIMELINE_CURSOR_POSITION.signal().for_each(|cursor_pos| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().timeline_cursor_position.set_neq(cursor_pos);
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        TIMELINE_ZOOM_LEVEL.signal().for_each(|zoom_level| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().timeline_zoom_level.set_neq(zoom_level);
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        TIMELINE_VISIBLE_RANGE_START.signal().for_each(|start| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().timeline_visible_range_start.set_neq(start);
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        TIMELINE_VISIBLE_RANGE_END.signal().for_each(|end| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                config_store().workspace.lock_mut().timeline_visible_range_end.set_neq(end);
                save_config_to_backend();
            }
        }).await
    });

    // Observe panel dimensions back to config when UI updates them  
    Task::start(async {
        FILES_PANEL_WIDTH.signal().for_each(|width| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
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
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        FILES_PANEL_HEIGHT.signal().for_each(|height| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
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
                save_config_to_backend();
            }
        }).await
    });

    // Observe column widths and update config when user drags dividers
    Task::start(async {
        VARIABLES_NAME_COLUMN_WIDTH.signal().for_each(|width| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let dock_mode = config_store().workspace.lock_ref().dock_mode.get_cloned();
                let workspace_ref = config_store().workspace.lock_ref();
                let layouts = workspace_ref.panel_layouts.lock_ref();
                
                match dock_mode {
                    DockMode::Bottom => {
                        layouts.docked_to_bottom.lock_ref().variables_name_column_width.set_neq(width as f32);
                    }
                    DockMode::Right => {
                        layouts.docked_to_right.lock_ref().variables_name_column_width.set_neq(width as f32);
                    }
                }
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        VARIABLES_VALUE_COLUMN_WIDTH.signal().for_each(|width| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let dock_mode = config_store().workspace.lock_ref().dock_mode.get_cloned();
                let workspace_ref = config_store().workspace.lock_ref();
                let layouts = workspace_ref.panel_layouts.lock_ref();
                
                match dock_mode {
                    DockMode::Bottom => {
                        layouts.docked_to_bottom.lock_ref().variables_value_column_width.set_neq(width as f32);
                    }
                    DockMode::Right => {
                        layouts.docked_to_right.lock_ref().variables_value_column_width.set_neq(width as f32);
                    }
                }
                save_config_to_backend();
            }
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