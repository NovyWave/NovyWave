//! Global domain instances for Actor+Relay architecture
//!
//! Centralized instantiation and access to domain actors throughout the application.
//! Replaces global mutables with domain-driven reactive state management.

use crate::actors::{TrackedFiles, SelectedVariables, WaveformTimeline, UserConfiguration, PanelLayout, DialogManager, ErrorManager};
use crate::actors::{panel_layout, dialog_manager, error_manager, config_sync};
use std::sync::OnceLock;
use shared::{TrackedFile, LoadingFile, WaveformFile, SelectedVariable};
use indexmap::{IndexSet, IndexMap};
use zoon::*;

/// Global TrackedFiles domain instance
static TRACKED_FILES_DOMAIN_INSTANCE: OnceLock<TrackedFiles> = OnceLock::new();

/// Global SelectedVariables domain instance  
pub static SELECTED_VARIABLES_DOMAIN_INSTANCE: OnceLock<SelectedVariables> = OnceLock::new();

/// Global WaveformTimeline domain instance
static WAVEFORM_TIMELINE_DOMAIN_INSTANCE: OnceLock<WaveformTimeline> = OnceLock::new();

/// Global UserConfiguration domain instance
static USER_CONFIGURATION_DOMAIN_INSTANCE: OnceLock<UserConfiguration> = OnceLock::new();

/// Global PanelLayout domain instance
static PANEL_LAYOUT_DOMAIN_INSTANCE: OnceLock<PanelLayout> = OnceLock::new();

/// Global DialogManager domain instance
static DIALOG_MANAGER_DOMAIN_INSTANCE: OnceLock<DialogManager> = OnceLock::new();

/// Global ErrorManager domain instance
static ERROR_MANAGER_DOMAIN_INSTANCE: OnceLock<ErrorManager> = OnceLock::new();

// === STATIC SIGNAL STORAGE FOR DOMAIN SIGNAL LIFETIME FIX ===

/// Static signal storage for TrackedFiles domain
/// Enables UI components to use `tracked_files_signal().map(...)` patterns
/// by providing owned signals that survive domain instance lifecycle
struct TrackedFilesSignalStorage {
    files_mutable: MutableVec<TrackedFile>,
    loading_files_mutable: MutableVec<LoadingFile>, 
    loaded_files_mutable: MutableVec<WaveformFile>,
    is_loading_mutable: Mutable<bool>,
    file_ids_mutable: Mutable<IndexSet<String>>,
    file_paths_mutable: Mutable<IndexMap<String, String>>,
}

impl TrackedFilesSignalStorage {
    fn new() -> Self {
        Self {
            files_mutable: MutableVec::new(),
            loading_files_mutable: MutableVec::new(),
            loaded_files_mutable: MutableVec::new(),
            is_loading_mutable: Mutable::new(false),
            file_ids_mutable: Mutable::new(IndexSet::new()),
            file_paths_mutable: Mutable::new(IndexMap::new()),
        }
    }
}

/// Global TrackedFiles signal storage
static TRACKED_FILES_SIGNALS: OnceLock<TrackedFilesSignalStorage> = OnceLock::new();

/// Static signal storage for SelectedVariables domain  
struct SelectedVariablesSignalStorage {
    variables_mutable: MutableVec<SelectedVariable>,
    expanded_scopes_mutable: Mutable<IndexSet<String>>,
    search_filter_mutable: Mutable<String>,
}

impl SelectedVariablesSignalStorage {
    fn new() -> Self {
        Self {
            variables_mutable: MutableVec::new(),
            expanded_scopes_mutable: Mutable::new(IndexSet::new()),
            search_filter_mutable: Mutable::new(String::new()),
        }
    }
}

/// Global SelectedVariables signal storage
static SELECTED_VARIABLES_SIGNALS: OnceLock<SelectedVariablesSignalStorage> = OnceLock::new();

