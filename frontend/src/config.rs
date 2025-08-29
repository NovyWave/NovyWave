// New Unified Config System
// Based on ringrev_private patterns: Big struct with nested Mutable values and triggers

use zoon::*;
use serde::{Deserialize, Serialize};
use shared::UpMsg;
pub use shared::{Theme, DockMode}; // Re-export for frontend usage
use crate::CONFIG_INITIALIZATION_COMPLETE;
use crate::dataflow::Atom;
use crate::actors::waveform_timeline::{current_cursor_position, current_viewport, viewport_signal,
    current_ns_per_pixel, ns_per_pixel_signal};
use crate::actors::global_domains::waveform_timeline_domain;
use crate::actors::panel_layout::{files_panel_width_signal, files_panel_height_signal, 
    variables_name_column_width_signal, variables_value_column_width_signal};
use crate::time_types::{NsPerPixel, TimeNs};

// Reactive triggers module
pub mod triggers;

// Timeline validation constants  
const MIN_VALID_RANGE_NS: u64 = 1_000;       // 1 microsecond minimum range in nanoseconds
const SAFE_FALLBACK_START_NS: u64 = 0;       // Safe fallback start time in nanoseconds
const SAFE_FALLBACK_END_NS: u64 = 100_000_000_000;  // Safe fallback end time (100s) in nanoseconds

/// Validate timeline values from config to prevent invalid values
fn validate_timeline_values_ns(cursor_ns: u64, zoom: f32, start_ns: u64, end_ns: u64) -> (TimeNs, f32, TimeNs, TimeNs) {
    // Validate cursor position (convert to TimeNs)
    let safe_cursor = TimeNs::from_nanos(cursor_ns);
    
    // Validate zoom level
    let safe_zoom = if zoom.is_finite() && zoom >= 1.0 && zoom <= 1e9 { zoom } else { 1.0 };
    
    // Validate range
    let (safe_start_ns, safe_end_ns) = if start_ns < end_ns && (end_ns - start_ns) >= MIN_VALID_RANGE_NS {
        (start_ns, end_ns)
    } else {
        crate::debug_utils::debug_critical(&format!("CONFIG: Invalid timeline range ({}, {}), using fallback", start_ns, end_ns));
        (SAFE_FALLBACK_START_NS, SAFE_FALLBACK_END_NS)
    };
    
    (safe_cursor, safe_zoom, TimeNs::from_nanos(safe_start_ns), TimeNs::from_nanos(safe_end_ns))
}

// =============================================================================
// MAIN CONFIG STORE - Single Source of Truth with Reactive Fields
// =============================================================================

#[derive(Clone)]
pub struct ConfigStore {
    pub app: Atom<AppSection>,
    pub ui: Atom<UiSection>,
    pub session: Atom<SessionSection>,
    pub workspace: Atom<WorkspaceSection>,
    pub dialogs: Atom<DialogSection>,
}

// =============================================================================
// APP SECTION - Version and Migration
// =============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct AppSection {
    pub version: String,
    pub migration_strategy: MigrationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStrategy {
    None,
    Upgrade,
    Recreate,
}

// =============================================================================
// UI SECTION - Theme and Visual Preferences
// =============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct UiSection {
    pub theme: Theme,
    pub font_size: f32,
    pub show_tooltips: bool,
    pub toast_dismiss_ms: u64,
}

// Theme and DockMode enums now imported from shared crate for type safety

// DockMode enum now imported from shared crate for type safety

// =============================================================================
// SESSION SECTION - Files and Search State
// =============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct SessionSection {
    pub opened_files: Vec<String>,
    pub variables_search_filter: String,
    pub file_picker: FilePickerSection,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct FilePickerSection {
    pub current_directory: Option<String>,
    pub expanded_directories: Vec<String>,
    pub show_hidden_files: bool,
    pub scroll_position: i32,
}

