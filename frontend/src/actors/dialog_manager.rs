#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::dataflow::Actor;
use zoon::*;
use std::collections::HashSet;
use indexmap::IndexSet;

/// Dialog Manager Domain - Basic state holder for dialog management
/// 
/// Simplified Actor that holds dialog state for compilation

#[derive(Debug, Clone, PartialEq)]
pub struct DialogManagerState {
    pub file_dialog_open: bool,
    pub paths_input: String,
    pub expanded_dirs: IndexSet<String>,
    pub viewport_y: i32,
    pub scroll_position: i32,
    pub last_expanded: HashSet<String>,
}

impl Default for DialogManagerState {
    fn default() -> Self {
        Self {
            file_dialog_open: false,
            paths_input: String::new(),
            expanded_dirs: IndexSet::new(),
            viewport_y: 0,
            scroll_position: 0,
            last_expanded: HashSet::new(),
        }
    }
}

// Global domain instance  
static DIALOG_MANAGER_DOMAIN: Lazy<Actor<DialogManagerState>> = Lazy::new(|| {
    Actor::new(DialogManagerState::default(), |_state| async move {
        // Simple Actor that just holds state - no event processing needed for now
        loop {
            Timer::sleep(1000).await;
        }
    })
});

// ===== SIGNAL ACCESS FUNCTIONS =====

/// Get file dialog open state signal
pub fn file_dialog_open_signal() -> impl Signal<Item = bool> {
    DIALOG_MANAGER_DOMAIN.signal().map(|state| state.file_dialog_open).dedupe()
}

/// Get paths input signal
pub fn paths_input_signal() -> impl Signal<Item = String> {
    DIALOG_MANAGER_DOMAIN.signal().map(|state| state.paths_input.clone()).dedupe_cloned()
}

/// Get expanded directories signal
pub fn expanded_directories_signal() -> impl Signal<Item = IndexSet<String>> {
    DIALOG_MANAGER_DOMAIN.signal().map(|state| state.expanded_dirs.clone()).dedupe_cloned()
}

/// Get viewport Y position signal
pub fn viewport_y_signal() -> impl Signal<Item = i32> {
    DIALOG_MANAGER_DOMAIN.signal().map(|state| state.viewport_y).dedupe()
}

/// Get scroll position signal
pub fn scroll_position_signal() -> impl Signal<Item = i32> {
    DIALOG_MANAGER_DOMAIN.signal().map(|state| state.scroll_position).dedupe()
}

/// Get last expanded directories signal
pub fn last_expanded_signal() -> impl Signal<Item = HashSet<String>> {
    DIALOG_MANAGER_DOMAIN.signal().map(|state| state.last_expanded.clone()).dedupe_cloned()
}

/// Get complete dialog manager state signal
pub fn dialog_manager_signal() -> impl Signal<Item = DialogManagerState> {
    DIALOG_MANAGER_DOMAIN.signal().dedupe_cloned()
}

// ===== INITIALIZATION =====

/// Initialize the dialog manager domain
pub fn initialize() {
    // Domain is automatically initialized when first accessed via Lazy
    let _ = &*DIALOG_MANAGER_DOMAIN;
}