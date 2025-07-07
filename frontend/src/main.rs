use zoon::{*, futures_util::future::try_join_all};
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::theme::{Theme, init_theme, theme, toggle_theme};
use serde::{Serialize, Deserialize};
use std::collections::{HashSet, HashMap};
use web_sys::Performance;
use wasm_bindgen::JsCast;

// Virtual variables JavaScript integration - loaded via backend public API


// Performance measurement utilities
fn get_performance() -> Option<Performance> {
    web_sys::window()?.performance()
}

fn now() -> f64 {
    get_performance()
        .map(|perf| perf.now())
        .unwrap_or(0.0)
}

struct PerformanceTimer {
    start_time: f64,
    label: String,
}

impl PerformanceTimer {
    fn new(label: &str) -> Self {
        let start_time = now();
        Self {
            start_time,
            label: label.to_string(),
        }
    }
    
    fn elapsed(&self) -> f64 {
        now() - self.start_time
    }
    
    fn log_elapsed(&self) {
        let _elapsed = self.elapsed();
    }
}

impl Drop for PerformanceTimer {
    fn drop(&mut self) {
        self.log_elapsed();
    }
}

// Panel resizing state
static LEFT_PANEL_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 470.into());
static FILES_PANEL_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 300.into());
static VERTICAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
static HORIZONTAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Search filter for Variables panel
static VARIABLES_SEARCH_FILTER: Lazy<Mutable<String>> = lazy::default();




// Dock state management - DEFAULT TO DOCKED MODE  
static IS_DOCKED_TO_BOTTOM: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(true));
static MAIN_AREA_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 350.into());

// File dialog state
static SHOW_FILE_DIALOG: Lazy<Mutable<bool>> = lazy::default();
static FILE_PATHS_INPUT: Lazy<Mutable<String>> = lazy::default();

// File loading progress state
static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();

// Loaded files hierarchy for TreeView
static LOADED_FILES: Lazy<MutableVec<WaveformFile>> = lazy::default();
static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = lazy::default();
static TREE_SELECTED_ITEMS: Lazy<Mutable<HashSet<String>>> = lazy::default();

// Track file ID to full path mapping for config persistence
static FILE_PATHS: Lazy<Mutable<HashMap<String, String>>> = lazy::default();

// Track expanded scopes for TreeView persistence
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = lazy::default();

// Store scope selections from config to apply after files load
static SAVED_SCOPE_SELECTIONS: Lazy<Mutable<HashMap<String, String>>> = lazy::default();

// Store the loaded config to preserve inactive mode settings when saving
static LOADED_CONFIG: Lazy<Mutable<Option<AppConfig>>> = lazy::default();

// Flag to prevent auto-saving before initial config is loaded
static CONFIG_LOADED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));


fn generate_file_id(file_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    format!("file_{:x}", hasher.finish())
}

#[derive(Clone, Debug)]
pub struct LoadingFile {
    pub file_id: String,
    pub filename: String,
    pub progress: f32,
    pub status: LoadingStatus,
}

#[derive(Clone, Debug)]
pub enum LoadingStatus {
    Starting,
    Parsing,
    Completed,
    Error(String),
}