// =============================================================================
// WORKSPACE SECTION - Layout and Panel State
// =============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct WorkspaceSection {
    pub dock_mode: DockMode,
    pub selected_scope_id: Option<String>,
    pub expanded_scopes: Vec<String>,
    pub load_files_expanded_directories: Vec<String>,
    pub panel_layouts: PanelLayouts,
    pub selected_variables: Vec<shared::SelectedVariable>,
    pub timeline_cursor_position: TimeNs,
    pub timeline_zoom_level: f32,
    pub timeline_visible_range_start: TimeNs,
    pub timeline_visible_range_end: TimeNs,
}

// DockMode enum now imported from shared crate for type safety

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct PanelLayouts {
    pub docked_to_bottom: PanelDimensions,
    pub docked_to_right: PanelDimensions,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct PanelDimensions {
    pub files_panel_width: f32,
    pub files_panel_height: f32,
    pub variables_panel_width: f32,
    pub timeline_panel_height: f32,
    pub variables_name_column_width: f32,
    pub variables_value_column_width: f32,
}

// =============================================================================
// DIALOG SECTION - Dialog and Modal State
// =============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DialogSection {
    pub show_file_dialog: bool,
    pub show_settings_dialog: bool,
    pub show_about_dialog: bool,
    pub file_paths_input: String,
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
            app: Atom::new(AppSection::default()),
            ui: Atom::new(UiSection::default()),
            session: Atom::new(SessionSection::default()),
            workspace: Atom::new(WorkspaceSection::default()),
            dialogs: Atom::new(DialogSection::default()),
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
            version: "1.0.0".to_string(),
            migration_strategy: MigrationStrategy::None,
        }
    }
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            font_size: 14.0,
            show_tooltips: true,
            toast_dismiss_ms: 10000, // Default 10 seconds
        }
    }
}

impl Default for SessionSection {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            variables_search_filter: String::new(),
            file_picker: FilePickerSection::default(),
        }
    }
}

impl Default for FilePickerSection {
    fn default() -> Self {
        Self {
            current_directory: None,
            expanded_directories: Vec::new(),
            show_hidden_files: false,
            scroll_position: 0,
        }
    }
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            dock_mode: DockMode::Bottom,
            selected_scope_id: None,
            expanded_scopes: Vec::new(),
            load_files_expanded_directories: Vec::new(),
            panel_layouts: PanelLayouts::default(),
            selected_variables: Vec::new(),
            timeline_cursor_position: TimeNs::from_nanos(10_000_000_000), // 10 seconds
            timeline_zoom_level: 1.0,
            timeline_visible_range_start: TimeNs::from_nanos(0),
            timeline_visible_range_end: TimeNs::from_nanos(100_000_000_000), // 100 seconds
        }
    }
}

impl Default for PanelLayouts {
    fn default() -> Self {
        Self {
            docked_to_bottom: PanelDimensions {
                files_panel_width: 1400.0,
                files_panel_height: 600.0,
                variables_panel_width: 300.0,
                timeline_panel_height: 200.0,
                variables_name_column_width: 180.0,
                variables_value_column_width: 100.0,
            },
            docked_to_right: PanelDimensions {
                files_panel_width: 400.0,
                files_panel_height: 300.0,
                variables_panel_width: 250.0,
                timeline_panel_height: 150.0,
                variables_name_column_width: 180.0,
                variables_value_column_width: 100.0,
            },
        }
    }
}