/// Static signal storage for PanelLayout domain
pub struct PanelLayoutSignalStorage {
    pub files_panel_width_mutable: Mutable<u32>,
    pub files_panel_height_mutable: Mutable<u32>,
    pub variables_name_column_width_mutable: Mutable<u32>,
    pub variables_value_column_width_mutable: Mutable<u32>,
    pub timeline_panel_height_mutable: Mutable<u32>,
    pub dock_mode_mutable: Mutable<shared::DockMode>,
    pub dock_transitioning_mutable: Mutable<bool>,
    pub files_vertical_dragging_mutable: Mutable<bool>,
    pub files_horizontal_dragging_mutable: Mutable<bool>,
    pub name_divider_dragging_mutable: Mutable<bool>,
    pub value_divider_dragging_mutable: Mutable<bool>,
}

impl PanelLayoutSignalStorage {
    fn new() -> Self {
        Self {
            files_panel_width_mutable: Mutable::new(470),
            files_panel_height_mutable: Mutable::new(300),
            variables_name_column_width_mutable: Mutable::new(180),
            variables_value_column_width_mutable: Mutable::new(100),
            timeline_panel_height_mutable: Mutable::new(200),
            dock_mode_mutable: Mutable::new(shared::DockMode::Right),
            dock_transitioning_mutable: Mutable::new(false),
            files_vertical_dragging_mutable: Mutable::new(false),
            files_horizontal_dragging_mutable: Mutable::new(false),
            name_divider_dragging_mutable: Mutable::new(false),
            value_divider_dragging_mutable: Mutable::new(false),
        }
    }
}

/// Global PanelLayout signal storage
pub static PANEL_LAYOUT_SIGNALS: OnceLock<PanelLayoutSignalStorage> = OnceLock::new();

/// Static signal storage for DialogManager domain
pub struct DialogManagerSignalStorage {
    pub dialog_visible_mutable: Mutable<bool>,
    pub paths_input_mutable: Mutable<String>,
    pub expanded_directories_mutable: Mutable<IndexSet<String>>,
    pub selected_files_mutable: MutableVec<String>,
    pub current_error_mutable: Mutable<Option<String>>,
    pub error_cache_mutable: Mutable<std::collections::HashMap<String, String>>,
    pub viewport_y_mutable: Mutable<i32>,
    pub scroll_position_mutable: Mutable<i32>,
    pub last_expanded_mutable: Mutable<std::collections::HashSet<String>>,
}

impl DialogManagerSignalStorage {
    fn new() -> Self {
        Self {
            dialog_visible_mutable: Mutable::new(false),
            paths_input_mutable: Mutable::new(String::new()),
            expanded_directories_mutable: Mutable::new(IndexSet::new()),
            selected_files_mutable: MutableVec::new(),
            current_error_mutable: Mutable::new(None),
            error_cache_mutable: Mutable::new(std::collections::HashMap::new()),
            viewport_y_mutable: Mutable::new(0),
            scroll_position_mutable: Mutable::new(0),
            last_expanded_mutable: Mutable::new(std::collections::HashSet::new()),
        }
    }
}

/// Global DialogManager signal storage
pub static DIALOG_MANAGER_SIGNALS: OnceLock<DialogManagerSignalStorage> = OnceLock::new();

/// Static signal storage for ErrorManager domain
pub struct ErrorManagerSignalStorage {
    pub alerts_mutable: MutableVec<crate::state::ErrorAlert>,
    pub notifications_mutable: MutableVec<crate::state::ErrorAlert>,
    pub picker_error_mutable: Mutable<Option<String>>,
    pub error_cache_mutable: Mutable<std::collections::HashMap<String, String>>,
    pub next_alert_id_mutable: Mutable<u32>,
}

impl ErrorManagerSignalStorage {
    fn new() -> Self {
        Self {
            alerts_mutable: MutableVec::new(),
            notifications_mutable: MutableVec::new(),
            picker_error_mutable: Mutable::new(None),
            error_cache_mutable: Mutable::new(std::collections::HashMap::new()),
            next_alert_id_mutable: Mutable::new(1),
        }
    }
}

/// Global ErrorManager signal storage
pub static ERROR_MANAGER_SIGNALS: OnceLock<ErrorManagerSignalStorage> = OnceLock::new();