// Backend message types
#[derive(Serialize, Deserialize, Debug)]
pub enum UpMsg {
    LoadWaveformFile(String),
    GetParsingProgress(String),
    LoadConfig,
    SaveConfig(AppConfig),
    SaveTheme(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DownMsg {
    ParsingStarted { file_id: String, filename: String },
    ParsingProgress { file_id: String, progress: f32 },
    FileLoaded { file_id: String, hierarchy: FileHierarchy },
    ParsingError { file_id: String, error: String },
    ConfigLoaded(AppConfig),
    ConfigSaved,
    ConfigError(String),
    ThemeSaved,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileHierarchy {
    pub files: Vec<WaveformFile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaveformFile {
    pub id: String,
    pub filename: String,
    pub format: FileFormat,
    pub scopes: Vec<ScopeData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileFormat {
    VCD,
    FST,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScopeData {
    pub id: String,
    pub name: String,
    pub full_name: String,
    pub children: Vec<ScopeData>,
    pub variables: Vec<Signal>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signal {
    pub id: String,
    pub name: String,
    pub signal_type: String,
    pub width: u32,
}


// Configuration structures (matching backend)
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AppConfig {
    pub app: AppSection,
    pub ui: UiSection,
    pub files: FilesSection,
    pub workspace: WorkspaceSection,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppSection {
    pub version: String,
    pub auto_load_previous_files: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UiSection {
    pub theme: String, // "dark" or "light"
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct FilesSection {
    pub opened_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct WorkspaceSection {
    pub dock_to_bottom: bool,
    pub docked_to_bottom: DockedToBottomLayout,
    pub docked_to_right: DockedToRightLayout,
    pub scope_selection: std::collections::HashMap<String, String>,
    pub expanded_scopes: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DockedToBottomLayout {
    pub main_area_height: u32,
    pub files_panel_width: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DockedToRightLayout {
    pub files_panel_height: u32,
    pub files_panel_width: u32,
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            auto_load_previous_files: true,
        }
    }
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
        }
    }
}


fn show_file_paths_dialog() {
    SHOW_FILE_DIALOG.set(true);
    FILE_PATHS_INPUT.set_neq(String::new());
}

fn process_file_paths() {
    let input = FILE_PATHS_INPUT.get_cloned();
    let paths: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    
    if !paths.is_empty() {
        IS_LOADING.set(true);
    }
    
    for path in paths {
        // Generate file ID and store path mapping for config persistence
        let file_id = generate_file_id(&path);
        FILE_PATHS.lock_mut().insert(file_id, path.clone());
        
        send_up_msg(UpMsg::LoadWaveformFile(path));
    }
    
    SHOW_FILE_DIALOG.set(false);
}

static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    Connection::new(|down_msg, _| {
        // DownMsg logging disabled - causes CLI overflow with large files
        match down_msg {
            DownMsg::ParsingStarted { file_id, filename } => {
                // Add or update loading file
                let loading_file = LoadingFile {
                    file_id: file_id.clone(),
                    filename: filename.clone(),
                    progress: 0.0,
                    status: LoadingStatus::Starting,
                };
                
                LOADING_FILES.lock_mut().push_cloned(loading_file);
            }
            DownMsg::ParsingProgress { file_id, progress } => {
                // Update progress for the file
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.progress = progress;
                        file.status = LoadingStatus::Parsing;
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
            }
            DownMsg::FileLoaded { file_id, hierarchy } => {
                // Add loaded files to the TreeView state
                for file in hierarchy.files {
                    LOADED_FILES.lock_mut().push_cloned(file.clone());
                    
                    // Store scope selection for later restoration (don't restore immediately)
                    // This prevents multiple files from fighting over global selection during loading
                }
                
                // Mark file as completed
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.progress = 1.0;
                        file.status = LoadingStatus::Completed;
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
                
                // Check if all files are completed
                check_loading_complete();
                
                // Auto-save config with updated file list
                save_current_config();
            }
            DownMsg::ParsingError { file_id, error } => {
                zoon::println!("Error parsing file {}: {}", file_id, error);
                
                // Mark file as error
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.status = LoadingStatus::Error(error.clone());
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
                
                // Check if all files are completed
                check_loading_complete();
            }
            DownMsg::ConfigLoaded(config) => {
                apply_config(config);
            }
            DownMsg::ConfigSaved => {
                // Config saved successfully
            }
            DownMsg::ConfigError(_error) => {
                // Config error: {}
            }
            DownMsg::ThemeSaved => {
                // Theme saved successfully
            }
        }
    })
});

fn check_loading_complete() {
    let loading_files = LOADING_FILES.lock_ref();
    let all_done = loading_files.iter().all(|f| {
        matches!(f.status, LoadingStatus::Completed | LoadingStatus::Error(_))
    });
    
    if all_done {
        IS_LOADING.set(false);
        
        // Restore scope selections - defer signal updates to prevent deadlock
        restore_scope_selections_deferred();
        
        // Clear completed files after a delay to show final state
        Task::start(async {
            Timer::sleep(2000).await;
            LOADING_FILES.lock_mut().clear();
        });
    }
}

fn restore_scope_selections_deferred() {
    // Extract data from locks first, then update signals without holding locks
    let scope_to_restore = {
        let saved_selections = SAVED_SCOPE_SELECTIONS.lock_ref();
        if saved_selections.is_empty() {
            None
        } else {
            // Find the first valid scope to restore
            let mut found_scope = None;
            for (file_id, scope_id) in saved_selections.iter() {
                let file_exists = LOADED_FILES.lock_ref().iter().any(|f| f.id == *file_id);
                if !file_exists {
                    continue;
                }
                
                let scope_exists = LOADED_FILES.lock_ref().iter().any(|file| {
                    file.id == *file_id && file_contains_scope(&file.scopes, scope_id)
                });
                
                if scope_exists {
                    found_scope = Some(scope_id.clone());
                    break;
                }
            }
            found_scope
        }
    };
    
    // Only proceed if we found a scope to restore
    if let Some(scope_id) = scope_to_restore {
        // Clear saved selections after successful extraction
        SAVED_SCOPE_SELECTIONS.lock_mut().clear();
        
        // Update signals without holding any locks
        SELECTED_SCOPE_ID.set_neq(Some(scope_id.clone()));
        
        let mut new_selection = HashSet::new();
        new_selection.insert(scope_id);
        TREE_SELECTED_ITEMS.set_neq(new_selection);
    }
}

fn restore_scope_selections() {
    let saved_selections = SAVED_SCOPE_SELECTIONS.lock_ref();
    if saved_selections.is_empty() {
        return;
    }
    
    // Since the config system only saves ONE global scope selection,
    // we should only have one entry, but iterate to be safe
    for (file_id, scope_id) in saved_selections.iter() {
        // Check if this file is currently loaded
        let file_exists = LOADED_FILES.lock_ref().iter().any(|f| f.id == *file_id);
        if !file_exists {
            continue;
        }
        
        // Verify the scope actually exists in the loaded files
        let scope_exists = LOADED_FILES.lock_ref().iter().any(|file| {
            file.id == *file_id && file_contains_scope(&file.scopes, scope_id)
        });
        
        if scope_exists {
            // Update the global scope selection
            SELECTED_SCOPE_ID.set_neq(Some(scope_id.clone()));
            
            // Queue TreeView update to avoid triggering auto-save during restoration
            let scope_id_clone = scope_id.clone();
            Task::start(async move {
                let mut new_selection = HashSet::new();
                new_selection.insert(scope_id_clone);
                TREE_SELECTED_ITEMS.set_neq(new_selection);
            });
            
            // Only restore the first (and should be only) valid selection
            break;
        }
    }
    
    // Clear saved selections after restoration attempt
    SAVED_SCOPE_SELECTIONS.lock_mut().clear();
}

fn load_files_button_with_progress(variant: ButtonVariant, size: ButtonSize, icon: Option<IconName>) -> impl Element {
    El::new()
        .child_signal(IS_LOADING.signal().map(move |is_loading| {
            let mut btn = button();
            
            if is_loading {
                btn = btn.label("Loading...")
                    .disabled(true);
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            } else {
                btn = btn.label("Load Files")
                    .on_press(|| show_file_paths_dialog());
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            }
            
            btn.variant(variant.clone())
                .size(size.clone())
                .build()
                .into_element()
        }))
}

fn load_files_dialog_button() -> impl Element {
    El::new()
        .child_signal(IS_LOADING.signal().map(|is_loading| {
            let mut btn = button();
            
            if is_loading {
                btn = btn.label("Loading...")
                    .disabled(true);
            } else {
                btn = btn.label("Load Files")
                    .on_press(|| process_file_paths());
            }
            
            btn.variant(ButtonVariant::Primary)
                .size(ButtonSize::Medium)
                .build()
                .into_element()
        }))
}

fn send_up_msg(up_msg: UpMsg) {
    Task::start(async move {
        let result = CONNECTION.send_up_msg(up_msg).await;
        if let Err(error) = result {
            zoon::println!("Failed to send message: {:?}", error);
        }
    });
}

fn apply_config(config: AppConfig) {
    // Store the loaded config to preserve inactive mode settings when saving
    LOADED_CONFIG.set_neq(Some(config.clone()));
    
    // Apply UI settings - initialize NovyUI theme with custom persistence
    let initial_theme = match config.ui.theme.as_str() {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => Some(Theme::Dark), // Default fallback
    };
    
    // Initialize theme system with custom persistence that saves to config
    init_theme(initial_theme, Some(Box::new(|theme| {
        let theme_str = match theme {
            Theme::Light => "light",
            Theme::Dark => "dark",
        };
        send_up_msg(UpMsg::SaveTheme(theme_str.to_string()));
    })));
    
    // Apply workspace settings
    IS_DOCKED_TO_BOTTOM.set_neq(config.workspace.dock_to_bottom);
    
    // Apply layout-specific settings based on current mode
    if config.workspace.dock_to_bottom {
        // Docked to bottom mode
        LEFT_PANEL_WIDTH.set_neq(u32::max(50, config.workspace.docked_to_bottom.files_panel_width));
        MAIN_AREA_HEIGHT.set_neq(u32::max(50, config.workspace.docked_to_bottom.main_area_height));
    } else {
        // Docked to right mode  
        LEFT_PANEL_WIDTH.set_neq(u32::max(50, config.workspace.docked_to_right.files_panel_width));
        FILES_PANEL_HEIGHT.set_neq(u32::max(50, config.workspace.docked_to_right.files_panel_height));
    }
    
    // Restore expanded scopes
    if let Some(expanded_items) = config.workspace.expanded_scopes.get("current_session") {
        let expanded_set: HashSet<String> = expanded_items.iter().cloned().collect();
        EXPANDED_SCOPES.set_neq(expanded_set);
    }
    
    // Restore scope selections - store them to apply after files are loaded
    SAVED_SCOPE_SELECTIONS.set_neq(config.workspace.scope_selection);
    
    // Auto-load files if enabled
    if config.app.auto_load_previous_files && !config.files.opened_files.is_empty() {
        for file_path in config.files.opened_files {
            // Generate file ID and store path mapping for config persistence
            let file_id = generate_file_id(&file_path);
            FILE_PATHS.lock_mut().insert(file_id, file_path.clone());
            
            send_up_msg(UpMsg::LoadWaveformFile(file_path));
        }
    }
    
    // Mark config as loaded to enable auto-saving
    CONFIG_LOADED.set_neq(true);
}

fn save_current_config() {
    // Don't save until initial config has been loaded
    if !CONFIG_LOADED.get() {
        return;
    }
    
    // Collect current state - use full paths from FILE_PATHS mapping
    let file_paths = FILE_PATHS.lock_ref();
    let opened_files: Vec<String> = LOADED_FILES.lock_ref()
        .iter()
        .filter_map(|file| file_paths.get(&file.id).cloned())
        .collect();
    
    // Build scope selection mapping - find which file the selected scope belongs to
    let mut scope_selection = std::collections::HashMap::new();
    if let Some(scope_id) = SELECTED_SCOPE_ID.lock_ref().as_ref() {
        // Find which file contains this scope
        for file in LOADED_FILES.lock_ref().iter() {
            if file_contains_scope(&file.scopes, scope_id) {
                scope_selection.insert(file.id.clone(), scope_id.clone());
                break;
            }
        }
    }
    
    let is_docked = *IS_DOCKED_TO_BOTTOM.lock_ref();
    
    let config = AppConfig {
        app: AppSection {
            version: "0.1.0".to_string(),
            auto_load_previous_files: true,
        },
        ui: UiSection {
            theme: LOADED_CONFIG.lock_ref().as_ref()
                .map(|c| c.ui.theme.clone())
                .unwrap_or_else(|| "dark".to_string()),
        },
        files: FilesSection {
            opened_files,
        },
        workspace: {
            // Get existing config to preserve inactive mode settings
            let existing_config = LOADED_CONFIG.lock_ref().clone().unwrap_or_default();
            
            WorkspaceSection {
                dock_to_bottom: is_docked,
                docked_to_bottom: if is_docked {
                    // Active mode: update with current values
                    DockedToBottomLayout {
                        main_area_height: *MAIN_AREA_HEIGHT.lock_ref(),
                        files_panel_width: *LEFT_PANEL_WIDTH.lock_ref(),
                    }
                } else {
                    // Inactive mode: preserve existing values, but use defaults if zero
                    let existing = existing_config.workspace.docked_to_bottom;
                    DockedToBottomLayout {
                        main_area_height: if existing.main_area_height == 0 { 350 } else { existing.main_area_height },
                        files_panel_width: if existing.files_panel_width == 0 { 470 } else { existing.files_panel_width },
                    }
                },
                docked_to_right: if !is_docked {
                    // Active mode: update with current values
                    DockedToRightLayout {
                        files_panel_height: *FILES_PANEL_HEIGHT.lock_ref(),
                        files_panel_width: *LEFT_PANEL_WIDTH.lock_ref(),
                    }
                } else {
                    // Inactive mode: preserve existing values, but use defaults if zero
                    let existing = existing_config.workspace.docked_to_right;
                    DockedToRightLayout {
                        files_panel_height: if existing.files_panel_height == 0 { 300 } else { existing.files_panel_height },
                        files_panel_width: if existing.files_panel_width == 0 { 470 } else { existing.files_panel_width },
                    }
                },
                scope_selection,
                expanded_scopes: {
                    let mut expanded_scopes = std::collections::HashMap::new();
                    let expanded_items: Vec<String> = EXPANDED_SCOPES.lock_ref().iter().cloned().collect();
                    if !expanded_items.is_empty() {
                        expanded_scopes.insert("current_session".to_string(), expanded_items);
                    }
                    expanded_scopes
                },
            }
        },
    };
    
    send_up_msg(UpMsg::SaveConfig(config));
}

/// Entry point: loads fonts and starts the app.
pub fn main() {
    Task::start(async {
        load_and_register_fonts().await;
        
        // Connect TreeView selections to scope selection
        init_scope_selection();
        
        start_app("app", root);
        CONNECTION.init_lazy();
        
        // Load configuration on startup
        send_up_msg(UpMsg::LoadConfig);
    });
}

fn init_scope_selection() {
    Task::start(async {
        TREE_SELECTED_ITEMS.signal_ref(|selected_items| {
            selected_items.clone()
        }).for_each_sync(|selected_items| {
            // Find the first selected scope (has _scope_ pattern, not just file_)
            if let Some(scope_id) = selected_items.iter().find(|id| id.contains("_scope_")) {
                SELECTED_SCOPE_ID.set_neq(Some(scope_id.clone()));
                save_current_config();
            } else {
                SELECTED_SCOPE_ID.set_neq(None);
                save_current_config();
            }
        }).await
    });
    
    // Auto-save when expanded scopes change
    Task::start(async {
        EXPANDED_SCOPES.signal_ref(|expanded_scopes| {
            expanded_scopes.clone()
        }).for_each_sync(|_expanded_scopes| {
            save_current_config();
        }).await
    });
}

/// Loads and registers required fonts asynchronously.
async fn load_and_register_fonts() {
    let fonts = try_join_all([
        fast2d::fetch_file("/_api/public/fonts/FiraCode-Regular.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-Regular.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-Bold.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-BoldItalic.ttf"),
    ]).await.unwrap_throw();
    fast2d::register_fonts(fonts).unwrap_throw();
}


fn file_paths_dialog() -> impl Element {
    El::new()
        .s(Background::new().color("rgba(0, 0, 0, 0.8)"))
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .child(
            El::new()
                .s(Background::new().color(hsluv!(220, 15, 15)))
                .s(RoundedCorners::all(8))
                .s(Borders::all(Border::new().width(2).color(hsluv!(220, 10, 30))))
                .s(Padding::all(24))
                .s(Width::exact(500))
                .child(
                    Column::new()
                        .s(Gap::new().y(16))
                        .item(
                            El::new()
                                .s(Font::new().size(18).weight(FontWeight::Bold).color(hsluv!(220, 10, 85)))
                                .child("Load Waveform Files")
                        )
                        .item(
                            El::new()
                                .s(Font::new().size(14).color(hsluv!(220, 10, 70)))
                                .child("Enter absolute file paths, separated by commas:")
                        )
                        .item(
                            input()
                                .placeholder("/path/to/file1.vcd, /path/to/file2.fst")
                                .on_change(|text| FILE_PATHS_INPUT.set_neq(text))
                                .size(InputSize::Medium)
                                .build()
                        )
                        .item(
                            Row::new()
                                .s(Gap::new().x(12))
                                .s(Align::new().right())
                                .item(
                                    button()
                                        .label("Cancel")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Medium)
                                        .on_press(|| SHOW_FILE_DIALOG.set(false))
                                        .build()
                                )
                                .item(
                                    load_files_dialog_button()
                                )
                        )
                )
        )
}

fn root() -> impl Element {
    Stack::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Background::new().color(hsluv!(220, 15, 8)))
        .layer(main_layout())
        .layer_signal(SHOW_FILE_DIALOG.signal().map_true(
            || file_paths_dialog()
        ))
}

// --- Waveform Viewer Layout ---

fn create_panel(header_content: impl Element, content: impl Element) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Background::new().color(hsluv!(220, 15, 11)))
        .s(RoundedCorners::all(6))
        .s(Borders::all(Border::new().width(1).color(hsluv!(220, 10, 25))))
        .child(
            Column::new()
                .s(Height::fill())
                .item(
                    El::new()
                        .s(Padding::new().x(12).y(8))
                        .s(Background::new().color(hsluv!(220, 15, 13)))
                        .s(Borders::new().bottom(Border::new().width(1).color(hsluv!(220, 10, 25))))
                        .s(RoundedCorners::new().top(6))
                        .s(Font::new().weight(FontWeight::SemiBold).size(14).color(hsluv!(220, 5, 80)))
                        .child(header_content)
                )
                .item(
                    El::new()
                        .s(Height::fill())
                        .s(Scrollbars::both())
                        .child(content)
                )
        )
}

fn app_header() -> impl Element {
    Row::new()
        .s(Height::exact(40))
        .s(Width::fill())
        .s(Background::new().color(hsluv!(220, 15, 12)))
        .s(Borders::new().bottom(Border::new().width(1).color(hsluv!(220, 15, 20))))
        .s(Padding::new().x(16).y(8))
        .item(
            Row::new()
                .s(Gap::new().x(8))
                .s(Align::center())
                .item(
                    button()
                        .label("📁 Load files")
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Small)
                        .on_press(|| show_file_paths_dialog())
                        .build()
                )
        )
        .item(
            El::new()
                .s(Width::fill())
        )
}

fn main_layout() -> impl Element {
    let is_any_divider_dragging = map_ref! {
        let vertical = VERTICAL_DIVIDER_DRAGGING.signal(),
        let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal() =>
        *vertical || *horizontal
    };

    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Scrollbars::both())
        .text_content_selecting_signal(
            is_any_divider_dragging.map(|is_dragging| {
                if is_dragging {
                    TextContentSelecting::none()
                } else {
                    TextContentSelecting::auto()
                }
            })
        )
        .s(Cursor::with_signal(
            map_ref! {
                let vertical = VERTICAL_DIVIDER_DRAGGING.signal(),
                let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal() =>
                if *vertical {
                    Some(CursorIcon::ColumnResize)
                } else if *horizontal {
                    Some(CursorIcon::RowResize)
                } else {
                    None
                }
            }
        ))
        .on_pointer_up(|| {
            VERTICAL_DIVIDER_DRAGGING.set_neq(false);
            HORIZONTAL_DIVIDER_DRAGGING.set_neq(false);
        })
        .on_pointer_leave(|| {
            VERTICAL_DIVIDER_DRAGGING.set_neq(false);
            HORIZONTAL_DIVIDER_DRAGGING.set_neq(false);
        })
        .on_pointer_move_event(|event| {
            if VERTICAL_DIVIDER_DRAGGING.get() {
                LEFT_PANEL_WIDTH.update(|width| {
                    let new_width = width as i32 + event.movement_x();
                    u32::max(50, u32::try_from(new_width).unwrap_or(50))
                });
                save_current_config();
            } else if HORIZONTAL_DIVIDER_DRAGGING.get() {
                if IS_DOCKED_TO_BOTTOM.get() {
                    // In "Docked to Bottom" mode, horizontal divider controls main area height
                    MAIN_AREA_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                    save_current_config();
                } else {
                    // In "Docked to Right" mode, horizontal divider controls files panel height
                    FILES_PANEL_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                    save_current_config();
                }
            }
        })
        .child(docked_layout_wrapper())
}

// Wrapper function that switches between docked and undocked layouts
fn docked_layout_wrapper() -> impl Element {
    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Scrollbars::both())
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            if is_docked {
                // Docked to Bottom layout
                El::new()
                    .s(Height::fill())
                    .s(Scrollbars::both())
                    .child(
                        Column::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(
                                Row::new()
                                    .s(Height::exact_signal(MAIN_AREA_HEIGHT.signal()))
                                    .s(Width::fill())
                                    .item(files_panel_docked())
                                    .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                                    .item(variables_panel_docked())
                            )
                            .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                            .item(selected_variables_with_waveform_panel())
                    )
            } else {
                // Docked to Right layout
                El::new()
                    .s(Height::fill())
                    .s(Scrollbars::both())
                    .child(
                        Row::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(
                                El::new()
                                    .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))
                                    .s(Height::fill())
                                    .child(
                                        Column::new()
                                            .s(Height::fill())
                                            .item(files_panel_with_height())
                                            .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                                            .item(variables_panel_with_fill())
                                    )
                            )
                            .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                            .item(
                                El::new()
                                    .s(Width::fill())
                                    .s(Height::fill())
                                    .child(selected_variables_with_waveform_panel())
                            )
                    )
            }
        }))
}

