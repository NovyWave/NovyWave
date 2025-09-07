//! LEGACY: Global domain instances - DEPRECATED
//!
//! ⚠️  **ARCHITECTURAL DEBT WARNING** ⚠️ 
//! 
//! This file contains legacy global domain patterns that contradict the completed
//! NovyWaveApp self-contained architecture. The new NovyWaveApp creates its own
//! domain instances and doesn't use these global domains.
//!
//! **STATUS**: Used by legacy views.rs and other files that haven't been migrated
//! **TODO**: Remove this file after migrating views.rs to use NovyWaveApp domains
//! **NEW ARCHITECTURE**: See app.rs for proper domain ownership patterns

use crate::{TrackedFiles, SelectedVariables};
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
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



// DialogManager eliminated - replaced with simple file_dialog.rs Atom-based UI state

// ErrorManager removed as hollow stub

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

// ✅ ELIMINATED: ErrorManagerSignalStorage - hollow stub removed

/// Global ErrorManager signal storage - Bridge between domain and UI signals
// ✅ ELIMINATED: ERROR_MANAGER_SIGNALS static signal bypass - now uses direct domain access

/// LEGACY: Initialize all domain instances - DEPRECATED
/// 
/// ⚠️ **NOT USED** by NovyWaveApp architecture - domains are created directly in app.rs
/// Only kept for compatibility with legacy views.rs that still use global domains.
#[deprecated(note = "Use NovyWaveApp::new() instead - creates domains without global state")]
pub async fn initialize_all_domains() -> Result<(), &'static str> {
    // PHASE 1: Initialize static signal storage first  
    // ✅ ELIMINATED: TRACKED_FILES_SIGNALS - now uses direct domain access
    // ✅ ELIMINATED: ERROR_MANAGER_SIGNALS - hollow stub removed
    
    // PHASE 2: Initialize legacy domains in parallel for better startup performance
    let (tracked_files, selected_variables, waveform_timeline) = futures::join!(
        TrackedFiles::new(),
        SelectedVariables::new(),
        WaveformTimeline::new()
    );
    
    // Store legacy instances for global access
    TRACKED_FILES_DOMAIN_INSTANCE.set(tracked_files)
        .map_err(|_| "FATAL: TrackedFiles domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    SELECTED_VARIABLES_DOMAIN_INSTANCE.set(selected_variables)
        .map_err(|_| "FATAL: SelectedVariables domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.set(waveform_timeline)
        .map_err(|_| "FATAL: WaveformTimeline domain already initialized. This indicates initialize_all_domains() was called multiple times, which suggests a serious application initialization bug. The application must restart to recover.")?;
    // DialogManager eliminated - replaced with simple file_dialog.rs Atom-based UI state
    // ErrorManager removed as hollow stub
    
    // Initialize Phase 2 domains (Lazy-initialized automatically on first access)
    // dialog_manager::initialize() eliminated - no longer needed with simple Atom approach
    
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


// dialog_manager_domain() eliminated - replaced with simple file_dialog.rs functions


/// Check if all domains are initialized
pub fn _are_domains_initialized() -> bool {
    TRACKED_FILES_DOMAIN_INSTANCE.get().is_some() && 
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get().is_some() && 
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get().is_some()
    // DialogManager eliminated - no longer part of domain initialization check
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


// === FILE DIALOG LEGACY COMPATIBILITY FUNCTIONS ===
// DialogManager enterprise antipattern eliminated - replaced with simple file_dialog.rs Atom-based UI state

// ✅ CLEANED UP: All legacy dialog_manager compatibility functions removed as they were unused



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