/// Initialize all domain instances - call once on app startup
pub async fn initialize_all_domains() -> Result<(), &'static str> {
    zoon::println!("🔄 Starting Actor+Relay domain initialization...");
    
    // PHASE 1: Initialize static signal storage first
    zoon::println!("🔄 Initializing static signal storage...");
    TRACKED_FILES_SIGNALS.set(TrackedFilesSignalStorage::new())
        .map_err(|_| "FATAL: TrackedFiles signal storage already initialized")?;
    SELECTED_VARIABLES_SIGNALS.set(SelectedVariablesSignalStorage::new())
        .map_err(|_| "FATAL: SelectedVariables signal storage already initialized")?;
    PANEL_LAYOUT_SIGNALS.set(PanelLayoutSignalStorage::new())
        .map_err(|_| "FATAL: PanelLayout signal storage already initialized")?;
    DIALOG_MANAGER_SIGNALS.set(DialogManagerSignalStorage::new())
        .map_err(|_| "FATAL: DialogManager signal storage already initialized")?;
    ERROR_MANAGER_SIGNALS.set(ErrorManagerSignalStorage::new())
        .map_err(|_| "FATAL: ErrorManager signal storage already initialized")?;
    
    // PHASE 2: Initialize legacy domains in parallel for better startup performance
    zoon::println!("🔄 Creating domain instances...");
    let (tracked_files, selected_variables, waveform_timeline, user_config, panel_layout, dialog_manager, error_manager) = futures::join!(
        TrackedFiles::new(),
        SelectedVariables::new(),
        WaveformTimeline::new(),
        UserConfiguration::new(),
        PanelLayout::new(),
        DialogManager::new(),
        ErrorManager::new()
    );
    
    zoon::println!("🔄 Domain constructors completed, storing instances...");
    
    // Store legacy instances for global access
    TRACKED_FILES_DOMAIN_INSTANCE.set(tracked_files)
        .map_err(|_| "FATAL: TrackedFiles domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    SELECTED_VARIABLES_DOMAIN_INSTANCE.set(selected_variables)
        .map_err(|_| "FATAL: SelectedVariables domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.set(waveform_timeline)
        .map_err(|_| "FATAL: WaveformTimeline domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    USER_CONFIGURATION_DOMAIN_INSTANCE.set(user_config)
        .map_err(|_| "FATAL: UserConfiguration domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    PANEL_LAYOUT_DOMAIN_INSTANCE.set(panel_layout)
        .map_err(|_| "FATAL: PanelLayout domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    DIALOG_MANAGER_DOMAIN_INSTANCE.set(dialog_manager)
        .map_err(|_| "FATAL: DialogManager domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    ERROR_MANAGER_DOMAIN_INSTANCE.set(error_manager)
        .map_err(|_| "FATAL: ErrorManager domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    
    // Initialize Phase 2 domains (Lazy-initialized automatically on first access)
    panel_layout::initialize();
    dialog_manager::initialize();
    error_manager::initialize();
    config_sync::initialize();
    
    // All Actor+Relay domains initialized successfully
    zoon::println!("✅ All Actor+Relay domains initialized successfully");
    Ok(())
}

/// Get the global TrackedFiles domain instance
pub fn tracked_files_domain() -> TrackedFiles {
    TRACKED_FILES_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            zoon::println!("🚨 FATAL: TrackedFiles domain not initialized - initialize_all_domains() must be called during app startup before accessing domains");
            panic!("TrackedFiles domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global SelectedVariables domain instance
pub fn selected_variables_domain() -> SelectedVariables {
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            zoon::println!("🚨 FATAL: SelectedVariables domain not initialized - initialize_all_domains() must be called during app startup before accessing domains");
            panic!("SelectedVariables domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global WaveformTimeline domain instance
pub fn waveform_timeline_domain() -> WaveformTimeline {
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get()
        .map(|instance| instance.clone())
        .unwrap_or_else(|| {
            zoon::println!("⚠️ WaveformTimeline domain not initialized, creating dummy instance for initialization");
            WaveformTimeline::new_dummy_for_initialization()
        })
}

/// Get the global UserConfiguration domain instance
pub fn _user_configuration_domain() -> UserConfiguration {
    USER_CONFIGURATION_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            zoon::println!("🚨 FATAL: UserConfiguration domain not initialized - initialize_all_domains() must be called during app startup before accessing domains");
            panic!("UserConfiguration domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global PanelLayout domain instance
pub fn panel_layout_domain() -> PanelLayout {
    PANEL_LAYOUT_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            zoon::println!("🚨 FATAL: PanelLayout domain not initialized - initialize_all_domains() must be called during app startup before accessing domains");
            panic!("PanelLayout domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global DialogManager domain instance
pub fn dialog_manager_domain() -> DialogManager {
    DIALOG_MANAGER_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            zoon::println!("🚨 FATAL: DialogManager domain not initialized - initialize_all_domains() must be called during app startup before accessing domains");
            panic!("DialogManager domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global ErrorManager domain instance
pub fn error_manager_domain() -> ErrorManager {
    ERROR_MANAGER_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            zoon::println!("🚨 FATAL: ErrorManager domain not initialized - initialize_all_domains() must be called during app startup before accessing domains");
            panic!("ErrorManager domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Check if all domains are initialized
pub fn _are_domains_initialized() -> bool {
    TRACKED_FILES_DOMAIN_INSTANCE.get().is_some() && 
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get().is_some() && 
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get().is_some() && 
    USER_CONFIGURATION_DOMAIN_INSTANCE.get().is_some() &&
    PANEL_LAYOUT_DOMAIN_INSTANCE.get().is_some() &&
    DIALOG_MANAGER_DOMAIN_INSTANCE.get().is_some() &&
    ERROR_MANAGER_DOMAIN_INSTANCE.get().is_some()
}

// === GLOBAL SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) ===

/// Get owned signal for all tracked files - LIFETIME SAFE for UI components
/// Enables: tracked_files_signal().map(|files| render(files))
pub fn tracked_files_signal() -> impl Signal<Item = Vec<TrackedFile>> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| signals.files_mutable.signal_vec_cloned().to_signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ TrackedFiles signals not initialized, returning empty signal");
            MutableVec::<TrackedFile>::new().signal_vec_cloned().to_signal_cloned()
        })
}

/// Get owned signal vec for tracked files - LIFETIME SAFE for items_signal_vec
/// Enables: .items_signal_vec(tracked_files_signal_vec().map(|file| render(file)))
pub fn tracked_files_signal_vec() -> impl SignalVec<Item = TrackedFile> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| {
            zoon::println!("🔍 DEBUG: TrackedFiles signals found, returning signal_vec");
            signals.files_mutable.signal_vec_cloned()
        })
        .unwrap_or_else(|| {
            zoon::println!("⚠️ DEBUG: TrackedFiles signals not initialized, returning empty signal vec");
            zoon::MutableVec::new().signal_vec_cloned()
        })
}

/// Get owned signal for loading files - LIFETIME SAFE
pub fn loading_files_signal() -> impl Signal<Item = Vec<LoadingFile>> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| signals.loading_files_mutable.signal_vec_cloned().to_signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ TrackedFiles signals not initialized, returning empty loading files signal");
            MutableVec::<LoadingFile>::new().signal_vec_cloned().to_signal_cloned()
        })
}

/// Get owned signal for loaded files - LIFETIME SAFE
pub fn loaded_files_signal() -> impl Signal<Item = Vec<WaveformFile>> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| signals.loaded_files_mutable.signal_vec_cloned().to_signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ TrackedFiles signals not initialized, returning empty loaded files signal");
            MutableVec::<WaveformFile>::new().signal_vec_cloned().to_signal_cloned()
        })
}

/// Get owned signal for loading state - LIFETIME SAFE
pub fn is_loading_signal() -> impl Signal<Item = bool> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| signals.is_loading_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ TrackedFiles signals not initialized, returning false loading signal");
            Mutable::new(false).signal()
        })
}

/// Get owned signal for file count - SIMPLE ACTOR+RELAY APPROACH
pub fn file_count_signal() -> impl Signal<Item = usize> {
    // Connect to real config data instead of static hardcoded value
    crate::config::app_config().session_state_actor.signal().map(|session| {
        let count = session.opened_files.len();
        zoon::println!("🔍 DEBUG: file_count_signal returning: {}", count);
        count
    })
}

/// Get owned signal for loaded files count - SIMPLE ACTOR+RELAY APPROACH
pub fn loaded_files_count_signal() -> impl Signal<Item = usize> {
    // Use a simple Mutable<usize> that gets updated by the TrackedFiles domain
    use std::sync::OnceLock;
    static LOADED_COUNT_SIGNAL: OnceLock<Mutable<usize>> = OnceLock::new();
    
    let signal = LOADED_COUNT_SIGNAL.get_or_init(|| Mutable::new(0));
    signal.signal()
}

/// Get owned signal for selected variables - LIFETIME SAFE
/// Enables: selected_variables_signal().map(|vars| render(vars))
pub fn selected_variables_signal() -> impl Signal<Item = Vec<SelectedVariable>> {
    SELECTED_VARIABLES_SIGNALS.get()
        .map(|signals| signals.variables_mutable.signal_vec_cloned().to_signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables signals not initialized, returning empty signal");
            MutableVec::<SelectedVariable>::new().signal_vec_cloned().to_signal_cloned()
        })
}

/// Get owned signal vec for selected variables - LIFETIME SAFE for items_signal_vec
pub fn selected_variables_signal_vec() -> impl SignalVec<Item = SelectedVariable> {
    SELECTED_VARIABLES_SIGNALS.get()
        .map(|signals| signals.variables_mutable.signal_vec_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables signals not initialized, returning empty signal vec");
            zoon::MutableVec::new().signal_vec_cloned()
        })
}

/// Get owned signal for expanded scopes - LIFETIME SAFE
pub fn expanded_scopes_signal() -> impl Signal<Item = IndexSet<String>> {
    SELECTED_VARIABLES_SIGNALS.get()
        .map(|signals| signals.expanded_scopes_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables signals not initialized, returning empty expanded scopes signal");
            Mutable::new(IndexSet::<String>::new()).signal_cloned()
        })
}

/// Get owned signal for search filter - LIFETIME SAFE
pub fn search_filter_signal() -> impl Signal<Item = String> {
    SELECTED_VARIABLES_SIGNALS.get()
        .map(|signals| signals.search_filter_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables signals not initialized, returning empty search filter signal");
            Mutable::new(String::new()).signal_cloned()
        })
}