// Docked layout: Top area (Files & Scopes | Variables) + Bottom area (Selected Variables)
fn docked_layout() -> impl Element {
    Column::new()
        .s(Height::fill())
        .s(Width::fill())
        .item(
            Row::new()
                .s(Height::exact_signal(MAIN_AREA_HEIGHT.signal()))
                .s(Width::fill())
                .item(files_panel_docked())
                .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                .item(variables_panel_docked())
        )
        .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
        .item(selected_variables_with_waveform_panel())
}

// Undocked layout: (Files & Scopes + Variables) | Selected Variables
fn undocked_layout() -> impl Element {
    Row::new()
        .s(Height::fill())
        .s(Width::fill())
        .item(
            Column::new()
                .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))
                .s(Height::fill())
                .item(files_panel_with_height())
                .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                .item(variables_panel_with_fill())
        )
        .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
        .item(selected_variables_with_waveform_panel())
}

// Helper functions for different panel configurations

fn files_panel_with_width() -> impl Element {
    El::new()
        .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))
        .s(Height::fill())
        .child(files_panel())
}

fn files_panel_with_height() -> impl Element {
    El::new()
        .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
        .s(Width::fill())
        .s(Scrollbars::both())
        .child(files_panel())
}

fn variables_panel_with_fill() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Scrollbars::both())
        .child(variables_panel())
}

