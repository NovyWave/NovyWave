//! Global domain instances for Actor+Relay architecture
//!
//! Centralized instantiation and access to domain actors throughout the application.
//! Replaces global mutables with domain-driven reactive state management.

use crate::actors::{TrackedFiles, SelectedVariables, WaveformTimeline, UserConfiguration};
use crate::actors::{panel_layout, dialog_manager, error_manager, config_sync};
use std::sync::OnceLock;

/// Global TrackedFiles domain instance
static TRACKED_FILES_DOMAIN_INSTANCE: OnceLock<TrackedFiles> = OnceLock::new();

/// Global SelectedVariables domain instance  
static SELECTED_VARIABLES_DOMAIN_INSTANCE: OnceLock<SelectedVariables> = OnceLock::new();

/// Global WaveformTimeline domain instance
static WAVEFORM_TIMELINE_DOMAIN_INSTANCE: OnceLock<WaveformTimeline> = OnceLock::new();

/// Global UserConfiguration domain instance
static USER_CONFIGURATION_DOMAIN_INSTANCE: OnceLock<UserConfiguration> = OnceLock::new();

/// Initialize all domain instances - call once on app startup
pub async fn initialize_all_domains() {
    // Initialize legacy domains in parallel for better startup performance
    let (tracked_files, selected_variables, waveform_timeline, user_config) = futures::join!(
        TrackedFiles::new(),
        SelectedVariables::new(),
        WaveformTimeline::new(),
        UserConfiguration::new()
    );
    
    // Store legacy instances for global access
    TRACKED_FILES_DOMAIN_INSTANCE.set(tracked_files)
        .expect("TrackedFiles domain already initialized - initialize_all_domains() should only be called once");
    SELECTED_VARIABLES_DOMAIN_INSTANCE.set(selected_variables)
        .expect("SelectedVariables domain already initialized - initialize_all_domains() should only be called once");
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.set(waveform_timeline)
        .expect("WaveformTimeline domain already initialized - initialize_all_domains() should only be called once");
    USER_CONFIGURATION_DOMAIN_INSTANCE.set(user_config)
        .expect("UserConfiguration domain already initialized - initialize_all_domains() should only be called once");
    
    // Initialize Phase 2 domains (Lazy-initialized automatically on first access)
    panel_layout::initialize();
    dialog_manager::initialize();
    error_manager::initialize();
    config_sync::initialize();
    
    // All Actor+Relay domains initialized successfully
}

/// Get the global TrackedFiles domain instance
pub fn tracked_files_domain() -> TrackedFiles {
    TRACKED_FILES_DOMAIN_INSTANCE.get()
        .expect("TrackedFiles domain not initialized - call initialize_all_domains() first")
        .clone()
}

/// Get the global SelectedVariables domain instance
pub fn selected_variables_domain() -> SelectedVariables {
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .expect("SelectedVariables domain not initialized - call initialize_all_domains() first")
        .clone()
}

/// Get the global WaveformTimeline domain instance
pub fn waveform_timeline_domain() -> WaveformTimeline {
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get()
        .expect("WaveformTimeline domain not initialized - call initialize_all_domains() first")
        .clone()
}

/// Get the global UserConfiguration domain instance
pub fn _user_configuration_domain() -> UserConfiguration {
    USER_CONFIGURATION_DOMAIN_INSTANCE.get()
        .expect("UserConfiguration domain not initialized - call initialize_all_domains() first")
        .clone()
}

/// Check if all domains are initialized
pub fn _are_domains_initialized() -> bool {
    TRACKED_FILES_DOMAIN_INSTANCE.get().is_some() && 
    SELECTED_VARIABLES_DOMAIN_INSTANCE.get().is_some() && 
    WAVEFORM_TIMELINE_DOMAIN_INSTANCE.get().is_some() && 
    USER_CONFIGURATION_DOMAIN_INSTANCE.get().is_some()
}