// === SIGNAL SYNCHRONIZATION FUNCTIONS (INTERNAL) ===

/// Update the static signal storage when TrackedFiles domain changes
/// This bridges domain events to static signals for UI reactive access
pub fn _update_tracked_files_signals(files: Vec<TrackedFile>) {
    // Update count signals
    use std::sync::OnceLock;
    static FILE_COUNT_SIGNAL: OnceLock<Mutable<usize>> = OnceLock::new();
    static LOADED_COUNT_SIGNAL: OnceLock<Mutable<usize>> = OnceLock::new();
    
    let file_count = FILE_COUNT_SIGNAL.get_or_init(|| Mutable::new(0));
    let loaded_count = LOADED_COUNT_SIGNAL.get_or_init(|| Mutable::new(0));
    
    file_count.set_neq(files.len());
    
    let loaded = files.iter()
        .filter(|file| matches!(file.state, shared::FileState::Loaded(_)))
        .count();
    loaded_count.set_neq(loaded);
    if let Some(signals) = TRACKED_FILES_SIGNALS.get() {
        signals.files_mutable.lock_mut().replace_cloned(files);
    }
}

/// Update tracked files in signal storage (for config sync and external updates)
pub fn update_tracked_files_signals(files: Vec<TrackedFile>) {
    if let Some(signals) = TRACKED_FILES_SIGNALS.get() {
        signals.files_mutable.lock_mut().replace_cloned(files);
    }
}

