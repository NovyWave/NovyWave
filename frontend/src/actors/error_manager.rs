#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::dataflow::Actor;
use crate::state::ErrorAlert; // Reuse existing ErrorAlert struct
use zoon::*;

/// Error Manager Domain - Basic state holder for error management
/// 
/// Simplified Actor that holds error state for compilation

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ErrorManagerState {
    pub alerts: Vec<ErrorAlert>,
    pub notifications: Vec<ErrorAlert>,
}

// Global domain instance
static ERROR_MANAGER_DOMAIN: Lazy<Actor<ErrorManagerState>> = Lazy::new(|| {
    Actor::new(ErrorManagerState::default(), |_state| async move {
        // Simple Actor that just holds state - no event processing needed for now
        loop {
            Timer::sleep(1000).await;
        }
    })
});

// ===== SIGNAL ACCESS FUNCTIONS =====

/// Get all error alerts signal
pub fn alerts_signal() -> impl Signal<Item = Vec<ErrorAlert>> {
    ERROR_MANAGER_DOMAIN.signal().map(|state| state.alerts.clone()).dedupe_cloned()
}

/// Get all toast notifications signal
pub fn notifications_signal() -> impl Signal<Item = Vec<ErrorAlert>> {
    ERROR_MANAGER_DOMAIN.signal().map(|state| state.notifications.clone()).dedupe_cloned()
}

/// Get alert count signal
pub fn alert_count_signal() -> impl Signal<Item = usize> {
    ERROR_MANAGER_DOMAIN.signal().map(|state| state.alerts.len()).dedupe()
}

/// Get notification count signal
pub fn notification_count_signal() -> impl Signal<Item = usize> {
    ERROR_MANAGER_DOMAIN.signal().map(|state| state.notifications.len()).dedupe()
}

/// Check if any alerts exist
pub fn has_alerts_signal() -> impl Signal<Item = bool> {
    ERROR_MANAGER_DOMAIN.signal().map(|state| !state.alerts.is_empty()).dedupe()
}

/// Check if any notifications exist
pub fn has_notifications_signal() -> impl Signal<Item = bool> {
    ERROR_MANAGER_DOMAIN.signal().map(|state| !state.notifications.is_empty()).dedupe()
}

/// Get complete error manager state signal
pub fn error_manager_signal() -> impl Signal<Item = ErrorManagerState> {
    ERROR_MANAGER_DOMAIN.signal().dedupe_cloned()
}

// ===== HELPER FUNCTIONS =====

/// Add error alert (convenience function - for now just a placeholder)
pub fn add_error_alert(_alert: ErrorAlert) {
    // Placeholder implementation
}

/// Add toast notification (convenience function - for now just a placeholder)
pub fn add_toast_notification(_notification: ErrorAlert) {
    // Placeholder implementation
}

/// Remove alert by ID (convenience function - for now just a placeholder)
pub fn remove_error_alert(_alert_id: String) {
    // Placeholder implementation
}

/// Remove notification by ID (convenience function - for now just a placeholder)
pub fn remove_toast_notification(_notification_id: String) {
    // Placeholder implementation
}

// ===== INITIALIZATION =====

/// Initialize the error manager domain
pub fn initialize() {
    // Domain is automatically initialized when first accessed via Lazy
    let _ = &*ERROR_MANAGER_DOMAIN;
}