// Docked mode specific panels with proper sizing
fn files_panel_docked() -> impl Element {
    El::new()
        .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))  // Use draggable width in docked mode too
        .s(Height::fill())
        .s(Scrollbars::both())
        .child(files_panel())
}

fn variables_panel_docked() -> impl Element {
    El::new()
        .s(Width::fill())  // Variables takes remaining space
        .s(Height::fill())
        .s(Scrollbars::both())
        .child(variables_panel())
}

fn remove_all_button() -> impl Element {
    button()
        .label("Remove All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(|| {
            LOADED_FILES.lock_mut().clear();
            FILE_PATHS.lock_mut().clear();
            EXPANDED_SCOPES.lock_mut().clear();
            save_current_config();
        })
        .build()
}

fn theme_toggle_button() -> impl Element {
    El::new()
        .child_signal(theme().map(|current_theme| {
            button()
                .left_icon(match current_theme {
                    Theme::Light => IconName::Moon,
                    Theme::Dark => IconName::Sun,
                })
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Small)
                .on_press(|| toggle_theme())
                .build()
                .into_element()
        }))
}

fn dock_toggle_button() -> impl Element {
    El::new()
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            button()
                .label(if is_docked { "Dock to Right" } else { "Dock to Bottom" })
                .left_icon_element(|| {
                    El::new()
                        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
                            let icon_el = icon(IconName::ArrowDownToLine).size(IconSize::Small).build();
                            if is_docked {
                                El::new()
                                    .s(Transform::new().rotate(-90))
                                    .child(icon_el)
                                    .into_element()
                            } else {
                                El::new().child(icon_el).into_element()
                            }
                        }))
                        .unify()
                })
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(|| {
                    let new_is_docked = !IS_DOCKED_TO_BOTTOM.get();
                    IS_DOCKED_TO_BOTTOM.set_neq(new_is_docked);
                    
                    // Load appropriate panel sizes for the new mode
                    if let Some(config) = LOADED_CONFIG.lock_ref().clone() {
                        if new_is_docked {
                            // Switching to "Docked to Bottom" mode
                            LEFT_PANEL_WIDTH.set_neq(u32::max(50, config.workspace.docked_to_bottom.files_panel_width));
                            MAIN_AREA_HEIGHT.set_neq(u32::max(50, config.workspace.docked_to_bottom.main_area_height));
                        } else {
                            // Switching to "Docked to Right" mode  
                            LEFT_PANEL_WIDTH.set_neq(u32::max(50, config.workspace.docked_to_right.files_panel_width));
                            FILES_PANEL_HEIGHT.set_neq(u32::max(50, config.workspace.docked_to_right.files_panel_height));
                        }
                    } else {
                        // Fallback to defaults if no config available
                        if new_is_docked {
                            LEFT_PANEL_WIDTH.set_neq(470);
                            MAIN_AREA_HEIGHT.set_neq(350);
                        } else {
                            LEFT_PANEL_WIDTH.set_neq(470);
                            FILES_PANEL_HEIGHT.set_neq(300);
                        }
                    }
                    
                    save_current_config();
                })
                .align(Align::center())
                .build()
                .into_element()
        }))
}

fn convert_files_to_tree_data(files: &[WaveformFile]) -> Vec<TreeViewItemData> {
    files.iter().map(|file| {
        let children = file.scopes.iter().map(|scope| {
            convert_scope_to_tree_data(scope)
        }).collect();
        
        TreeViewItemData::new(file.id.clone(), file.filename.clone())
            .item_type(TreeViewItemType::File)
            .with_children(children)
    }).collect()
}

fn convert_scope_to_tree_data(scope: &ScopeData) -> TreeViewItemData {
    let mut children = Vec::new();
    
    // Add child scopes first
    for child_scope in &scope.children {
        children.push(convert_scope_to_tree_data(child_scope));
    }
    
    TreeViewItemData::new(scope.id.clone(), scope.name.clone())
        .item_type(TreeViewItemType::Folder)
        .with_children(children)
}

fn files_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Files & Scopes")
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        load_files_button_with_progress(
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            Some(IconName::Folder)
                        )
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        remove_all_button()
                    ),
                Column::new()
                    .s(Gap::new().y(4))
                    .s(Padding::all(12))
                    .s(Height::fill())  // Make the column fill available height
                    .item(
                        El::new()
                            .s(Height::fill())
                            .child_signal(
                                LOADED_FILES.signal_vec_cloned()
                                    .to_signal_map(|files| {
                                        let tree_data = convert_files_to_tree_data(&files);
                                        
                                        if tree_data.is_empty() {
                                            // Show placeholder when no files loaded
                                            El::new()
                                                .s(Padding::all(20))
                                                .s(Font::new().color(hsluv!(0, 0, 50)).italic())
                                                .child("No files loaded. Click 'Load Files' to add waveform files.")
                                                .unify()
                                        } else {
                                            // Show TreeView with loaded files 
                                            tree_view()
                                                .data(tree_data)
                                                .size(TreeViewSize::Medium)
                                                .variant(TreeViewVariant::Basic)
                                                .show_icons(true)
                                                .show_checkboxes(true)
                                                .external_expanded(EXPANDED_SCOPES.clone())
                                                .external_selected(TREE_SELECTED_ITEMS.clone())
                                                .build()
                                                .unify()
                                        }
                                    })
                            )
                    )
            )
        )
}

