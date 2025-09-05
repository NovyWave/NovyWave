//! Global domain instances for Actor+Relay architecture
//!
//! Centralized instantiation and access to domain actors throughout the application.
//! Replaces global mutables with domain-driven reactive state management.

use crate::actors::{TrackedFiles, SelectedVariables, UserConfiguration, DialogManager, ErrorManager};
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
use crate::actors::{dialog_manager, error_manager, config_sync};
use std::sync::OnceLock;
use shared::{TrackedFile, LoadingFile, WaveformFile, SelectedVariable};
use indexmap::{IndexSet, IndexMap};
use zoon::*;

/// Global TrackedFiles domain instance
// ❌ ANTIPATTERN: Static domain instances create non-functional signals that never update
#[deprecated(note = "Use proper Actor+Relay initialization with working signal connections instead of static OnceLock")]
static TRACKED_FILES_DOMAIN_INSTANCE: OnceLock<TrackedFiles> = OnceLock::new();

/// Global SelectedVariables domain instance  
// ❌ ANTIPATTERN: Static domain instances create non-functional signals that never update
#[deprecated(note = "Use proper Actor+Relay initialization with working signal connections instead of static OnceLock")]
pub static SELECTED_VARIABLES_DOMAIN_INSTANCE: OnceLock<SelectedVariables> = OnceLock::new();

/// Global WaveformTimeline domain instance
// ❌ ANTIPATTERN: Static domain instances create non-functional signals that never update
#[deprecated(note = "Use proper Actor+Relay initialization with working signal connections instead of static OnceLock")]
static WAVEFORM_TIMELINE_DOMAIN_INSTANCE: OnceLock<WaveformTimeline> = OnceLock::new();

/// Global UserConfiguration domain instance
static USER_CONFIGURATION_DOMAIN_INSTANCE: OnceLock<UserConfiguration> = OnceLock::new();


/// Global DialogManager domain instance
// ❌ ANTIPATTERN: Static domain instances create non-functional signals that never update
#[deprecated(note = "Use proper Actor+Relay initialization with working signal connections instead of static OnceLock")]
static DIALOG_MANAGER_DOMAIN_INSTANCE: OnceLock<DialogManager> = OnceLock::new();

/// Global ErrorManager domain instance
// ❌ ANTIPATTERN: Static domain instances create non-functional signals that never update
#[deprecated(note = "Use proper Actor+Relay initialization with working signal connections instead of static OnceLock")]
static ERROR_MANAGER_DOMAIN_INSTANCE: OnceLock<ErrorManager> = OnceLock::new();

// === STATIC SIGNAL STORAGE FOR DOMAIN SIGNAL LIFETIME FIX ===

/// Static signal storage for TrackedFiles domain
/// Enables UI components to use `tracked_files_signal().map(...)` patterns
/// by providing owned signals that survive domain instance lifecycle
pub struct TrackedFilesSignalStorage {
    pub files_mutable: MutableVec<TrackedFile>,
    pub loading_files_mutable: MutableVec<LoadingFile>, 
    pub loaded_files_mutable: MutableVec<WaveformFile>,
    pub is_loading_mutable: Mutable<bool>,
    #[allow(dead_code)] // Migration signal storage - preserve for actor transition
    pub file_ids_mutable: Mutable<IndexSet<String>>,
    #[allow(dead_code)] // Migration signal storage - preserve for actor transition
    pub file_paths_mutable: Mutable<IndexMap<String, String>>,
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
// ❌ ANTIPATTERN: Static signal storage creates non-reactive signal chains
#[deprecated(note = "Use proper Actor signal methods instead of static signal storage")]
pub static TRACKED_FILES_SIGNALS: OnceLock<TrackedFilesSignalStorage> = OnceLock::new();

/// Static signal storage for SelectedVariables domain  
pub struct SelectedVariablesSignalStorage {
    #[allow(dead_code)] // Migration signal storage - preserve for actor transition
    pub variables_mutable: MutableVec<SelectedVariable>,
    #[allow(dead_code)] // Migration signal storage - preserve for actor transition
    pub expanded_scopes_mutable: Mutable<IndexSet<String>>,
    #[allow(dead_code)] // Migration signal storage - preserve for actor transition
    pub search_filter_mutable: Mutable<String>,
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
// ❌ ANTIPATTERN: Static signal storage creates non-reactive signal chains
#[deprecated(note = "Use proper Actor signal methods instead of static signal storage")]
pub static SELECTED_VARIABLES_SIGNALS: OnceLock<SelectedVariablesSignalStorage> = OnceLock::new();


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
// ❌ ANTIPATTERN: Static signal storage creates non-reactive signal chains
#[deprecated(note = "Use proper Actor signal methods instead of static signal storage")]
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
// ❌ ANTIPATTERN: Static signal storage creates non-reactive signal chains
#[deprecated(note = "Use proper Actor signal methods instead of static signal storage")]
pub static ERROR_MANAGER_SIGNALS: OnceLock<ErrorManagerSignalStorage> = OnceLock::new();

/// Initialize all domain instances - call once on app startup
pub async fn initialize_all_domains() -> Result<(), &'static str> {
    // PHASE 1: Initialize static signal storage first
    TRACKED_FILES_SIGNALS.set(TrackedFilesSignalStorage::new())
        .map_err(|_| "FATAL: TrackedFiles signal storage already initialized")?;
    SELECTED_VARIABLES_SIGNALS.set(SelectedVariablesSignalStorage::new())
        .map_err(|_| "FATAL: SelectedVariables signal storage already initialized")?;
    DIALOG_MANAGER_SIGNALS.set(DialogManagerSignalStorage::new())
        .map_err(|_| "FATAL: DialogManager signal storage already initialized")?;
    ERROR_MANAGER_SIGNALS.set(ErrorManagerSignalStorage::new())
        .map_err(|_| "FATAL: ErrorManager signal storage already initialized")?;
    