/// Update loading files signal
pub fn _update_loading_files_signals(loading_files: Vec<LoadingFile>) {
    if let Some(signals) = TRACKED_FILES_SIGNALS.get() {
        signals.loading_files_mutable.lock_mut().replace_cloned(loading_files);
    }
}

/// Update loaded files signal
pub fn _update_loaded_files_signals(loaded_files: Vec<WaveformFile>) {
    if let Some(signals) = TRACKED_FILES_SIGNALS.get() {
        signals.loaded_files_mutable.lock_mut().replace_cloned(loaded_files);
    }
}

/// Update loading state signal
pub fn _update_is_loading_signal(is_loading: bool) {
    if let Some(signals) = TRACKED_FILES_SIGNALS.get() {
        signals.is_loading_mutable.set_neq(is_loading);
    }
}

/// Update the static signal storage when SelectedVariables domain changes
pub fn _update_selected_variables_signals(variables: Vec<SelectedVariable>) {
    if let Some(signals) = SELECTED_VARIABLES_SIGNALS.get() {
        signals.variables_mutable.lock_mut().replace_cloned(variables);
    }
}

/// Update expanded scopes signal
pub fn _update_expanded_scopes_signals(expanded_scopes: IndexSet<String>) {
    if let Some(signals) = SELECTED_VARIABLES_SIGNALS.get() {
        signals.expanded_scopes_mutable.set_neq(expanded_scopes);
    }
}