fn get_all_variables_from_files() -> Vec<Signal> {
    let mut all_variables = Vec::new();
    for file in LOADED_FILES.lock_ref().iter() {
        collect_variables_from_scopes(&file.scopes, &mut all_variables);
    }
    // CRITICAL: Sort variables alphabetically to ensure consistent virtual scrolling order
    all_variables.sort_by(|a, b| a.name.cmp(&b.name));
    all_variables
}

fn get_variables_from_selected_scope(selected_scope_id: &str) -> Vec<Signal> {
    for file in LOADED_FILES.lock_ref().iter() {
        if let Some(mut variables) = find_variables_in_scope(&file.scopes, selected_scope_id) {
            // CRITICAL: Sort variables alphabetically to ensure consistent virtual scrolling order
            variables.sort_by(|a, b| a.name.cmp(&b.name));
            return variables;
        }
    }
    Vec::new()
}

fn find_variables_in_scope(scopes: &[ScopeData], scope_id: &str) -> Option<Vec<Signal>> {
    for scope in scopes {
        if scope.id == scope_id {
            return Some(scope.variables.clone());
        }
        if let Some(variables) = find_variables_in_scope(&scope.children, scope_id) {
            return Some(variables);
        }
    }
    None
}

fn collect_variables_from_scopes(scopes: &[ScopeData], variables: &mut Vec<Signal>) {
    for scope in scopes {
        variables.extend(scope.variables.clone());
        collect_variables_from_scopes(&scope.children, variables);
    }
}

fn file_contains_scope(scopes: &[ScopeData], scope_id: &str) -> bool {
    for scope in scopes {
        if scope.id == scope_id {
            return true;
        }
        
        // Recursively search in child scopes
        if file_contains_scope(&scope.children, scope_id) {
            return true;
        }
    }
    false
}

fn count_variables_in_scopes(scopes: &[ScopeData]) -> usize {
    let mut count = 0;
    for scope in scopes {
        count += scope.variables.len();
        count += count_variables_in_scopes(&scope.children);
    }
    count
}

fn filter_variables(variables: &[Signal], search_filter: &str) -> Vec<Signal> {
    if search_filter.is_empty() {
        variables.to_vec()
    } else {
        let filter_lower = search_filter.to_lowercase();
        let mut filtered: Vec<Signal> = variables.iter()
            .filter(|var| var.name.to_lowercase().contains(&filter_lower))
            .cloned()
            .collect();
        // CRITICAL: Ensure filtered variables remain alphabetically sorted
        filtered.sort_by(|a, b| a.name.cmp(&b.name));
        filtered
    }
}

fn variables_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Variables")
                    )
                    .item(
                        El::new()
                            .s(Font::new().no_wrap().color(hsluv!(220, 10, 60)).size(13))
                            .child_signal(
                                map_ref! {
                                    let selected_scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
                                    let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned() =>
                                    {
                                        if let Some(scope_id) = selected_scope_id {
                                            let variables = get_variables_from_selected_scope(&scope_id);
                                            let filtered_variables = filter_variables(&variables, &search_filter);
                                            filtered_variables.len().to_string()
                                        } else {
                                            "0".to_string()
                                        }
                                    }
                                }
                            )
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        input()
                            .placeholder("variable_name")
                            .left_icon(IconName::Search)
                            .size(InputSize::Small)
                            .on_change(|text| VARIABLES_SEARCH_FILTER.set_neq(text))
                            .build()
                    ),
                simple_variables_content()
            )
        )
}


fn simple_variables_content() -> impl Element {
    Column::new()
        .s(Gap::new().y(0))
        .s(Padding::all(12))
        .s(Height::fill())
        .s(Width::fill())
        .item(
            El::new()
                .s(Height::fill())
                .s(Width::fill())
                .child_signal(
                    map_ref! {
                        let selected_scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
                        let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned() =>
                        {
                            if let Some(scope_id) = selected_scope_id {
                                let variables = get_variables_from_selected_scope(&scope_id);
                                virtual_variables_list(variables, search_filter.clone()).into_element()
                            } else {
                                virtual_variables_list(Vec::new(), "Select a scope to view variables".to_string()).into_element()
                            }
                        }
                    }
                )
        )
}


fn virtual_variables_list(variables: Vec<Signal>, search_filter: String) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // Handle special cases first (empty states)
    if variables.is_empty() && search_filter.starts_with("Select a scope") {
        return Column::new()
            .s(Gap::new().y(4))
            .item(
                El::new()
                    .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                    .child(search_filter)
            );
    }
    
    if variables.is_empty() {
        return Column::new()
            .s(Gap::new().y(4))
            .item(
                El::new()
                    .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                    .child("No variables in selected scope")
            );
    }
    
    // Apply search filter
    let filtered_variables = filter_variables(&variables, &search_filter);
    
    if filtered_variables.is_empty() {
        return Column::new()
            .s(Gap::new().y(4))
            .item(
                El::new()
                    .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                    .child("No variables match search filter")
            );
    }
    
    // FIXED-HEIGHT VIRTUAL LIST - only render ~15 visible items
    // PHASE 1 TEST: Use signal-based version with always(400.0)
    // SIMPLE FILL: Use Height::fill() directly (works now that Column hierarchy is fixed)
    rust_virtual_variables_list_simple_fill(filtered_variables)
}

fn rust_virtual_variables_list_simple_fill(variables: Vec<Signal>) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // DYNAMIC HEIGHT SOLUTION: Parent-child pattern with real viewport monitoring
    let height_mutable = Mutable::new(400u32); // Start with reasonable default
    let virtual_list_height = Broadcaster::new(height_mutable.signal());
    
    Column::new()
        .s(Height::fill()) // Parent fills available space
        .s(Width::fill())
        .item(
            El::new()
                .s(Width::fill())
                .s(Height::fill()) // Monitor parent claims all available height
                .on_viewport_size_change({
                    let height_mutable = height_mutable.clone();
                    move |_width, height| {
                        // Remove height cap to allow unlimited panel height (Step 1)
                        let constrained_height = (height as f64).max(100.0) as u32;
                        zoon::println!("DYNAMIC: Parent height={}, constrained={}", height, constrained_height);
                        height_mutable.set_neq(constrained_height);
                    }
                })
                .child(
                    // Child uses exact height from parent measurement
                    rust_virtual_variables_list_with_signal(variables, virtual_list_height)
                )
        )
}

fn simple_variables_list(variables: Vec<Signal>, search_filter: String) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // Special case for displaying a message when called with empty variables and a message
    if variables.is_empty() && search_filter.starts_with("Select a scope") {
        return Column::new()
            .s(Gap::new().y(4))
            .item(
                El::new()
                    .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                    .child(search_filter)
            );
    }
    
    // Apply search filter
    let filtered_variables = filter_variables(&variables, &search_filter);
    
    if variables.is_empty() {
        Column::new()
            .s(Gap::new().y(4))
            .item(
                El::new()
                    .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                    .child("No variables in selected scope")
            )
    } else if filtered_variables.is_empty() {
        Column::new()
            .s(Gap::new().y(4))
            .item(
                El::new()
                    .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                    .child("No variables match search filter")
            )
    } else {
        // Simple list showing all variables - clean and working
        Column::new()
            .s(Gap::new().y(0))
            .items(filtered_variables.into_iter().map(|signal| {
                simple_variable_row(signal)
            }))
    }
}

fn simple_variable_row(signal: Signal) -> Row<row::EmptyFlagNotSet, row::MultilineFlagNotSet, RawHtmlEl> {
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Padding::new().x(12).y(2))
        .item(
            El::new()
                .s(Font::new().color(hsluv!(220, 10, 85)).size(14))
                .s(Font::new().no_wrap())
                .child(signal.name.clone())
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color(hsluv!(210, 80, 70)).size(12))
                .s(Font::new().no_wrap())
                .child(format!("{} {}-bit", signal.signal_type, signal.width))
        )
}