impl Default for DialogSection {
    fn default() -> Self {
        Self {
            show_file_dialog: false,
            show_settings_dialog: false,
            show_about_dialog: false,
            file_paths_input: String::new(),
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
    pub timeline_cursor_position: u64,  // nanoseconds
    pub timeline_zoom_level: f32,
    pub timeline_visible_range_start: u64,  // nanoseconds
    pub timeline_visible_range_end: u64,  // nanoseconds
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
                version: self.app.current_value().version.clone(),
                migration_strategy: self.app.current_value().migration_strategy.clone(),
            },
            ui: SerializableUiSection {
                theme: self.ui.current_value().theme,
                font_size: self.ui.current_value().font_size,
                show_tooltips: self.ui.current_value().show_tooltips,
                toast_dismiss_ms: self.ui.current_value().toast_dismiss_ms,
            },
            session: SerializableSessionSection {
                opened_files: self.session.current_value().opened_files.clone(),
                variables_search_filter: self.session.current_value().variables_search_filter.clone(),
                file_picker: SerializableFilePickerSection {
                    current_directory: self.session.current_value().file_picker.current_directory.clone(),
                    expanded_directories: self.session.current_value().file_picker.expanded_directories.clone(),
                    show_hidden_files: self.session.current_value().file_picker.show_hidden_files,
                    scroll_position: self.session.current_value().file_picker.scroll_position,
                },
            },
            workspace: SerializableWorkspaceSection {
                dock_mode: self.workspace.current_value().dock_mode,
                selected_scope_id: self.workspace.current_value().selected_scope_id.clone(),
                expanded_scopes: self.workspace.current_value().expanded_scopes.clone(),
                load_files_expanded_directories: self.workspace.current_value().load_files_expanded_directories.clone(),
                selected_variables: self.workspace.current_value().selected_variables.clone(),
                panel_layouts: SerializablePanelLayouts {
                    docked_to_bottom: SerializablePanelDimensions {
                        files_panel_width: self.workspace.current_value().panel_layouts.docked_to_bottom.files_panel_width,
                        files_panel_height: self.workspace.current_value().panel_layouts.docked_to_bottom.files_panel_height,
                        variables_panel_width: self.workspace.current_value().panel_layouts.docked_to_bottom.variables_panel_width,
                        timeline_panel_height: self.workspace.current_value().panel_layouts.docked_to_bottom.timeline_panel_height,
                        variables_name_column_width: self.workspace.current_value().panel_layouts.docked_to_bottom.variables_name_column_width,
                        variables_value_column_width: self.workspace.current_value().panel_layouts.docked_to_bottom.variables_value_column_width,
                    },
                    docked_to_right: SerializablePanelDimensions {
                        files_panel_width: self.workspace.current_value().panel_layouts.docked_to_right.files_panel_width,
                        files_panel_height: self.workspace.current_value().panel_layouts.docked_to_right.files_panel_height,
                        variables_panel_width: self.workspace.current_value().panel_layouts.docked_to_right.variables_panel_width,
                        timeline_panel_height: self.workspace.current_value().panel_layouts.docked_to_right.timeline_panel_height,
                        variables_name_column_width: self.workspace.current_value().panel_layouts.docked_to_right.variables_name_column_width,
                        variables_value_column_width: self.workspace.current_value().panel_layouts.docked_to_right.variables_value_column_width,
                    },
                },
                timeline_cursor_position: self.workspace.current_value().timeline_cursor_position.nanos(),
                timeline_zoom_level: self.workspace.current_value().timeline_zoom_level,
                timeline_visible_range_start: self.workspace.current_value().timeline_visible_range_start.nanos(),
                timeline_visible_range_end: self.workspace.current_value().timeline_visible_range_end.nanos(),
            },
            dialogs: SerializableDialogSection {
                show_file_dialog: self.dialogs.current_value().show_file_dialog,
                show_settings_dialog: self.dialogs.current_value().show_settings_dialog,
                show_about_dialog: self.dialogs.current_value().show_about_dialog,
                file_paths_input: self.dialogs.current_value().file_paths_input.clone(),
            },
        }
    }

    pub fn load_from_serializable(&self, config: SerializableConfig) {
        // Load app section - update entire section at once
        let app_section = AppSection {
            version: config.app.version,
            migration_strategy: config.app.migration_strategy,
        };
        self.app.set(app_section);

        // Load UI section
        let ui_section = UiSection {
            theme: config.ui.theme,
            font_size: config.ui.font_size,
            show_tooltips: config.ui.show_tooltips,
            toast_dismiss_ms: config.ui.toast_dismiss_ms,
        };
        self.ui.set(ui_section);

        // Load session section
        let file_picker = FilePickerSection {
            current_directory: config.session.file_picker.current_directory,
            expanded_directories: config.session.file_picker.expanded_directories,
            show_hidden_files: config.session.file_picker.show_hidden_files,
            scroll_position: config.session.file_picker.scroll_position,
        };
        
        let session_section = SessionSection {
            opened_files: config.session.opened_files,
            variables_search_filter: config.session.variables_search_filter,
            file_picker,
        };
        self.session.set(session_section);

        // Load workspace section
        // Validate timeline values before creating workspace section
        let (safe_cursor, safe_zoom, safe_start, safe_end) = validate_timeline_values_ns(
            config.workspace.timeline_cursor_position,
            config.workspace.timeline_zoom_level,
            config.workspace.timeline_visible_range_start,
            config.workspace.timeline_visible_range_end
        );

        let bottom_dims = PanelDimensions {
            files_panel_width: config.workspace.panel_layouts.docked_to_bottom.files_panel_width,
            files_panel_height: config.workspace.panel_layouts.docked_to_bottom.files_panel_height,
            variables_panel_width: config.workspace.panel_layouts.docked_to_bottom.variables_panel_width,
            timeline_panel_height: config.workspace.panel_layouts.docked_to_bottom.timeline_panel_height,
            variables_name_column_width: config.workspace.panel_layouts.docked_to_bottom.variables_name_column_width,
            variables_value_column_width: config.workspace.panel_layouts.docked_to_bottom.variables_value_column_width,
        };
        
        let right_dims = PanelDimensions {
            files_panel_width: config.workspace.panel_layouts.docked_to_right.files_panel_width,
            files_panel_height: config.workspace.panel_layouts.docked_to_right.files_panel_height,
            variables_panel_width: config.workspace.panel_layouts.docked_to_right.variables_panel_width,
            timeline_panel_height: config.workspace.panel_layouts.docked_to_right.timeline_panel_height,
            variables_name_column_width: config.workspace.panel_layouts.docked_to_right.variables_name_column_width,
            variables_value_column_width: config.workspace.panel_layouts.docked_to_right.variables_value_column_width,
        };
        
        let panel_layouts = PanelLayouts {
            docked_to_bottom: bottom_dims,
            docked_to_right: right_dims,
        };
        
        let workspace_section = WorkspaceSection {
            dock_mode: config.workspace.dock_mode,
            selected_scope_id: config.workspace.selected_scope_id,
            expanded_scopes: config.workspace.expanded_scopes,
            load_files_expanded_directories: config.workspace.load_files_expanded_directories,
            selected_variables: config.workspace.selected_variables,
            panel_layouts,
            timeline_cursor_position: safe_cursor,
            timeline_zoom_level: safe_zoom,
            timeline_visible_range_start: safe_start,
            timeline_visible_range_end: safe_end,
        };
        self.workspace.set(workspace_section);

        // Load dialogs section
        let dialogs_section = DialogSection {
            show_file_dialog: config.dialogs.show_file_dialog,
            show_settings_dialog: config.dialogs.show_settings_dialog,
            show_about_dialog: config.dialogs.show_about_dialog,
            file_paths_input: config.dialogs.file_paths_input,
        };
        self.dialogs.set(dialogs_section);
    }
}