    // PHASE 2: Initialize legacy domains in parallel for better startup performance
    let (tracked_files, selected_variables, waveform_timeline, user_config, dialog_manager, error_manager) = futures::join!(
        TrackedFiles::new(),
        SelectedVariables::new(),
        WaveformTimeline::new(),
        UserConfiguration::new(),
        DialogManager::new(),
        ErrorManager::new()
    );
    
    // Store legacy instances for global access
    TRACKED_FILES_DOMAIN_INSTANCE.set(tracked_files)
        .map_err(|_| "FATAL: TrackedFiles domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    SELECTED_VARIABLES_DOMAIN_INSTANCE.set(selected_variables)
        .map_err(|_| "FATAL: SelectedVariables domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.set(waveform_timeline)
        .map_err(|_| "FATAL: WaveformTimeline domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    USER_CONFIGURATION_DOMAIN_INSTANCE.set(user_config)
        .map_err(|_| "FATAL: UserConfiguration domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    DIALOG_MANAGER_DOMAIN_INSTANCE.set(dialog_manager)
        .map_err(|_| "FATAL: DialogManager domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    ERROR_MANAGER_DOMAIN_INSTANCE.set(error_manager)
        .map_err(|_| "FATAL: ErrorManager domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    
    // Initialize Phase 2 domains (Lazy-initialized automatically on first access)
    dialog_manager::initialize();
    error_manager::initialize();
    config_sync::initialize();
    
    // PHASE 3: Connect TrackedFiles domain to config persistence
    setup_tracked_files_config_bridge().await;
    
    Ok(())
}

/// Get the global TrackedFiles domain instance
pub fn tracked_files_domain() -> TrackedFiles {
    TRACKED_FILES_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            panic!("TrackedFiles domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global SelectedVariables domain instance
pub fn selected_variables_domain() -> SelectedVariables {
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            panic!("SelectedVariables domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global WaveformTimeline domain instance
pub fn waveform_timeline_domain() -> WaveformTimeline {
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get()
        .map(|instance| instance.clone())
        .unwrap_or_else(|| {
            panic!("WaveformTimeline domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
}

/// Get the global UserConfiguration domain instance
pub fn _user_configuration_domain() -> UserConfiguration {
    USER_CONFIGURATION_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            panic!("UserConfiguration domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}


/// Get the global DialogManager domain instance
pub fn dialog_manager_domain() -> DialogManager {
    DIALOG_MANAGER_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            panic!("DialogManager domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
        .clone()
}

/// Get the global ErrorManager domain instance
pub fn error_manager_domain() -> ErrorManager {
    ERROR_MANAGER_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
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
    DIALOG_MANAGER_DOMAIN_INSTANCE.get().is_some() &&
    ERROR_MANAGER_DOMAIN_INSTANCE.get().is_some()
}

// === GLOBAL SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) ===

/// Get owned signal for all tracked files - LIFETIME SAFE for UI components
/// Enables: tracked_files_signal().map(|files| render(files))
pub fn tracked_files_signal() -> impl Signal<Item = Vec<TrackedFile>> {
    // ✅ FIXED: Use dedicated Vec signal from domain instead of conversion antipattern
    if let Some(domain) = TRACKED_FILES_DOMAIN_INSTANCE.get() {
        domain.files_signal().boxed_local()
    } else {
        Mutable::new(vec![]).signal_cloned().boxed_local()
    }
}

/// Compare two TrackedFiles for sorting: filename first, then visible distinguishing prefix
fn compare_tracked_files(a: &shared::TrackedFile, b: &shared::TrackedFile) -> std::cmp::Ordering {
    // Use filename directly from TrackedFile (already extracted)
    let filename_a = &a.filename;
    let filename_b = &b.filename;
    
    // Extract visible distinguishing prefix (immediate parent directory name only)
    // This matches how smart labels are generated
    let prefix_a = std::path::Path::new(&a.path)
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "".into());
    
    let prefix_b = std::path::Path::new(&b.path)
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "".into());
    
    // Sort by filename first (case insensitive), then by visible distinguishing prefix
    let filename_cmp = filename_a.to_lowercase().cmp(&filename_b.to_lowercase());
    if filename_cmp == std::cmp::Ordering::Equal {
        prefix_a.to_lowercase().cmp(&prefix_b.to_lowercase())
    } else {
        filename_cmp
    }
}

/// Get owned signal vec for tracked files - LIFETIME SAFE for items_signal_vec
/// Enables: .items_signal_vec(tracked_files_signal_vec().map(|file| render(file)))
pub fn tracked_files_signal_vec() -> impl SignalVec<Item = TrackedFile> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| {
            signals.files_mutable.signal_vec_cloned().sort_by_cloned(compare_tracked_files)
        })
        .unwrap_or_else(|| {
            zoon::MutableVec::new().signal_vec_cloned().sort_by_cloned(compare_tracked_files)
        })
}

/// Get owned signal for loading files - LIFETIME SAFE
// ❌ ANTIPATTERN: SignalVec to Signal conversion causes 20+ renders from single change
#[deprecated(note = "Use tracked_files_domain().loading_files_signal() with items_signal_vec instead of signal conversion")]
#[allow(dead_code)]
pub fn loading_files_signal() -> impl Signal<Item = Vec<LoadingFile>> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| signals.loading_files_mutable.signal_vec_cloned().to_signal_cloned().dedupe_cloned())
        .unwrap_or_else(|| {
            MutableVec::<LoadingFile>::new().signal_vec_cloned().to_signal_cloned().dedupe_cloned()
        })
}

/// Get owned signal for loaded files - LIFETIME SAFE
// ❌ ANTIPATTERN: SignalVec to Signal conversion causes 20+ renders from single change
#[deprecated(note = "Use tracked_files_domain().loaded_files_signal() with items_signal_vec instead of signal conversion")]
#[allow(dead_code)]
pub fn loaded_files_signal() -> impl Signal<Item = Vec<WaveformFile>> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| signals.loaded_files_mutable.signal_vec_cloned().to_signal_cloned().dedupe_cloned())
        .unwrap_or_else(|| {
            MutableVec::<WaveformFile>::new().signal_vec_cloned().to_signal_cloned().dedupe_cloned()
        })
}

/// Get owned signal for loading state - LIFETIME SAFE
pub fn is_loading_signal() -> impl Signal<Item = bool> {
    TRACKED_FILES_SIGNALS.get()
        .map(|signals| signals.is_loading_mutable.signal())
        .unwrap_or_else(|| {
            Mutable::new(false).signal()
        })
}

/// Get owned signal for file count - CONNECTS TO TRACKEDFILES DOMAIN
pub fn file_count_signal() -> impl Signal<Item = usize> {
    // ✅ FIXED: Use proper Actor signal from TrackedFiles domain
    if let Some(domain) = TRACKED_FILES_DOMAIN_INSTANCE.get() {
        domain.file_count_signal().boxed_local()
    } else {
        Mutable::new(0).signal().boxed_local()
    }
}

/// Get owned signal for loaded files count - SIMPLE ACTOR+RELAY APPROACH
#[allow(dead_code)]
pub fn loaded_files_count_signal() -> impl Signal<Item = usize> {
    // ✅ FIXED: Use proper Actor signal from TrackedFiles domain
    if let Some(domain) = TRACKED_FILES_DOMAIN_INSTANCE.get() {
        domain.loaded_count_signal().boxed_local()
    } else {
        Mutable::new(0).signal().boxed_local()
    }
}

/// Get owned signal for selected variables - LIFETIME SAFE
/// Enables: selected_variables_signal().map(|vars| render(vars))
pub fn selected_variables_signal() -> impl Signal<Item = Vec<SelectedVariable>> {
    // ✅ FIXED: Read from Actor's state directly (single source of truth)
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .expect("SelectedVariables domain not initialized - initialize_all_domains() must be called first")
        .variables_signal()
}

/// Get owned signal vec for selected variables - LIFETIME SAFE for items_signal_vec
pub fn selected_variables_signal_vec() -> impl SignalVec<Item = SelectedVariable> {
    // ✅ FIXED: Read from Actor's state directly (single source of truth)
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .expect("SelectedVariables domain not initialized - initialize_all_domains() must be called first")
        .variables_signal_vec()
}

/// Get owned signal for expanded scopes - LIFETIME SAFE
#[allow(dead_code)] // Actor+Relay API function - preserve for completeness
pub fn expanded_scopes_signal() -> impl Signal<Item = IndexSet<String>> {
    SELECTED_VARIABLES_SIGNALS.get()
        .map(|signals| signals.expanded_scopes_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(IndexSet::<String>::new()).signal_cloned()
        })
}

/// Get owned signal for search filter - LIFETIME SAFE
#[allow(dead_code)] // Actor+Relay API function - preserve for completeness
pub fn search_filter_signal() -> impl Signal<Item = String> {
    SELECTED_VARIABLES_SIGNALS.get()
        .map(|signals| signals.search_filter_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(String::new()).signal_cloned()
        })
}

// === SIGNAL SYNCHRONIZATION FUNCTIONS (INTERNAL) ===

/// Update the static signal storage when TrackedFiles domain changes
/// This bridges domain events to static signals for UI reactive access
pub fn _update_tracked_files_signals(files: Vec<TrackedFile>) {
    // ✅ FIXED: Removed OnceLock antipatterns - count signals now use proper domain signals
    // The file_count_signal() and loaded_files_count_signal() functions now read directly from 
    // TrackedFiles domain instead of these static signals
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


// === DIALOG MANAGER SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) ===

/// Get owned signal for dialog visibility - LIFETIME SAFE
pub fn dialog_manager_visible_signal() -> impl Signal<Item = bool> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.dialog_visible_mutable.signal())
        .unwrap_or_else(|| {
            Mutable::new(false).signal()
        })
}

/// Get owned signal for paths input - LIFETIME SAFE  
pub fn dialog_manager_paths_input_signal() -> impl Signal<Item = String> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.paths_input_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(String::new()).signal_cloned()
        })
}

/// Get owned signal for expanded directories - LIFETIME SAFE
pub fn dialog_manager_expanded_directories_signal() -> impl Signal<Item = IndexSet<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.expanded_directories_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(IndexSet::<String>::new()).signal_cloned()
        })
}