// ============================================================================
// VIRTUAL VARIABLES LIST - FIXED HEIGHT IMPLEMENTATION
// ============================================================================
// 
// CURRENT STATUS: WORKING with fixed 400px height
// GOAL: Convert to dynamic height while maintaining scroll functionality
// 
// CORE CHALLENGE: When switching from Height::exact(400) to Height::fill(),
// the container gets clientHeight=0, scrollHeight=0 which breaks scrolling
// even though height detection and item rendering works correctly.
//
// This implementation uses the Stack+Transform pattern from the working backup
// to ensure reliable scrolling behavior.
// ============================================================================

fn rust_virtual_variables_list(variables: Vec<Signal>) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    let total_items = variables.len();
    let item_height = 24.0; // Fixed height per item - DO NOT CHANGE
    
    // ===== HEIGHT MANAGEMENT =====
    // CURRENT: Fixed 400px height (WORKING)
    // DYNAMIC: Should be updated by viewport monitoring
    let container_height = Mutable::new(400.0); // FIXED HEIGHT - change for dynamic
    
    // ===== VISIBLE ITEM CALCULATIONS =====
    // Calculate how many items fit in the container + buffer
    // Buffer of +5 items ensures smooth scrolling
    let initial_visible_count = ((400.0_f64 / item_height).ceil() as usize + 5).min(total_items);
    let visible_count = Mutable::new(initial_visible_count); // For future dynamic updates
    
    // ===== VIRTUAL SCROLLING STATE =====
    // These track the current scroll position and visible range
    let scroll_top = Mutable::new(0.0);        // Current scroll offset in pixels
    let visible_start = Mutable::new(0usize);  // First visible item index
    let visible_end = Mutable::new(initial_visible_count.min(total_items)); // Last visible item index
    
    // ===== DYNAMIC HEIGHT INFRASTRUCTURE (PREPARED BUT DISABLED) =====
    // TODO: Enable this when solving the clientHeight=0 issue
    // This would make visible_count reactive to container height changes
    /*
    Task::start({
        let container_height = container_height.clone();
        let visible_count = visible_count.clone();
        async move {
            container_height.signal().for_each_sync(move |height| {
                let new_count = ((height / item_height).ceil() as usize + 5).min(total_items);
                zoon::println!("Height changed to {}, new visible_count: {}", height, new_count);
                visible_count.set_neq(new_count);
            }).await
        }
    });
    */
    
    zoon::println!("Virtual List: {} total, {} initial visible [TEST]", total_items, initial_visible_count);
    
    Column::new()
        .item(
            // ===== SCROLL CONTAINER =====
            // This El creates the scrollable area with fixed dimensions
            // CRITICAL: Height::exact(400) creates proper clientHeight for scrolling
            // PROBLEM: Height::fill() results in clientHeight=0, breaking scroll
            El::new()
                .s(Width::fill())
                .s(Height::exact(400))  // FIXED HEIGHT - WORKING
                // .s(Height::fill())   // DYNAMIC HEIGHT - BREAKS SCROLLING
                .s(Background::new().color(hsluv!(220, 15, 11)))
                .s(RoundedCorners::all(8))
                .s(Padding::all(4))
                // ===== VIEWPORT SIZE MONITORING (DISABLED) =====
                // This would track container size changes for dynamic height
                // PROBLEM: Works for height detection but breaks scrolling when combined with Height::fill()
                /*
                .on_viewport_size_change({
                    let container_height = container_height.clone();
                    move |_width, height| {
                        // Use reasonable height constraints to prevent viewport size bugs
                        let actual_height = (height as f64).max(100.0).min(800.0); // Reasonable bounds
                        zoon::println!("Virtual list height: raw={}, constrained={}", height, actual_height);
                        container_height.set_neq(actual_height);
                    }
                })
                */
                // ===== DOM MANIPULATION & SCROLL SETUP =====
                // This sets up the actual scrollable DOM element
                .update_raw_el({
                    let scroll_top = scroll_top.clone();
                    let visible_start = visible_start.clone();
                    let visible_end = visible_end.clone();
                    let visible_count = visible_count.clone();
                    let variables = variables.clone();
                    
                    move |el| {
                        // ===== SCROLL CONTAINER SETUP =====
                        // Configure the DOM element for scrolling
                        if let Some(html_el) = el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                            html_el.set_id("virtual-container"); // Unique ID for scroll event targeting
                            html_el.style().set_property("overflow-y", "auto").unwrap(); // Enable vertical scrolling
                            html_el.style().set_property("display", "block").unwrap(); // Block layout for proper sizing
                            
                            // ===== CRITICAL DIAGNOSTIC =====
                            // These values show the core problem: clientHeight=0 when using Height::fill()
                            zoon::println!("Virtual container setup: clientHeight={}, scrollHeight={}", 
                                html_el.client_height(), html_el.scroll_height());
                            
                            // ===== SCROLL EVENT HANDLER =====
                            // This handles scroll events and updates the visible range
                            let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                                let scroll_top = scroll_top.clone();
                                let visible_start = visible_start.clone();
                                let visible_end = visible_end.clone();
                                let visible_count = visible_count.clone();
                                
                                move |_event: web_sys::Event| {
                                    // Find the scroll container element
                                    if let Some(scroll_el) = web_sys::window()
                                        .and_then(|w| w.document())
                                        .and_then(|d| d.get_element_by_id("virtual-container"))
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) 
                                    {
                                        // Get current scroll position
                                        let new_scroll_top = scroll_el.scroll_top() as f64;
                                        scroll_top.set_neq(new_scroll_top);
                                        
                                        // ===== VIRTUAL RANGE CALCULATION =====
                                        // Calculate which items should be visible based on scroll position
                                        let start_index = (new_scroll_top / item_height).floor() as usize;
                                        // CURRENT: Uses fixed initial_visible_count (WORKING)
                                        // DYNAMIC: Should use visible_count.get() for reactive height
                                        let end_index = (start_index + initial_visible_count).min(total_items);
                                        // let end_index = (start_index + visible_count.get()).min(total_items); // FOR DYNAMIC
                                        
                                        // Update the visible range state
                                        visible_start.set_neq(start_index);
                                        visible_end.set_neq(end_index);
                                        
                                        // Diagnostic logging for scroll behavior
                                        zoon::println!("Scroll: top={}, start={}, end={}, visible_count={}", 
                                            new_scroll_top, start_index, end_index, initial_visible_count);
                                    }
                                }
                            }) as Box<dyn FnMut(_)>);
                            
                            // ===== SCROLL EVENT REGISTRATION =====
                            // Attach the scroll handler to the DOM element
                            html_el.add_event_listener_with_callback(
                                "scroll",
                                scroll_closure.as_ref().unchecked_ref()
                            ).unwrap();
                            
                            // Prevent closure from being garbage collected
                            scroll_closure.forget();
                            
                            // ===== POST-SETUP DIAGNOSTIC =====
                            // Check if the container has proper scroll dimensions
                            // WORKING: clientHeight=400, scrollHeight=large_number
                            // BROKEN: clientHeight=0, scrollHeight=0 (when using Height::fill())
                            zoon::println!("Virtual container after setup: clientHeight={}, scrollHeight={}", 
                                html_el.client_height(), html_el.scroll_height());
                            
                        }
                        
                        el
                    }
                })
                .child(
                    // ===== VIRTUAL CONTENT AREA =====
                    // This El represents the total scrollable content height
                    // Its height = total_items * item_height, creating the scroll thumb size
                    El::new()
                        .s(Width::fill())
                        .s(Height::exact((total_items as f64 * item_height) as u32)) // Total virtual height
                        .child_signal(
                            // ===== REACTIVE CONTENT RENDERING =====
                            // This signal updates whenever the visible range changes
                            map_ref! {
                                let start = visible_start.signal(),
                                let end = visible_end.signal() => {
                                    // Optional: Debug visible range changes
                                    // zoon::println!("Rendering virtual items: start={}, end={}, count={}", start, end, end - start);
                                    
                                    // ===== STACK + TRANSFORM PATTERN =====
                                    // Uses Stack with Transform positioning (from working backup)
                                    // This pattern ensures proper layered rendering
                                    Stack::new()
                                        .s(Width::fill())
                                        .s(Height::exact((total_items as f64 * item_height) as u32)) // Match parent height
                                        .layers(
                                            // ===== VISIBLE ITEM RENDERING =====
                                            // Only render items in the visible range [start..end]
                                            variables[*start..*end].iter().enumerate().map(|(i, signal)| {
                                                // Calculate absolute position in the full list
                                                let absolute_index = *start + i;
                                                // Position each item using Transform (absolute positioning)
                                                virtual_variable_row_positioned(signal.clone(), absolute_index as f64 * item_height)
                                            })
                                        )
                                        .into_element() // Convert to unified Element type
                                }
                            }
                        )
                )
        )
}

