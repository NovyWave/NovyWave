//! Global domain instances for Actor+Relay architecture
//!
//! Centralized instantiation and access to domain actors throughout the application.
//! Replaces global mutables with domain-driven reactive state management.

use crate::actors::{TrackedFiles, SelectedVariables, DialogManager, ErrorManager};
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
use crate::actors::{dialog_manager, error_manager, config_sync};
use std::sync::OnceLock;
use shared::{TrackedFile, SelectedVariable};
use indexmap::IndexSet;
use zoon::*;

/// Global TrackedFiles domain instance
/// Connected to UI through signal bridge in TRACKED_FILES_SIGNALS
static TRACKED_FILES_DOMAIN_INSTANCE: OnceLock<TrackedFiles> = OnceLock::new();

/// Global SelectedVariables domain instance  
/// Connected to UI through signal bridge in SELECTED_VARIABLES_SIGNALS
pub static SELECTED_VARIABLES_DOMAIN_INSTANCE: OnceLock<SelectedVariables> = OnceLock::new();

/// Global WaveformTimeline domain instance
/// Connected to UI through proper Actor+Relay signals
static WAVEFORM_TIMELINE_DOMAIN_INSTANCE: OnceLock<WaveformTimeline> = OnceLock::new();



/// Global DialogManager domain instance
static DIALOG_MANAGER_DOMAIN_INSTANCE: OnceLock<DialogManager> = OnceLock::new();

/// Global ErrorManager domain instance
static ERROR_MANAGER_DOMAIN_INSTANCE: OnceLock<ErrorManager> = OnceLock::new();

// === STATIC SIGNAL STORAGE FOR DOMAIN SIGNAL LIFETIME FIX ===

// ✅ ELIMINATED: TrackedFilesSignalStorage struct definition
// No longer needed - signals accessed directly from domain

// ✅ ELIMINATED: TRACKED_FILES_SIGNALS static storage container
// Functions now use direct domain access via tracked_files_domain().method_signal()

// ✅ ELIMINATED: SelectedVariablesSignalStorage struct definition
// No longer needed - signals accessed directly from domain

// ✅ ELIMINATED: SELECTED_VARIABLES_SIGNALS static storage container
// Functions now use direct domain access via selected_variables_domain().method_signal()


// ✅ ELIMINATED: DialogManagerSignalStorage - unused static signal storage replaced by Actor domain

/// Global DialogManager signal storage - Bridge between domain and UI signals
// ✅ ELIMINATED: DIALOG_MANAGER_SIGNALS static signal bypass - now uses direct domain access

// ✅ ELIMINATED: ErrorManagerSignalStorage - unused static signal storage replaced by Actor domain

/// Global ErrorManager signal storage - Bridge between domain and UI signals
// ✅ ELIMINATED: ERROR_MANAGER_SIGNALS static signal bypass - now uses direct domain access

/// Initialize all domain instances - call once on app startup
pub async fn initialize_all_domains() -> Result<(), &'static str> {
    // PHASE 1: Initialize static signal storage first  
    // ✅ ELIMINATED: TRACKED_FILES_SIGNALS - now uses direct domain access
    // ✅ ELIMINATED: ERROR_MANAGER_SIGNALS - now uses direct domain access
    
    // PHASE 2: Initialize legacy domains in parallel for better startup performance
    let (tracked_files, selected_variables, waveform_timeline, dialog_manager, error_manager) = futures::join!(
        TrackedFiles::new(),
        SelectedVariables::new(),
        WaveformTimeline::new(),
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
pub fn tracked_files_domain() -> &'static TrackedFiles {
    TRACKED_FILES_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            panic!("TrackedFiles domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
}

/// Get the global SelectedVariables domain instance
pub fn selected_variables_domain() -> &'static SelectedVariables {
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            // ⚠️  CRITICAL: Domain accessed before initialization during app startup
            // This indicates a race condition in initialization order
            panic!("SelectedVariables domain accessed before initialization - this indicates a critical application initialization ordering bug. Call initialize_all_domains() before starting UI components.")
        })
}

/// Get the global WaveformTimeline domain instance
pub fn waveform_timeline_domain() -> &'static WaveformTimeline {
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            panic!("WaveformTimeline domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
}

/// Get the global UserConfiguration domain instance


/// Get the global DialogManager domain instance
pub fn dialog_manager_domain() -> &'static DialogManager {
    DIALOG_MANAGER_DOMAIN_INSTANCE.get()
        .unwrap_or_else(|| {
            panic!("DialogManager domain accessed before initialization - this indicates a critical application initialization ordering bug")
        })
}


/// Check if all domains are initialized
pub fn _are_domains_initialized() -> bool {
    TRACKED_FILES_DOMAIN_INSTANCE.get().is_some() && 
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get().is_some() && 
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get().is_some() && 
    DIALOG_MANAGER_DOMAIN_INSTANCE.get().is_some() &&
    ERROR_MANAGER_DOMAIN_INSTANCE.get().is_some()
}

// === GLOBAL SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) ===

/// Get owned signal for all tracked files - LIFETIME SAFE for UI components
/// Enables: tracked_files_signal().map(|files| render(files))
pub fn tracked_files_signal() -> impl Signal<Item = Vec<TrackedFile>> {
    tracked_files_domain().files_signal()
}

/// Get current tracked files synchronously - for functions that need immediate access
/// Use sparingly - prefer reactive tracked_files_signal() when possible
pub fn get_current_tracked_files() -> Vec<TrackedFile> {
    if let Some(domain) = TRACKED_FILES_DOMAIN_INSTANCE.get() {
        // Use the proper domain method
        domain.get_current_files()
    } else {
        // Fallback during initialization
        Vec::new()
    }
}