// =============================================================================
// TRIGGERS MODULE - Reactive Config Persistence
// =============================================================================

pub fn create_config_triggers() {
    store_config_on_any_change();
}

fn store_config_on_any_change() {
    // UI theme changes - simple signal for theme field
    Task::start(async move {
        config_store().ui.signal_ref(|ui| ui.theme.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    // Dock mode changes - simple signal for dock_mode field
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.dock_mode.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    // Panel dimension changes - simplified signal handling
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_bottom.files_panel_width.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_bottom.files_panel_height.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_right.files_panel_width.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_right.files_panel_height.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    // Variables column width changes - bottom dock mode
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_bottom.variables_name_column_width.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_bottom.variables_value_column_width.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    // Variables column width changes - right dock mode
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_right.variables_name_column_width.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    Task::start(async move {
        config_store().workspace.signal_ref(|ws| ws.panel_layouts.docked_to_right.variables_value_column_width.clone()).for_each_sync(|_| {
            // Only save if initialization is complete to prevent race conditions
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        }).await
    });
    
    // Timeline cursor position changes - TRUE DEBOUNCING with droppable tasks
    Task::start(async move {
        let waveform_timeline = waveform_timeline_domain();
        let timeline_cursor_position_signal = waveform_timeline.cursor_position_signal();
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
            // New nanosecond fields
            timeline_cursor_position_ns: {
                let cursor_ns = current_cursor_position();
                cursor_ns.nanos()
            },
            timeline_visible_range_start_ns: {
                let viewport = current_viewport();
                Some(viewport.start.nanos())
            },
            timeline_visible_range_end_ns: {
                let viewport = current_viewport();
                Some(viewport.end.nanos())
            },
            timeline_zoom_level: {
                let ns_per_pixel = current_ns_per_pixel();
                // Convert NsPerPixel to normalized factor for config storage
                let factor = (NsPerPixel::LOW_ZOOM.nanos() as f64 / ns_per_pixel.nanos() as f64).clamp(0.0, 1.0);
                factor as f32
            },
        },
    };

    // Use platform abstraction instead of direct connection
    Task::start(async move {
        let _ = CurrentPlatform::send_message(UpMsg::SaveConfig(app_config)).await;
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
    let mut session = config_store().session.current_value();
    session.opened_files = file_paths;
    config_store().session.set(session);
    
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
    
    let mut workspace = config_store().workspace.current_value();
    workspace.dock_mode = dock_mode;
    config_store().workspace.set(workspace);
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
            timeline_cursor_position: config.workspace.timeline_cursor_position_ns,  // u64 - no unwrap needed
            timeline_zoom_level: config.workspace.timeline_zoom_level,
            timeline_visible_range_start: config.workspace.timeline_visible_range_start_ns.unwrap_or(0),  // Option<u64>
            timeline_visible_range_end: config.workspace.timeline_visible_range_end_ns.unwrap_or(100_000_000_000),  // Option<u64>
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
    config_store().ui.signal_ref(|ui| ui.theme.clone())
}

/// Get current toast dismiss time
pub fn current_toast_dismiss_ms() -> u64 {
    config_store().ui.current_value().toast_dismiss_ms
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
                let mut session = config_store().session.current_value();
                session.opened_files = file_paths;
                config_store().session.set(session);
                save_config_to_backend();
            }
        }).await
    });

    // Observe EXPANDED_SCOPES_FOR_CONFIG and update config store
    Task::start(async {
        EXPANDED_SCOPES_FOR_CONFIG.signal_cloned().for_each(|expanded_scopes| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                workspace.expanded_scopes = expanded_scopes;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    // Observe LOAD_FILES_EXPANDED_DIRECTORIES_FOR_CONFIG and update config store
    Task::start(async {
        LOAD_FILES_EXPANDED_DIRECTORIES_FOR_CONFIG.signal_cloned().for_each(|expanded_dirs| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                workspace.load_files_expanded_directories = expanded_dirs;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    // Observe SELECTED_VARIABLES_FOR_CONFIG and update config store
    Task::start(async {
        SELECTED_VARIABLES_FOR_CONFIG.signal_cloned().for_each(|selected_vars| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                workspace.selected_variables = selected_vars;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    // Observe DOCK_MODE_FOR_CONFIG and update config store
    Task::start(async {
        DOCK_MODE_FOR_CONFIG.signal_cloned().for_each(|dock_mode| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                workspace.dock_mode = dock_mode;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    // Observe SELECTED_SCOPE_ID and update config store
    Task::start(async {
        SELECTED_SCOPE_ID.signal_cloned().for_each(|scope_id| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                workspace.selected_scope_id = scope_id;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    // Observe VARIABLES_SEARCH_FILTER and update config store
    Task::start(async {
        VARIABLES_SEARCH_FILTER.signal_cloned().for_each(|filter| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut session = config_store().session.current_value();
                session.variables_search_filter = filter;
                config_store().session.set(session);
                save_config_to_backend();
            }
        }).await
    });

    // Observe CURRENT_DIRECTORY and update config store
    Task::start(async {
        CURRENT_DIRECTORY.signal_cloned().for_each(|current_dir| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let dir_to_save = if current_dir.is_empty() { None } else { Some(current_dir) };
                let mut session = config_store().session.current_value();
                session.file_picker.current_directory = dir_to_save;
                config_store().session.set(session);
                save_config_to_backend();
            }
        }).await
    });

    // Observe LOAD_FILES_SCROLL_POSITION and update config store
    Task::start(async {
        LOAD_FILES_SCROLL_POSITION.signal().for_each(|scroll_pos| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut session = config_store().session.current_value();
                session.file_picker.scroll_position = scroll_pos;
                config_store().session.set(session);
                save_config_to_backend();
            }
        }).await
    });

    // Observe timeline state and update config store
    Task::start(async {
        waveform_timeline_domain().cursor_position_signal().for_each(|cursor_pos| async move {
            let cursor_ns = cursor_pos;
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                workspace.timeline_cursor_position = cursor_ns;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        ns_per_pixel_signal().for_each(|ns_per_pixel| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                // Convert NsPerPixel to normalized factor for config storage
                let factor = (NsPerPixel::LOW_ZOOM.nanos() as f64 / ns_per_pixel.nanos() as f64).clamp(0.0, 1.0);
                let mut workspace = config_store().workspace.current_value();
                workspace.timeline_zoom_level = factor as f32;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        viewport_signal().for_each(|viewport| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                workspace.timeline_visible_range_start = viewport.start;
                workspace.timeline_visible_range_end = viewport.end;
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    // Observe panel dimensions back to config when UI updates them  
    Task::start(async {
        files_panel_width_signal().for_each(|width| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                let dock_mode = workspace.dock_mode;
                
                match dock_mode {
                    DockMode::Bottom => {
                        workspace.panel_layouts.docked_to_bottom.files_panel_width = width as f32;
                    }
                    DockMode::Right => {
                        workspace.panel_layouts.docked_to_right.files_panel_width = width as f32;
                    }
                }
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        files_panel_height_signal().for_each(|height| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                let dock_mode = workspace.dock_mode;
                
                match dock_mode {
                    DockMode::Bottom => {
                        workspace.panel_layouts.docked_to_bottom.files_panel_height = height as f32;
                    }
                    DockMode::Right => {
                        workspace.panel_layouts.docked_to_right.files_panel_height = height as f32;
                    }
                }
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    // Observe column widths and update config when user drags dividers
    Task::start(async {
        variables_name_column_width_signal().for_each(|width| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                let dock_mode = workspace.dock_mode;
                
                match dock_mode {
                    DockMode::Bottom => {
                        workspace.panel_layouts.docked_to_bottom.variables_name_column_width = width as f32;
                    }
                    DockMode::Right => {
                        workspace.panel_layouts.docked_to_right.variables_name_column_width = width as f32;
                    }
                }
                config_store().workspace.set(workspace);
                save_config_to_backend();
            }
        }).await
    });

    Task::start(async {
        variables_value_column_width_signal().for_each(|width| async move {
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                let mut workspace = config_store().workspace.current_value();
                let dock_mode = workspace.dock_mode;
                
                match dock_mode {
                    DockMode::Bottom => {
                        workspace.panel_layouts.docked_to_bottom.variables_value_column_width = width as f32;
                    }
                    DockMode::Right => {
                        workspace.panel_layouts.docked_to_right.variables_value_column_width = width as f32;
                    }
                }
                config_store().workspace.set(workspace);
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