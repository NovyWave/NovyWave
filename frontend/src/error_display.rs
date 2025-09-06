use crate::state::ErrorAlert;
use crate::actors::error_manager::{
    add_error_alert as add_domain_alert, add_toast_notification as add_domain_notification,
    remove_error_alert, remove_toast_notification
};
use zoon::*;


/// Add an error alert to the global error display system
/// This is the single entry point for all error handling:
/// - Logs technical details to console (for developers)
/// - Shows user-friendly toast notification (for users)
pub async fn add_error_alert(alert: ErrorAlert) {
    // Log technical error to console for developers
    
    // Add new alert using domain function
    add_domain_alert(alert.clone());
    
    // Always add error alerts as toast notifications - timeout will be handled by UI
    add_toast_notification(alert).await;
}

/// Log error to browser console only (no toast notification)
/// Use for background operations or non-user-initiated errors
pub fn log_error_console_only(alert: ErrorAlert) {
    // Log technical error to console for developers/debugging
    
    // Add to domain for error tracking but don't show toast
    add_domain_alert(alert);
}

/// Dismiss an error alert by ID
pub fn dismiss_error_alert(id: &str) {
    remove_error_alert(id.to_string());
    remove_toast_notification(id.to_string());
}

/// Add a toast notification that auto-dismisses
async fn add_toast_notification(mut alert: ErrorAlert) {
    let config = crate::config::app_config();
    if let Some(dismiss_ms) = config.toast_dismiss_ms_actor.signal().to_stream().next().await {
        alert.auto_dismiss_ms = dismiss_ms as u64;
    } else {
        alert.auto_dismiss_ms = 5000; // Default fallback
    }
    
    // Add new toast using domain function
    add_domain_notification(alert.clone());
    
    // Note: Auto-dismiss is now handled by the toast component itself
    // in error_ui.rs with pause-on-click functionality
}



/// Initialize error display system handlers
pub fn init_error_display_system() {
    // Error display system is now ready - toast dismiss time is read from config when needed
}