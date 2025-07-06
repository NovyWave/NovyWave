use zoon::{*, futures_util::future::try_join_all};
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::theme::{Theme, init_theme, theme, toggle_theme};
use serde::{Serialize, Deserialize};
use std::collections::{HashSet, HashMap};
use web_sys::Performance;

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
        zoon::println!("⏱️ [PERF] {} - Started", label);
        Self {
            start_time,
            label: label.to_string(),
        }
    }
    
    fn elapsed(&self) -> f64 {
        now() - self.start_time
    }
    
    fn log_elapsed(&self) {
        let elapsed = self.elapsed();
        zoon::println!("⏱️ [PERF] {} - Elapsed: {:.2}ms", self.label, elapsed);
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
    
    zoon::println!("Selected file paths: {:?}", paths);
    
    if !paths.is_empty() {
        IS_LOADING.set(true);
    }
    
    for path in paths {
        zoon::println!("Loading file: {}", path);
        
        // Generate file ID and store path mapping for config persistence
        let file_id = generate_file_id(&path);
        FILE_PATHS.lock_mut().insert(file_id, path.clone());
        
        send_up_msg(UpMsg::LoadWaveformFile(path));
    }
    
    SHOW_FILE_DIALOG.set(false);
}

static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    Connection::new(|down_msg, _| {
        zoon::println!("Received DownMsg: {:?}", down_msg);
        match down_msg {
            DownMsg::ParsingStarted { file_id, filename } => {
                zoon::println!("Started parsing file: {} ({})", filename, file_id);
                
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
                zoon::println!("File {} progress: {}%", file_id, progress * 100.0);
                
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
                zoon::println!("File loaded: {} with {} files", file_id, hierarchy.files.len());
                
                // Add loaded files to the TreeView state
                for file in hierarchy.files {
                    let total_variables = count_variables_in_scopes(&file.scopes);
                    zoon::println!("  - {}: {} variables", file.filename, total_variables);
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
                zoon::println!("Config loaded: {:?}", config);
                apply_config(config);
            }
            DownMsg::ConfigSaved => {
                zoon::println!("Config saved successfully");
            }
            DownMsg::ConfigError(error) => {
                zoon::println!("Config error: {}", error);
            }
            DownMsg::ThemeSaved => {
                zoon::println!("Theme saved successfully");
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
        zoon::println!("Restoring scope selection: {}", scope_id);
        
        // Clear saved selections after successful extraction
        SAVED_SCOPE_SELECTIONS.lock_mut().clear();
        
        // Update signals without holding any locks
        SELECTED_SCOPE_ID.set_neq(Some(scope_id.clone()));
        
        let mut new_selection = HashSet::new();
        new_selection.insert(scope_id);
        TREE_SELECTED_ITEMS.set_neq(new_selection);
    } else {
        zoon::println!("No valid scope found to restore yet (waiting for more files to load)");
    }
}

fn restore_scope_selections() {
    let saved_selections = SAVED_SCOPE_SELECTIONS.lock_ref();
    if saved_selections.is_empty() {
        return;
    }
    
    zoon::println!("Restoring scope selections for {} files", saved_selections.len());
    
    // Since the config system only saves ONE global scope selection,
    // we should only have one entry, but iterate to be safe
    for (file_id, scope_id) in saved_selections.iter() {
        // Check if this file is currently loaded
        let file_exists = LOADED_FILES.lock_ref().iter().any(|f| f.id == *file_id);
        if !file_exists {
            zoon::println!("Skipping scope restoration for file {}: not currently loaded", file_id);
            continue;
        }
        
        // Verify the scope actually exists in the loaded files
        let scope_exists = LOADED_FILES.lock_ref().iter().any(|file| {
            file.id == *file_id && file_contains_scope(&file.scopes, scope_id)
        });
        
        if scope_exists {
            zoon::println!("Restoring scope selection: {} in file {}", scope_id, file_id);
            
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
        } else {
            zoon::println!("Skipping scope restoration for {}: scope not found in loaded files", scope_id);
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
    zoon::println!("Applying configuration...");
    
    // Store the loaded config to preserve inactive mode settings when saving
    LOADED_CONFIG.set_neq(Some(config.clone()));
    
    // Apply UI settings - initialize NovyUI theme with custom persistence
    zoon::println!("Theme: {}", config.ui.theme);
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
        zoon::println!("Auto-loading {} files from config", config.files.opened_files.len());
        for file_path in config.files.opened_files {
            zoon::println!("Loading file from config: {}", file_path);
            
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
    
    zoon::println!("Auto-saving config with {} opened files", config.files.opened_files.len());
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
            zoon::println!("Expanded scopes changed, auto-saving config");
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
            zoon::println!("Cleared all loaded files");
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
    all_variables
}

fn get_variables_from_selected_scope(selected_scope_id: &str) -> Vec<Signal> {
    let _timer = PerformanceTimer::new("get_variables_from_selected_scope");
    for file in LOADED_FILES.lock_ref().iter() {
        if let Some(variables) = find_variables_in_scope(&file.scopes, selected_scope_id) {
            zoon::println!("⏱️ [PERF] Found {} variables in scope {}", variables.len(), selected_scope_id);
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
                            .s(Width::fill())
                    )
                    .item(
                        input()
                            .placeholder("variable_name")
                            .left_icon(IconName::Search)
                            .size(InputSize::Small)
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(4))
                    .s(Padding::all(12))
                    .s(Height::fill())
                    .s(Width::fill())
                    .s(Scrollbars::both())
                    // Performance optimizations for scrollable container
                    .update_raw_el(|raw_el| {
                        raw_el.style("scrollbar-gutter", "stable")
                    })
                    .item(
                        El::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .child_signal(
                                SELECTED_SCOPE_ID.signal_ref(|selected_scope_id| {
                                    if let Some(scope_id) = selected_scope_id {
                                        let variables = get_variables_from_selected_scope(scope_id);
                                        if variables.is_empty() {
                                            Column::new()
                                                .s(Gap::new().y(4))
                                                .item(
                                                    El::new()
                                                        .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                                                        .child("No variables in selected scope")
                                                )
                                                .into_element()
                                        } else {
                                            let render_timer = PerformanceTimer::new(&format!("render_variables_list ({})", variables.len()));
                                            
                                            let variable_items: Vec<_> = variables.iter().map(|signal| {
                                                Row::new()
                                                    .s(Gap::new().x(8))
                                                    .s(Width::fill())
                                                    .s(Height::exact(24)) // Fixed height for consistency
                                                    // CSS Performance Optimizations
                                                    .update_raw_el(|raw_el| {
                                                        raw_el
                                                            .style("content-visibility", "auto")
                                                            .style("contain-intrinsic-size", "0 24px")
                                                            .style("contain", "layout style")
                                                    })
                                                    .item(
                                                        El::new()
                                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(14))
                                                            .s(Font::new().no_wrap())
                                                            .child(signal.name.clone())
                                                    )
                                                    .item(
                                                        El::new()
                                                            .s(Font::new().color(hsluv!(210, 80, 70)).size(12))
                                                            .s(Font::new().no_wrap())
                                                            .child(format!("{} {}-bit", signal.signal_type, signal.width))
                                                    )
                                                    .into_element()
                                            }).collect();
                                            
                                            render_timer.log_elapsed();
                                            zoon::println!("⏱️ [PERF] Creating Column with {} variable items", variable_items.len());
                                            
                                            Column::new()
                                                .s(Gap::new().y(4))
                                                .s(Width::fill())
                                                // CSS Performance Optimizations for container
                                                .update_raw_el(|raw_el| {
                                                    raw_el
                                                        .style("contain", "layout style paint")
                                                        .style("transform", "translateZ(0)") // Hardware acceleration
                                                })
                                                .items(variable_items)
                                                .into_element()
                                        }
                                    } else {
                                        Column::new()
                                            .s(Gap::new().y(4))
                                            .s(Width::fill())
                                            .item(
                                                El::new()
                                                    .s(Font::new().color(hsluv!(220, 10, 70)).size(13))
                                                    .child("Select a scope to view variables")
                                            )
                                            .into_element()
                                    }
                                })
                            )
                    )
            )
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