/// Get owned signal for selected files - LIFETIME SAFE
// ❌ ANTIPATTERN: SignalVec to Signal conversion causes 20+ renders from single change
#[deprecated(note = "Use dialog_manager_domain().selected_files_signal() with items_signal_vec instead of signal conversion")]
pub fn dialog_manager_selected_files_signal() -> impl Signal<Item = Vec<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.selected_files_mutable.signal_vec_cloned().to_signal_cloned().dedupe_cloned())
        .unwrap_or_else(|| {
            MutableVec::<String>::new().signal_vec_cloned().to_signal_cloned().dedupe_cloned()
        })
}

/// Get owned signal for current error - LIFETIME SAFE
pub fn dialog_manager_current_error_signal() -> impl Signal<Item = Option<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.current_error_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(None::<String>).signal_cloned()
        })
}

/// Get owned signal for error cache - LIFETIME SAFE
pub fn dialog_manager_error_cache_signal() -> impl Signal<Item = std::collections::HashMap<String, String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.error_cache_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(std::collections::HashMap::<String, String>::new()).signal_cloned()
        })
}

/// Get owned signal for viewport Y position - LIFETIME SAFE
pub fn dialog_manager_viewport_y_signal() -> impl Signal<Item = i32> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.viewport_y_mutable.signal())
        .unwrap_or_else(|| {
            Mutable::new(0i32).signal()
        })
}