/// Update search filter signal
pub fn _update_search_filter_signals(search_filter: String) {
    if let Some(signals) = SELECTED_VARIABLES_SIGNALS.get() {
        signals.search_filter_mutable.set_neq(search_filter);
    }
}

// === PANEL LAYOUT SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) ===

/// Get owned signal for files panel width - LIFETIME SAFE
pub fn panel_layout_files_width_signal() -> impl Signal<Item = u32> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.files_panel_width_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default width signal");
            Mutable::new(300u32).signal()
        })
}

/// Get owned signal for files panel height - LIFETIME SAFE
pub fn panel_layout_files_height_signal() -> impl Signal<Item = u32> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.files_panel_height_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default height signal");
            Mutable::new(200u32).signal()
        })
}

/// Get owned signal for variables name column width - LIFETIME SAFE
pub fn panel_layout_name_column_width_signal() -> impl Signal<Item = u32> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.variables_name_column_width_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default name column width signal");
            Mutable::new(150u32).signal()
        })
}

/// Get owned signal for variables value column width - LIFETIME SAFE
pub fn panel_layout_value_column_width_signal() -> impl Signal<Item = u32> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.variables_value_column_width_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default value column width signal");
            Mutable::new(100u32).signal()
        })
}

/// Get owned signal for timeline panel height - LIFETIME SAFE
pub fn panel_layout_timeline_height_signal() -> impl Signal<Item = u32> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.timeline_panel_height_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default timeline height signal");
            Mutable::new(300u32).signal()
        })
}

/// Get owned signal for dock mode - LIFETIME SAFE
pub fn panel_layout_dock_mode_signal() -> impl Signal<Item = shared::DockMode> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.dock_mode_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default dock mode signal");
            Mutable::new(shared::DockMode::Right).signal_cloned()
        })
}

/// Get owned signal for dock transitioning state - LIFETIME SAFE
pub fn panel_layout_dock_transitioning_signal() -> impl Signal<Item = bool> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.dock_transitioning_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning false dock transitioning signal");
            Mutable::new(false).signal()
        })
}

/// Get owned signal for files vertical dragging state - LIFETIME SAFE
pub fn panel_layout_files_vertical_dragging_signal() -> impl Signal<Item = bool> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.files_vertical_dragging_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning false files vertical dragging signal");
            Mutable::new(false).signal()
        })
}

/// Get owned signal for files horizontal dragging state - LIFETIME SAFE
pub fn panel_layout_files_horizontal_dragging_signal() -> impl Signal<Item = bool> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.files_horizontal_dragging_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning false files horizontal dragging signal");
            Mutable::new(false).signal()
        })
}

/// Get owned signal for name divider dragging state - LIFETIME SAFE
pub fn panel_layout_name_divider_dragging_signal() -> impl Signal<Item = bool> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.name_divider_dragging_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning false name divider dragging signal");
            Mutable::new(false).signal()
        })
}

/// Get owned signal for value divider dragging state - LIFETIME SAFE
pub fn panel_layout_value_divider_dragging_signal() -> impl Signal<Item = bool> {
    PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.value_divider_dragging_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning false value divider dragging signal");
            Mutable::new(false).signal()
        })
}

