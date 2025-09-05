#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::dataflow::Actor;
use zoon::*;
use indexmap::IndexSet;
use shared::{SelectedVariable, DockMode};

/// Config Sync Domain - Basic state holder for config synchronization
/// 
/// Simplified Actor that holds config state for compilation

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigSyncState {
    // File state for config
    pub opened_files: Vec<String>,
    pub expanded_scopes: Vec<String>,
    pub expanded_directories: Vec<String>,
    pub selected_variables: Vec<SelectedVariable>,
    pub dock_mode: DockMode,
    
    // TreeView-specific state
    pub treeview_expanded: IndexSet<String>,
    
    // Config loading state
    pub config_loaded: bool,
    pub initialization_complete: bool,
    pub save_pending: bool,
    
    // Completion triggers
    pub completion_trigger: u32,
}

impl Default for ConfigSyncState {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            expanded_scopes: Vec::new(),
            expanded_directories: Vec::new(),
            selected_variables: Vec::new(),
            dock_mode: DockMode::Bottom, // Default to bottom dock
            
            treeview_expanded: IndexSet::new(),
            
            config_loaded: false,
            initialization_complete: false,
            save_pending: false,
            
            completion_trigger: 0,
        }
    }
}

// Global domain instance
static CONFIG_SYNC_DOMAIN: Lazy<Actor<ConfigSyncState>> = Lazy::new(|| {
    Actor::new(ConfigSyncState::default(), |_state| async move {
        // ✅ FIXED: Use pending future instead of Timer::sleep() coordination delays
        // Actor holds state and stays alive without artificial delays
        use futures::future;
        future::pending::<()>().await;  // Stays alive indefinitely without polling
    })
});

// ===== SIGNAL ACCESS FUNCTIONS =====

/// Get opened files for config signal
pub fn opened_files_signal() -> impl Signal<Item = Vec<String>> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.opened_files.clone()).dedupe_cloned()
}

/// Get expanded scopes for config signal
pub fn expanded_scopes_signal() -> impl Signal<Item = Vec<String>> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.expanded_scopes.clone()).dedupe_cloned()
}

/// Get expanded directories for config signal
pub fn expanded_directories_signal() -> impl Signal<Item = Vec<String>> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.expanded_directories.clone()).dedupe_cloned()
}

/// Get selected variables for config signal
pub fn selected_variables_signal() -> impl Signal<Item = Vec<SelectedVariable>> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.selected_variables.clone()).dedupe_cloned()
}


/// Get TreeView expanded state signal
pub fn treeview_expanded_signal() -> impl Signal<Item = IndexSet<String>> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.treeview_expanded.clone()).dedupe_cloned()
}

/// Get config loaded state signal
pub fn config_loaded_signal() -> impl Signal<Item = bool> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.config_loaded).dedupe()
}

/// Get initialization complete state signal
pub fn initialization_complete_signal() -> impl Signal<Item = bool> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.initialization_complete).dedupe()
}

/// Get save pending state signal
pub fn save_pending_signal() -> impl Signal<Item = bool> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.save_pending).dedupe()
}

/// Get completion trigger signal
pub fn completion_trigger_signal() -> impl Signal<Item = u32> {
    CONFIG_SYNC_DOMAIN.signal().map(|state| state.completion_trigger).dedupe()
}

/// Get complete config sync state signal
pub fn config_sync_signal() -> impl Signal<Item = ConfigSyncState> {
    CONFIG_SYNC_DOMAIN.signal().dedupe_cloned()
}

// ===== HELPER FUNCTIONS =====

/// Set config loaded state (convenience function - for now just a placeholder)
pub fn set_config_loaded(_loaded: bool) {
    // Placeholder implementation
}

/// Set initialization complete (convenience function - for now just a placeholder)
pub fn set_initialization_complete(_complete: bool) {
    // Placeholder implementation
}

/// Set save pending state (convenience function - for now just a placeholder)
pub fn set_save_pending(_pending: bool) {
    // Placeholder implementation
}

/// Trigger completion event (convenience function - for now just a placeholder)
pub fn trigger_completion() {
    // Placeholder implementation
}

// ===== INITIALIZATION =====

/// Initialize the config sync domain
pub fn initialize() {
    // Domain is automatically initialized when first accessed via Lazy
    let _ = &*CONFIG_SYNC_DOMAIN;
}