/// Get owned signal for scroll position - LIFETIME SAFE
pub fn dialog_manager_scroll_position_signal() -> impl Signal<Item = i32> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.scroll_position_mutable.signal())
        .unwrap_or_else(|| {
            Mutable::new(0i32).signal()
        })
}

/// Get owned signal for last expanded directories - LIFETIME SAFE
pub fn dialog_manager_last_expanded_signal() -> impl Signal<Item = std::collections::HashSet<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.last_expanded_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(std::collections::HashSet::<String>::new()).signal_cloned()
        })
}

/// Get expanded directories mutable for TreeView external_expanded - LIFETIME SAFE
#[allow(dead_code)]
pub fn dialog_manager_expanded_mutable() -> Mutable<IndexSet<String>> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.expanded_directories_mutable.clone())
        .unwrap_or_else(|| {
            Mutable::new(IndexSet::<String>::new())
        })
}

/// Get selected files mutable for TreeView external_selected - LIFETIME SAFE
pub fn dialog_manager_selected_mutable() -> MutableVec<String> {
    DIALOG_MANAGER_SIGNALS.get()
        .map(|signals| signals.selected_files_mutable.clone())
        .unwrap_or_else(|| {
            MutableVec::<String>::new()
        })
}

/// Get owned signal for error alerts - LIFETIME SAFE
pub fn error_manager_alerts_signal() -> impl Signal<Item = Vec<crate::state::ErrorAlert>> {
    // ✅ FIXED: Use dedicated Vec signal from domain instead of conversion antipattern
    if let Some(domain) = ERROR_MANAGER_DOMAIN_INSTANCE.get() {
        domain.alerts_vec_signal().boxed_local()
    } else {
        Mutable::new(vec![]).signal_cloned().boxed_local()
    }
}