// === DIALOG MANAGER SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) ===

/// Get owned signal for dialog visibility - LIFETIME SAFE
pub fn dialog_manager_visible_signal() -> impl Signal<Item = bool> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.dialog_visible_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning false dialog visible signal");
            Mutable::new(false).signal()
        })
}

/// Get owned signal for paths input - LIFETIME SAFE  
pub fn dialog_manager_paths_input_signal() -> impl Signal<Item = String> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.paths_input_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning empty paths input signal");
            Mutable::new(String::new()).signal_cloned()
        })
}

/// Get owned signal for expanded directories - LIFETIME SAFE
pub fn dialog_manager_expanded_directories_signal() -> impl Signal<Item = IndexSet<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.expanded_directories_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning empty expanded directories signal");
            Mutable::new(IndexSet::<String>::new()).signal_cloned()
        })
}

/// Get owned signal for selected files - LIFETIME SAFE
pub fn dialog_manager_selected_files_signal() -> impl Signal<Item = Vec<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.selected_files_mutable.signal_vec_cloned().to_signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning empty selected files signal");
            MutableVec::<String>::new().signal_vec_cloned().to_signal_cloned()
        })
}

/// Get owned signal for current error - LIFETIME SAFE
pub fn dialog_manager_current_error_signal() -> impl Signal<Item = Option<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.current_error_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning None current error signal");
            Mutable::new(None::<String>).signal_cloned()
        })
}

/// Get owned signal for error cache - LIFETIME SAFE
pub fn dialog_manager_error_cache_signal() -> impl Signal<Item = std::collections::HashMap<String, String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.error_cache_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning empty error cache signal");
            Mutable::new(std::collections::HashMap::<String, String>::new()).signal_cloned()
        })
}

/// Get owned signal for viewport Y position - LIFETIME SAFE
pub fn dialog_manager_viewport_y_signal() -> impl Signal<Item = i32> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.viewport_y_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning 0 viewport y signal");
            Mutable::new(0i32).signal()
        })
}

/// Get owned signal for scroll position - LIFETIME SAFE
pub fn dialog_manager_scroll_position_signal() -> impl Signal<Item = i32> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.scroll_position_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning 0 scroll position signal");
            Mutable::new(0i32).signal()
        })
}

/// Get owned signal for last expanded directories - LIFETIME SAFE
pub fn dialog_manager_last_expanded_signal() -> impl Signal<Item = std::collections::HashSet<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.last_expanded_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning empty last expanded signal");
            Mutable::new(std::collections::HashSet::<String>::new()).signal_cloned()
        })
}

/// Get expanded directories mutable for TreeView external_expanded - LIFETIME SAFE
pub fn dialog_manager_expanded_mutable() -> Mutable<IndexSet<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.expanded_directories_mutable.clone())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning new empty expanded directories mutable");
            Mutable::new(IndexSet::<String>::new())
        })
}

/// Get selected files mutable for TreeView external_selected - LIFETIME SAFE
pub fn dialog_manager_selected_mutable() -> MutableVec<String> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.selected_files_mutable.clone())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ DialogManager signals not initialized, returning new empty selected files mutable");
            MutableVec::<String>::new()
        })
}

/// Get owned signal for error alerts - LIFETIME SAFE
pub fn error_manager_alerts_signal() -> impl Signal<Item = Vec<crate::state::ErrorAlert>> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.alerts_mutable.signal_vec_cloned().to_signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ ErrorManager signals not initialized, returning empty alerts signal");
            MutableVec::<crate::state::ErrorAlert>::new().signal_vec_cloned().to_signal_cloned()
        })
}

/// Get owned signal for toast notifications - LIFETIME SAFE
pub fn error_manager_notifications_signal() -> impl Signal<Item = Vec<crate::state::ErrorAlert>> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.notifications_mutable.signal_vec_cloned().to_signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ ErrorManager signals not initialized, returning empty notifications signal");
            MutableVec::<crate::state::ErrorAlert>::new().signal_vec_cloned().to_signal_cloned()
        })
}

