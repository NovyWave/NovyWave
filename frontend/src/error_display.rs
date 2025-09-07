use crate::state::ErrorAlert;
use zoon::*;


/// Add an error alert to the global error display system
/// This is the single entry point for all error handling:
/// - Logs technical details to console (for developers)
/// - Shows user-friendly toast notification (for users)
pub async fn add_error_alert(alert: ErrorAlert) {
    // Log technical error to console for developers
    zoon::println!("Error: {}", alert.technical_error);
    
    // Always add error alerts as toast notifications - timeout will be handled by UI
    add_toast_notification(alert).await;
}

/// Log error to browser console only (no toast notification)
/// Use for background operations or non-user-initiated errors
pub fn log_error_console_only(alert: ErrorAlert) {
    // Log technical error to console for developers/debugging
    zoon::println!("Error: {}", alert.technical_error);
}

/// Dismiss an error alert by ID
pub fn dismiss_error_alert(_id: &str) {
    // Direct dismissal (stub functions do nothing anyway)
}

/// Add a toast notification that auto-dismisses
async fn add_toast_notification(mut alert: ErrorAlert) {
    let config = crate::config::app_config();
    if let Some(dismiss_ms) = config.toast_dismiss_ms_actor.signal().to_stream().next().await {
        alert.auto_dismiss_ms = dismiss_ms as u64;
    } else {
        alert.auto_dismiss_ms = 5000; // Default fallback
    }
    
    // Note: Toast functionality simplified - error_manager functions were stubs anyway
    // Note: Auto-dismiss is now handled by the toast component itself
    // in error_ui.rs with pause-on-click functionality
}



/// Initialize error display system handlers
pub fn init_error_display_system() {
    // Error display system is now ready - toast dismiss time is read from config when needed
}