//! ErrorManager domain for error management using Actor+Relay architecture
//!
//! Provides basic error management functionality.

use crate::state::ErrorAlert;
use zoon::*;

// Note: Most functionality moved to direct error handling patterns

/// Minimal error manager for basic error handling
#[derive(Clone, Debug)]
pub struct ErrorManager {
    // Minimal implementation for compatibility
}



impl ErrorManager {
    /// Create a minimal ErrorManager for compatibility
    pub async fn new() -> Self {
        Self {}
    }
}

// ===== ACTIVE FUNCTIONS IN USE =====

/// Add error alert (used by codebase)
pub fn add_error_alert(alert: ErrorAlert) {
    // Use direct error display for now
    zoon::println!("Error: {}", alert.technical_error);
}

/// Add toast notification (used by error_display.rs)
pub fn add_toast_notification(_notification: ErrorAlert) {
    // Minimal implementation
}

/// Remove error alert (used by error_display.rs)
pub fn remove_error_alert(_alert_id: String) {
    // Minimal implementation
}

/// Remove toast notification (used by error_display.rs)
pub fn remove_toast_notification(_notification_id: String) {
    // Minimal implementation
}

/// Get toast notifications signal vec (used by error_ui.rs)
pub fn toast_notifications_signal_vec() -> impl SignalVec<Item = ErrorAlert> {
    // Return empty signal vec for now
    MutableVec::new_with_values(vec![]).signal_vec_cloned()
}


/// Initialize the error manager domain (compatibility)
pub fn initialize() {
    // Minimal initialization for compatibility
}