/// Get owned signal for file picker error - LIFETIME SAFE
pub fn error_manager_picker_error_signal() -> impl Signal<Item = Option<String>> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.picker_error_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ ErrorManager signals not initialized, returning None picker error signal");
            Mutable::new(None::<String>).signal_cloned()
        })
}

/// Get owned signal for error cache - LIFETIME SAFE
pub fn error_manager_error_cache_signal() -> impl Signal<Item = std::collections::HashMap<String, String>> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.error_cache_mutable.signal_cloned())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ ErrorManager signals not initialized, returning empty error cache signal");
            Mutable::new(std::collections::HashMap::<String, String>::new()).signal_cloned()
        })
}

/// Get owned signal for next alert ID - LIFETIME SAFE  
pub fn error_manager_next_alert_id_signal() -> impl Signal<Item = u32> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.next_alert_id_mutable.signal())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ ErrorManager signals not initialized, creating default mutable with 1");
            zoon::Mutable::new(1).signal()
        })
}

// === PANEL LAYOUT SIGNAL SYNCHRONIZATION FUNCTIONS (INTERNAL) ===

/// Update panel layout signals when domain changes
pub fn _update_panel_layout_signals(
    files_width: u32,
    files_height: u32,
    name_column_width: u32,
    value_column_width: u32,
    timeline_height: u32,
    dock_mode: shared::DockMode,
    dock_transitioning: bool,
    files_vertical_dragging: bool,
    files_horizontal_dragging: bool,
    name_divider_dragging: bool,
    value_divider_dragging: bool,
) {
    if let Some(signals) = PANEL_LAYOUT_SIGNALS.get() {
        signals.files_panel_width_mutable.set_neq(files_width);
        signals.files_panel_height_mutable.set_neq(files_height);
        signals.variables_name_column_width_mutable.set_neq(name_column_width);
        signals.variables_value_column_width_mutable.set_neq(value_column_width);
        signals.timeline_panel_height_mutable.set_neq(timeline_height);
        signals.dock_mode_mutable.set_neq(dock_mode);
        signals.dock_transitioning_mutable.set_neq(dock_transitioning);
        signals.files_vertical_dragging_mutable.set_neq(files_vertical_dragging);
        signals.files_horizontal_dragging_mutable.set_neq(files_horizontal_dragging);
        signals.name_divider_dragging_mutable.set_neq(name_divider_dragging);
        signals.value_divider_dragging_mutable.set_neq(value_divider_dragging);
    }
}

// === VALIDATION FUNCTIONS FOR LIFETIME FIX ===

/// Test function to validate that domain signals have proper lifetimes
/// This would NOT compile with the old borrowed signal approach
#[allow(dead_code)]
pub fn test_domain_signal_lifetimes() -> impl Element {
    // ✅ BEFORE FIX: This would FAIL with lifetime errors  
    // ❌ tracked_files_domain().files_signal().map(|files| ...)  <- Domain instance dropped
    
    // ✅ AFTER FIX: This WORKS because signals are owned and stable
    Column::new()
        .item(
            Row::new()
                .item(Text::new("File Count: "))
                .item_signal(file_count_signal().map(|count| Text::new(&count.to_string())))
        )
        .item(
            Row::new()
                .item(Text::new("Loading: "))
                .item_signal(is_loading_signal().map(|loading| 
                    Text::new(if loading { "Yes" } else { "No" })
                ))
        )
        .items_signal_vec(
            tracked_files_signal_vec().map(|file| {
                Text::new(&format!("File: {}", file.filename))
            })
        )
        .items_signal_vec(
            selected_variables_signal_vec().map(|var| {
                Text::new(&format!("Variable: {}", var.unique_id))
            })
        )
}

/// MIGRATION: Test function for domain access - needs reactive conversion
#[allow(dead_code)]
pub fn test_synchronous_domain_access() {
    // MIGRATION: This should be converted to reactive patterns
    let _files = Vec::<String>::new(); // Temporary during migration
    let _file_count = 0;
    
    // Emit domain events (should update static signals automatically through bridge)
    tracked_files_domain().batch_load_files(vec!["test.vcd".to_string()]);
    
    // Domain relay example (if available in the domain API)
    // selected_variables_domain().some_relay.send(());
}