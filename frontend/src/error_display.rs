use crate::state::ErrorAlert;
use zoon::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// Global toast management
static TOAST_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_TOASTS: zoon::Lazy<zoon::MutableVec<ErrorAlert>> = zoon::Lazy::new(|| zoon::MutableVec::new());


/// Add an error alert to the global error display system
/// This is the single entry point for all error handling:
/// - Logs technical details to console (for developers)
/// - Shows user-friendly toast notification (for users)
pub async fn add_error_alert(mut alert: ErrorAlert) {
    // Log technical error to console for developers
    zoon::println!("Error: {}", alert.technical_error);
    
    // Generate unique ID for toast tracking
    let toast_id = TOAST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    alert.id = format!("toast_{}", toast_id);
    
    // Set auto-dismiss timeout from config
    let config = crate::config::app_config();
    if let Some(dismiss_ms) = config.toast_dismiss_ms_actor.signal().to_stream().next().await {
        alert.auto_dismiss_ms = dismiss_ms as u64;
    } else {
        alert.auto_dismiss_ms = 5000; // Default fallback
    }
    
    // Add to active toasts for UI display
    ACTIVE_TOASTS.lock_mut().push_cloned(alert);
}

/// Log error to browser console only (no toast notification)
/// Use for background operations or non-user-initiated errors
pub fn log_error_console_only(alert: ErrorAlert) {
    // Log technical error to console for developers/debugging
    zoon::println!("Error: {}", alert.technical_error);
}

/// Dismiss an error alert by ID
pub fn dismiss_error_alert(id: &str) {
    ACTIVE_TOASTS.lock_mut().retain(|alert| alert.id != id);
}

/// Get the active toasts signal for UI rendering
pub fn active_toasts_signal_vec() -> impl zoon::SignalVec<Item = ErrorAlert> {
    ACTIVE_TOASTS.signal_vec_cloned()
}

/// Initialize error display system handlers
pub fn init_error_display_system() {
    // Error display system is now ready - toast dismiss time is read from config when needed
}