// ============================================================================
// VIRTUAL VARIABLE ROW - POSITIONED ITEM RENDERING
// ============================================================================
// 
// Renders a single variable row with absolute positioning using Transform.
// This is the individual item template used by the virtual list.
// 
// CRITICAL: Uses Transform::move_down() for absolute positioning within Stack
// This pattern allows Stack to overlay items at specific pixel offsets.
// ============================================================================

fn virtual_variable_row_positioned(signal: Signal, top_offset: f64) -> impl Element {
    Row::new()
        .s(Gap::new().x(8))                                      // Horizontal spacing between elements
        .s(Width::fill())                                        // Full width within container
        .s(Height::exact(24))                                    // Fixed height per item (matches item_height)
        .s(Transform::new().move_down(top_offset as i32))        // CRITICAL: Absolute positioning within Stack
        .s(Padding::new().x(12).y(2))                           // Internal padding
        .s(Background::new().color(hsluv!(220, 15, 12)))         // Row background color
        .item(
            // ===== VARIABLE NAME =====
            El::new()
                .s(Font::new().color(hsluv!(220, 10, 85)).size(14))  // Text styling
                .s(Font::new().no_wrap())                             // Prevent text wrapping
                .child(signal.name.clone())                           // Display variable name
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color(hsluv!(210, 80, 70)).size(12))
                .s(Font::new().no_wrap())
                .child(format!("{} {}-bit", signal.signal_type, signal.width))
        )
}

// ============================================================================
// VIRTUAL VARIABLES LIST - SIGNAL-BASED HEIGHT (PHASE 1)
// ============================================================================
// 
// NEW APPROACH: Parent-Child Wrapper Pattern
// - Parent: Height::fill() + viewport monitoring (responsive)
// - Child: Height::exact_signal() (scrollable with exact dimensions)
// 
// PHASE 1: Test with always(400.0) - should work identically to fixed version
// PHASE 2: Connect to parent wrapper with dynamic height signal
// ============================================================================

fn rust_virtual_variables_list_with_signal(
    variables: Vec<Signal>,
    height_signal: Broadcaster<MutableSignal<u32>>
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    let total_items = variables.len();
    let item_height = 24.0;
    
    // Get initial visible count for scroll handler  
    let initial_visible_count = ((400.0_f64 / item_height).ceil() as usize + 5).min(total_items);
    
    // ===== VIRTUAL SCROLLING STATE =====
    let scroll_top = Mutable::new(0.0);
    let visible_start = Mutable::new(0usize);
    let visible_end = Mutable::new(initial_visible_count.min(total_items));
    
    // ===== STEP 2: REACTIVE VISIBLE COUNT =====
    // This will be updated when height changes (Step 3 will connect it)
    let visible_count = Mutable::new(initial_visible_count);
    
    // ===== STEP 3: HEIGHT SIGNAL LISTENER =====
    // Listen to height changes and recalculate visible count
    Task::start({
        let height_signal = height_signal.clone();
        let visible_count = visible_count.clone();
        async move {
            height_signal.signal().for_each(|height| {
                let new_visible_count = ((height as f64 / item_height).ceil() as usize + 5).min(total_items);
                zoon::println!("Height changed: {}px -> {} visible items", height, new_visible_count);
                visible_count.set_neq(new_visible_count);
                async {}
            }).await;
        }
    });
    
    // ===== STEP 4: UPDATE visible_end WHEN visible_count CHANGES =====
    // When visible_count changes, update visible_end to maintain current view
    Task::start({
        let visible_count = visible_count.clone();
        let visible_start = visible_start.clone();
        let visible_end = visible_end.clone();
        async move {
            visible_count.signal().for_each(move |new_count| {
                let current_start = visible_start.get();
                let new_end = (current_start + new_count).min(total_items);
                visible_end.set_neq(new_end);
                async {}
            }).await;
        }
    });
    
    zoon::println!("Virtual List (Signal): {} total, {} initial visible [SIGNAL-TEST]", total_items, initial_visible_count);
    
    Column::new()
        .item(
            // ===== SCROLL CONTAINER WITH SIGNAL HEIGHT =====
            // CRITICAL: Uses Height::exact_signal() instead of Height::exact()
            El::new()
                .s(Width::fill())
                .s(Height::exact_signal(height_signal.signal()))  // 🔥 KEY CHANGE: Signal-based height
                .s(Background::new().color(hsluv!(220, 15, 11)))
                .s(RoundedCorners::all(8))
                .s(Padding::all(4))
                .update_raw_el({
                    let scroll_top = scroll_top.clone();
                    let visible_start = visible_start.clone();
                    let visible_end = visible_end.clone();
                    let variables = variables.clone();
                    
                    move |el| {
                        if let Some(html_el) = el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                            html_el.set_id("virtual-container-signal");
                            html_el.style().set_property("overflow-y", "auto").unwrap();
                            html_el.style().set_property("display", "block").unwrap();
                            
                            zoon::println!("Virtual container (SIGNAL) setup: clientHeight={}, scrollHeight={}", 
                                html_el.client_height(), html_el.scroll_height());
                            
                            let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                                let scroll_top = scroll_top.clone();
                                let visible_start = visible_start.clone();
                                let visible_end = visible_end.clone();
                                
                                move |_event: web_sys::Event| {
                                    if let Some(scroll_el) = web_sys::window()
                                        .and_then(|w| w.document())
                                        .and_then(|d| d.get_element_by_id("virtual-container-signal"))
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) 
                                    {
                                        let new_scroll_top = scroll_el.scroll_top() as f64;
                                        scroll_top.set_neq(new_scroll_top);
                                        
                                        let start_index = (new_scroll_top / item_height).floor() as usize;
                                        // STEP 4: Use reactive visible_count instead of static initial_visible_count
                                        let end_index = (start_index + visible_count.get()).min(total_items);
                                        
                                        visible_start.set_neq(start_index);
                                        visible_end.set_neq(end_index);
                                        
                                        zoon::println!("Scroll (SIGNAL): top={}, start={}, end={} (visible_count={})", 
                                            new_scroll_top, start_index, end_index, visible_count.get());
                                    }
                                }
                            }) as Box<dyn FnMut(_)>);
                            
                            html_el.add_event_listener_with_callback(
                                "scroll",
                                scroll_closure.as_ref().unchecked_ref()
                            ).unwrap();
                            
                            scroll_closure.forget();
                            
                            zoon::println!("Virtual container (SIGNAL) after setup: clientHeight={}, scrollHeight={}", 
                                html_el.client_height(), html_el.scroll_height());
                        }
                        
                        el
                    }
                })
                .child(
                    El::new()
                        .s(Width::fill())
                        .s(Height::exact((total_items as f64 * item_height) as u32))
                        .child_signal(
                            map_ref! {
                                let start = visible_start.signal(),
                                let end = visible_end.signal() => {
                                    Stack::new()
                                        .s(Width::fill())
                                        .s(Height::exact((total_items as f64 * item_height) as u32))
                                        .layers(
                                            variables[*start..*end].iter().enumerate().map(|(i, signal)| {
                                                let absolute_index = *start + i;
                                                virtual_variable_row_positioned(signal.clone(), absolute_index as f64 * item_height)
                                            })
                                        )
                                        .into_element()
                                }
                            }
                        )
                )
        )
}

fn virtual_variable_row(signal: Signal) -> impl Element {
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Padding::new().x(12).y(2))
        .s(Background::new().color(hsluv!(220, 15, 12)))
        .item(
            El::new()
                .s(Font::new().color(hsluv!(220, 10, 85)).size(14))
                .s(Font::new().no_wrap())
                .child(signal.name.clone())
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color(hsluv!(210, 80, 70)).size(12))
                .s(Font::new().no_wrap())
                .child(format!("{} {}-bit", signal.signal_type, signal.width))
        )
}