/// Get owned signal for toast notifications - LIFETIME SAFE
pub fn error_manager_notifications_signal() -> impl Signal<Item = Vec<crate::state::ErrorAlert>> {
    // ✅ FIXED: Use dedicated Vec signal from domain instead of conversion antipattern
    if let Some(domain) = ERROR_MANAGER_DOMAIN_INSTANCE.get() {
        domain.notifications_vec_signal().boxed_local()
    } else {
        Mutable::new(vec![]).signal_cloned().boxed_local()
    }
}

/// Get SignalVec for toast notifications - EFFICIENT for items_signal_vec
pub fn error_manager_notifications_signal_vec() -> impl SignalVec<Item = crate::state::ErrorAlert> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.notifications_mutable.signal_vec_cloned())
        .unwrap_or_else(|| {
            MutableVec::<crate::state::ErrorAlert>::new().signal_vec_cloned()
        })
}

/// Get owned signal for file picker error - LIFETIME SAFE
pub fn error_manager_picker_error_signal() -> impl Signal<Item = Option<String>> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.picker_error_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(None::<String>).signal_cloned()
        })
}

/// Get owned signal for error cache - LIFETIME SAFE
pub fn error_manager_error_cache_signal() -> impl Signal<Item = std::collections::HashMap<String, String>> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.error_cache_mutable.signal_cloned())
        .unwrap_or_else(|| {
            Mutable::new(std::collections::HashMap::<String, String>::new()).signal_cloned()
        })
}

/// Get owned signal for next alert ID - LIFETIME SAFE  
#[allow(dead_code)]
pub fn error_manager_next_alert_id_signal() -> impl Signal<Item = u32> {
    ERROR_MANAGER_SIGNALS.get()
        .map(|signals| signals.next_alert_id_mutable.signal())
        .unwrap_or_else(|| {
            zoon::Mutable::new(1).signal()
        })
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

/// Set up bridge between TrackedFiles domain and config persistence
/// Similar to how expanded directories work with direct config connection
async fn setup_tracked_files_config_bridge() {
    
    // Step 1: Load files from config into TrackedFiles domain
    let app_config = crate::config::app_config();
    let initial_files = app_config.session_state_actor.signal().to_stream().next().await;
    
    if let Some(session_state) = initial_files {
        if !session_state.opened_files.is_empty() {
            let tracked_files_domain = tracked_files_domain();
            tracked_files_domain.config_files_loaded_relay.send(session_state.opened_files);
        }
    }
    
    // Step 2: Watch TrackedFiles changes and sync back to config (like expanded directories)
    let tracked_files_domain = tracked_files_domain();
    let session_relay = app_config.session_state_changed_relay.clone();
    let session_actor = app_config.session_state_actor.clone();
    
    Task::start(async move {
        tracked_files_domain.files_signal().for_each(move |files| {
            let session_relay = session_relay.clone();
            let session_actor = session_actor.clone();
            let files_copy = files.clone();
            async move {
                // Step 2a: Update static signal storage for UI
                update_tracked_files_signals(files_copy.clone());
                
                // Step 2b: Sync to config for persistence
                // Extract file paths from TrackedFiles
                let file_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
                
                // Get current session state and update opened_files
                if let Some(mut session_state) = session_actor.signal().to_stream().next().await {
                    if session_state.opened_files != file_paths {
                        session_state.opened_files = file_paths.clone();
                        
                        // Trigger config save through session state change
                        session_relay.send(session_state);
                    }
                }
            }
        }).await;
    });
    
}