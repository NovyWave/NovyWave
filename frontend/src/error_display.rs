use crate::state::ErrorAlert;
use crate::actors::error_manager::{
    add_error_alert as add_domain_alert, add_toast_notification as add_domain_notification,
    remove_error_alert, remove_toast_notification
};

/// Add an error alert to the global error display system
/// This is the single entry point for all error handling:
/// - Logs technical details to console (for developers)
/// - Shows user-friendly toast notification (for users)
pub fn add_error_alert(alert: ErrorAlert) {
    // Log technical error to console for developers
    zoon::eprintln!("{}", alert.technical_error);
    
    // Add new alert using domain function
    add_domain_alert(alert.clone());
    
    // If it has auto-dismiss, also add to toast system
    if alert.auto_dismiss_ms.is_some() {
        add_toast_notification(alert);
    }
}

/// Dismiss an error alert by ID
pub fn dismiss_error_alert(id: &str) {
    remove_error_alert(id.to_string());
    remove_toast_notification(id.to_string());
}

/// Add a toast notification that auto-dismisses
fn add_toast_notification(alert: ErrorAlert) {
    // Add new toast using domain function
    add_domain_notification(alert.clone());
    
    // Note: Auto-dismiss is now handled by the toast component itself
    // in error_ui.rs with pause-on-click functionality
}



/// Initialize error display system handlers
pub fn init_error_display_system() {
    // No additional initialization needed currently
    // Auto-dismiss logic is handled in add_toast_notification
}