use crate::state::ErrorAlert;
use crate::actors::error_manager::{
    add_error_alert as add_domain_alert, add_toast_notification as add_domain_notification,
    remove_error_alert, remove_toast_notification
};
use zoon::*;

// Cache for toast dismiss time from config
static TOAST_DISMISS_MS: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(5000));

/// Add an error alert to the global error display system
/// This is the single entry point for all error handling:
/// - Logs technical details to console (for developers)
/// - Shows user-friendly toast notification (for users)
pub fn add_error_alert(alert: ErrorAlert) {
    // Log technical error to console for developers
    zoon::eprintln!("{}", alert.technical_error);
    
    // Add new alert using domain function
    add_domain_alert(alert.clone());
    
    // Always add error alerts as toast notifications - timeout will be handled by UI
    add_toast_notification(alert);
}

/// Dismiss an error alert by ID
pub fn dismiss_error_alert(id: &str) {
    remove_error_alert(id.to_string());
    remove_toast_notification(id.to_string());
}

/// Add a toast notification that auto-dismisses
fn add_toast_notification(mut alert: ErrorAlert) {
    // Always ensure auto-dismiss time is set from config
    alert.auto_dismiss_ms = TOAST_DISMISS_MS.get();
    
    // Add new toast using domain function
    add_domain_notification(alert.clone());
    
    // Note: Auto-dismiss is now handled by the toast component itself
    // in error_ui.rs with pause-on-click functionality
}



/// Initialize error display system handlers
pub fn init_error_display_system() {
    // Set initial value and sync toast dismiss time with config actor
    Task::start(async {
        // Get initial value from config
        if let Some(initial_ms) = crate::config::app_config()
            .toast_dismiss_ms_actor
            .signal()
            .to_stream()
            .next()
            .await {
            TOAST_DISMISS_MS.set_neq(initial_ms as u64);
        }
        
        // Keep syncing with config changes
        crate::config::app_config()
            .toast_dismiss_ms_actor
            .signal()
            .for_each(|dismiss_ms| {
                TOAST_DISMISS_MS.set_neq(dismiss_ms as u64);
                async {}
            })
            .await;
    });
}