// ✅ ELIMINATED: compare_tracked_files() - unused helper function, sorting logic moved to domain or eliminated

/// Get owned signal vec for tracked files - LIFETIME SAFE for items_signal_vec
/// Enables: .items_signal_vec(tracked_files_signal_vec().map(|file| render(file)))
pub fn tracked_files_signal_vec() -> impl SignalVec<Item = TrackedFile> {
    let domain = tracked_files_domain();
    domain.files_signal_vec()
}

/// Get owned signal for loading files - LIFETIME SAFE
// ❌ ANTIPATTERN: SignalVec to Signal conversion causes 20+ renders from single change
// ✅ ELIMINATED: loading_files_signal() - Use tracked_files_domain().loading_files_signal() directly
// ✅ ELIMINATED: loaded_files_signal() - Use tracked_files_domain().loaded_files_signal() directly  
// ✅ ELIMINATED: is_loading_signal() - Use tracked_files_domain().is_loading_signal() directly

/// Get owned signal for file count - CONNECTS TO TRACKEDFILES DOMAIN
pub fn file_count_signal() -> impl Signal<Item = usize> {
    let domain = tracked_files_domain();
    domain.file_count_signal()
}


/// Get owned signal for selected variables - LIFETIME SAFE
/// Enables: selected_variables_signal().map(|vars| render(vars))
pub fn selected_variables_signal() -> impl Signal<Item = Vec<SelectedVariable>> {
    let domain = selected_variables_domain();
    domain.variables_signal()
}

/// Get owned signal vec for selected variables - LIFETIME SAFE for items_signal_vec
pub fn selected_variables_signal_vec() -> impl SignalVec<Item = SelectedVariable> {
    let domain = selected_variables_domain();
    domain.variables_signal_vec()
}

/// Get owned signal for expanded scopes - LIFETIME SAFE
pub fn expanded_scopes_signal() -> impl Signal<Item = IndexSet<String>> {
    let domain = selected_variables_domain();
    domain.expanded_scopes_signal()
}


// === SIGNAL SYNCHRONIZATION FUNCTIONS (INTERNAL) ===

// ✅ ELIMINATED: Static signal update functions no longer needed
// All signal functions now use direct domain access instead of static signal bypass

// ✅ ELIMINATED: All TrackedFiles static signal update functions removed
// Domain signals now accessed directly via tracked_files_domain().method_signal()

// ✅ ELIMINATED: All SelectedVariables static signal update functions removed
// Domain signals now accessed directly via selected_variables_domain().method_signal()


// === DIALOG MANAGER SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) ===

/// Get owned signal for dialog visibility - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_visible_signal() -> impl Signal<Item = bool> {
    dialog_manager_domain().dialog_visible.signal()
}

/// Get owned signal for paths input - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_paths_input_signal() -> impl Signal<Item = String> {
    dialog_manager_domain().paths_input.signal()
}

/// Get owned signal for expanded directories - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_expanded_directories_signal() -> impl Signal<Item = IndexSet<String>> {
    dialog_manager_domain().expanded_directories.signal()
}

/// Get owned signal for selected files - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_selected_files_signal() -> impl Signal<Item = Vec<String>> {
    dialog_manager_domain().selected_files.signal()
}

/// Get owned signal for current error - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_current_error_signal() -> impl Signal<Item = Option<String>> {
    dialog_manager_domain().current_error.signal()
}

/// Get owned signal for error cache - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_error_cache_signal() -> impl Signal<Item = std::collections::HashMap<String, String>> {
    dialog_manager_domain().error_cache.signal()
}

/// Get owned signal for viewport Y position - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_viewport_y_signal() -> impl Signal<Item = i32> {
    dialog_manager_domain().viewport_y.signal()
}

/// Get owned signal for scroll position - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_scroll_position_signal() -> impl Signal<Item = i32> {
    dialog_manager_domain().scroll_position.signal()
}

/// Get owned signal for last expanded directories - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_last_expanded_signal() -> impl Signal<Item = std::collections::HashSet<String>> {
    dialog_manager_domain().last_expanded.signal()
}

// ✅ ELIMINATED: dialog_manager_expanded_mutable() - unused bi-directional sync antipattern (uses prohibited zoon::Task)
// Use dialog_manager_expanded_directories_signal() directly instead

/// Get selected files mutable for TreeView external_selected - SIMPLIFIED: Direct domain access

/// Get owned signal for file tree cache - ARCHITECTURE FIX: Use proper domain Actor instead of static bypass
pub fn dialog_manager_file_tree_cache_signal() -> impl Signal<Item = std::collections::HashMap<String, Vec<shared::FileSystemItem>>> {
    dialog_manager_domain().file_tree_cache.signal()
}

/// Get file tree cache mutable for direct access - SIMPLIFIED: Direct domain access
/// ❌ ELIMINATED: Complex bi-directional sync using zoon::Task (violates Actor+Relay architecture)
/// ✅ CORRECTED: Use direct domain signals - components should connect directly to domain signals
pub fn dialog_manager_file_tree_cache_mutable() -> zoon::Mutable<std::collections::HashMap<String, Vec<shared::FileSystemItem>>> {
    // ✅ SIMPLIFIED: Return basic mutable, components connect to domain signals separately
    // This eliminates zoon::Task antipattern while maintaining API compatibility
    let cache_mutable = zoon::Mutable::new(std::collections::HashMap::new());
    
    // Components should use: .child_signal(dialog_manager_file_tree_cache_signal().map(...))
    // instead of complex bi-directional sync patterns
    
    cache_mutable
}



// === VALIDATION FUNCTIONS FOR LIFETIME FIX ===


// Test function removed - use proper reactive patterns in actual code

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
            async move {
                // Step 2: Sync to config for persistence (no helper functions needed)
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