fn vertical_divider(is_dragging: Mutable<bool>) -> impl Element {
    El::new()
        .s(Width::exact(4))  // Back to original 4px width
        .s(Height::fill())
        .s(Background::new().color_signal(
            is_dragging.signal().map_bool(
                || hsluv!(220, 100, 75), // Brighter blue when dragging
                || hsluv!(220, 85, 60)   // Default blue matching Figma exactly
            )
        ))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))  // Ensure no padding interferes
        .on_pointer_down(move || is_dragging.set_neq(true))
}

fn horizontal_divider(is_dragging: Mutable<bool>) -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::exact(4))
        .s(Background::new().color_signal(
            is_dragging.signal().map_bool(
                || hsluv!(220, 100, 75), // Brighter blue when dragging
                || hsluv!(220, 85, 60)   // Default blue matching Figma exactly
            )
        ))
        .s(Cursor::new(CursorIcon::RowResize))
        .on_pointer_down(move || is_dragging.set_neq(true))
}


fn selected_variables_with_waveform_panel() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Selected Variables")
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        theme_toggle_button()
                    )
                    .item(
                        dock_toggle_button()
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        remove_all_button()
                    ),
                // 3-column table layout: Variable Name | Value | Waveform
                El::new()
                    .s(Height::fill())
                    .child(
                        Column::new()
                            .s(Gap::new().y(0))
                            .s(Padding::all(8))
                            .item(
                                // Timeline header
                        Row::new()
                            .s(Gap::new().x(0))
                            .s(Align::new().center_y())
                            .s(Padding::new().y(2))
                            .item(
                                // Variable Name column header
                                El::new()
                                    .s(Width::exact(250))
                                    .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                    .child("Variable")
                            )
                            .item(
                                // Value column header  
                                El::new()
                                    .s(Width::exact(60))
                                    .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                    .child("Value")
                            )
                            .item(
                                // Timeline markers for waveform column
                                Row::new()
                                    .s(Width::fill())
                                    .s(Gap::new().x(40))
                                    .s(Padding::new().x(10))
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("0s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("10s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("20s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("30s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("40s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("50s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("60s")
                                    )
                            )
                    )
                    .items((0..8).map(|i| {
                        let var_names = [
                            "LsuPlugin_logic_bus_rsp_payload_error",
                            "LsuPlugin_logic_bus_rsp_payload_data",
                            "io_writes_0_payload_data", 
                            "logic_logic_onDebugCd_dmiStat_value_string",
                            "LsuPlugin_logic_bus_rsp_payload_error",
                            "LsuPlugin_logic_bus_rsp_payload_data",
                            "io_writes_0_payload_data",
                            "clk"
                        ];
                        
                        let values = ["0", "14x2106624", "0", "success", "0", "14x2106624", "0", "1"];
                        
                        // Each row: Variable Name | Value | Waveform
                        Row::new()
                            .s(Gap::new().x(0))
                            .s(Align::new().center_y())
                            .s(Padding::new().y(0))
                            .item(
                                // Variable Name column (250px width)
                                Row::new()
                                    .s(Width::exact(250))
                                    .s(Gap::new().x(8))
                                    .s(Align::new().center_y())
                                    .item("⋮⋮")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child(var_names[i as usize])
                                    )
                                    )
                            .item(
                                // Value column (60px width)
                                El::new()
                                    .s(Width::exact(60))
                                    .s(Font::new().color(hsluv!(220, 10, 75)).size(13))
                                    .child(values[i as usize])
                            )
                            .item(
                                // Waveform column (fills remaining width)
                                Row::new()
                                    .s(Width::fill())
                                    .s(Height::exact(20))
                                    .s(Gap::new().x(1))
                                    .s(Padding::new().x(10))
                                    .items((0..12).map(|j| {
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::exact(18))
                                            .s(Background::new().color(
                                                if (i + j) % 3 == 0 {
                                                    hsluv!(220, 80, 55) // Bright blue
                                                } else if (i + j) % 2 == 0 {
                                                    hsluv!(220, 60, 45) // Medium blue  
                                                } else {
                                                    hsluv!(220, 15, 8) // Dark background
                                                }
                                            ))
                                            .s(RoundedCorners::all(2))
                                    }))
                            )
                    }))
                    )
            )
        )
}

fn selected_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(10))
                    .item(
                        Text::new("Selected Variables")
                    )
                    .item(
                        dock_toggle_button()
                    ),
                Column::new()
                    .s(Gap::new().y(8))
                    .s(Padding::all(16))
                    .item(
                        Row::new()
                            .s(Gap::new().x(8))
                            .s(Align::new().center_y())
                            .item("⋮⋮")
                            .item(
                                El::new()
                                    .s(Font::new().color(hsluv!(0, 0, 80)).size(14))
                                    .child("clock")
                            )
                            .item(
                                button()
                                    .label("×")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Small)
                                    .on_press(|| {})
                                    .build()
                            )
                    )
                    .item(
                        Row::new()
                            .s(Gap::new().x(8))
                            .s(Align::new().center_y())
                            .item("⋮⋮")
                            .item(
                                El::new()
                                    .s(Font::new().color(hsluv!(0, 0, 80)).size(14))
                                    .child("reset")
                            )
                            .item(
                                button()
                                    .label("×")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Small)
                                    .on_press(|| {})
                                    .build()
                            )
                    )
            )
        )
}

fn waveform_panel() -> impl Element {
    El::new()
        .s(Width::fill().min(500))
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(10))
                    .item(
                        Text::new("Waveform")
                    )
                    .item(
                        button()
                            .label("Zoom In")
                            .left_icon(IconName::ZoomIn)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| {})
                            .build()
                    )
                    .item(
                        button()
                            .label("Zoom Out")
                            .left_icon(IconName::ZoomOut)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| {})
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(16))
                    .s(Padding::all(16))
                    .item(
                        Row::new()
                            .s(Gap::new().x(20))
                            .item("0s")
                            .item("10s")
                            .item("20s")
                            .item("30s")
                            .item("40s")
                            .item("50s")
                    )
                    .item(
                        El::new()
                            .s(Background::new().color(hsluv!(0, 0, 15)))
                            .s(Height::exact(200))
                            .s(Width::fill())
                            .s(Align::center())
                            .s(RoundedCorners::all(4))
                            .child(
                                El::new()
                                    .s(Font::new().color(hsluv!(0, 0, 50)).size(16))
                                    .child("Waveform display area")
                            )
                    )
            )
        )
}



// ============================================================================
// VIRTUAL VARIABLES LIST - DYNAMIC WRAPPER (PHASE 2)
// ============================================================================
// 
// PARENT-CHILD PATTERN:
// - Parent: Height::fill() + viewport monitoring (responsive, no scrolling)
// - Child: Height::exact_signal() (exact dimensions, scrollable)
// - Signal Bridge: Connects parent size changes to child height
// 
// This should solve the clientHeight=0 issue by separating concerns:
// - Parent handles responsive sizing without needing to scroll
// - Child handles scrolling with exact pixel dimensions
// ============================================================================

fn rust_virtual_variables_list_dynamic_wrapper(
    variables: Vec<Signal>
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // ===== SIGNAL BRIDGE =====
    // This Broadcaster allows parent to control child height
    let height_mutable = Mutable::new(400u32);
    let virtual_list_height = Broadcaster::new(height_mutable.signal());
    
    // ===== TEST: ADD HEIGHT::FILL() TO COLUMN =====
    // The Column itself needs Height::fill() to claim parent space!
    Column::new()
        .s(Height::fill())           // 🔥 ADD THIS TO COLUMN!
        .item(
            El::new()
                .s(Width::fill())
                .s(Height::fill())
                .on_viewport_size_change({
                    let height_mutable = height_mutable.clone();
                    move |_width, height| {
                        let constrained_height = (height as f64).max(100.0).min(800.0) as u32;
                        zoon::println!("FIXED Column+El: raw={}, constrained={}", height, constrained_height);
                        height_mutable.set_neq(constrained_height);
                    }
                })
                .child(
                    rust_virtual_variables_list_with_signal(variables, virtual_list_height)
                )
        )
}
