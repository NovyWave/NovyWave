use zoon::*;
use crate::state::{ErrorAlert, ERROR_ALERTS, TOAST_NOTIFICATIONS};

/// Add an error alert to the global error display system
/// This is the single entry point for all error handling:
/// - Logs technical details to console (for developers)
/// - Shows user-friendly toast notification (for users)
pub fn add_error_alert(alert: ErrorAlert) {
    // Log technical error to console for developers
    zoon::eprintln!("{}", alert.technical_error);
    
    // Remove existing alert with same ID to prevent duplicates
    ERROR_ALERTS.lock_mut().retain(|existing| existing.id != alert.id);
    
    // Add new alert
    ERROR_ALERTS.lock_mut().push_cloned(alert.clone());
    
    // If it has auto-dismiss, also add to toast system
    if alert.auto_dismiss_ms.is_some() {
        add_toast_notification(alert);
    }
}

/// Dismiss an error alert by ID
pub fn dismiss_error_alert(id: &str) {
    ERROR_ALERTS.lock_mut().retain(|alert| alert.id != id);
    TOAST_NOTIFICATIONS.lock_mut().retain(|alert| alert.id != id);
}

/// Add a toast notification that auto-dismisses
fn add_toast_notification(alert: ErrorAlert) {
    // Remove existing toast with same ID
    TOAST_NOTIFICATIONS.lock_mut().retain(|existing| existing.id != alert.id);
    
    // Add new toast
    TOAST_NOTIFICATIONS.lock_mut().push_cloned(alert.clone());
    
    // Auto-dismiss after specified time
    if let Some(dismiss_ms) = alert.auto_dismiss_ms {
        let alert_id = alert.id.clone();
        Task::start(async move {
            Timer::sleep(dismiss_ms as u32).await;
            dismiss_error_alert(&alert_id);
        });
    }
}

/// Clear all error alerts
pub fn clear_all_error_alerts() {
    ERROR_ALERTS.lock_mut().clear();
    TOAST_NOTIFICATIONS.lock_mut().clear();
}

/// Get current error alerts count
pub fn error_alerts_count() -> impl Signal<Item = usize> {
    ERROR_ALERTS.signal_vec_cloned().len()
}

/// Initialize error display system handlers
pub fn init_error_display_system() {
    // No additional initialization needed currently
    // Auto-dismiss logic is handled in add_toast_notification
}