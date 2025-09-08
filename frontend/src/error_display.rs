use crate::state::ErrorAlert;
use zoon::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures::StreamExt;

// Global toast management
static TOAST_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Error Display domain using proper Actor+Relay architecture
#[derive(Clone)]
pub struct ErrorDisplay {
    pub active_toasts: crate::dataflow::ActorVec<ErrorAlert>,
    pub toast_added_relay: crate::dataflow::Relay<ErrorAlert>,
    pub toast_dismissed_relay: crate::dataflow::Relay<String>,
}

impl ErrorDisplay {
    pub async fn new() -> Self {
        let (toast_added_relay, mut toast_added_stream) = crate::dataflow::relay::<ErrorAlert>();
        let (toast_dismissed_relay, mut toast_dismissed_stream) = crate::dataflow::relay::<String>();
        
        let active_toasts = crate::dataflow::ActorVec::new(vec![], async move |toasts| {
            loop {
                futures::select! {
                    toast = toast_added_stream.next() => {
                        if let Some(alert) = toast {
                            toasts.lock_mut().push_cloned(alert);
                        }
                    }
                    dismissed_id = toast_dismissed_stream.next() => {
                        if let Some(id) = dismissed_id {
                            toasts.lock_mut().retain(|alert| alert.id != id);
                        }
                    }
                }
            }
        });
        
        Self { active_toasts, toast_added_relay, toast_dismissed_relay }
    }
}

// Static instance for compatibility during migration
static ERROR_DISPLAY_INSTANCE: zoon::Lazy<std::sync::Arc<std::sync::Mutex<Option<ErrorDisplay>>>> = 
    zoon::Lazy::new(|| std::sync::Arc::new(std::sync::Mutex::new(None)));

/// Get or initialize the error display instance
pub async fn get_error_display() -> ErrorDisplay {
    let instance_arc = ERROR_DISPLAY_INSTANCE.clone();
    let mut instance_guard = instance_arc.lock().unwrap();
    
    if instance_guard.is_none() {
        let error_display = ErrorDisplay::new().await;
        *instance_guard = Some(error_display.clone());
        error_display
    } else {
        instance_guard.as_ref().unwrap().clone()
    }
}


/// Add an error alert to the global error display system
/// This is the single entry point for all error handling:
/// - Logs technical details to console (for developers)
/// - Shows user-friendly toast notification (for users)
pub async fn add_error_alert(mut alert: ErrorAlert, app_config: &crate::config::AppConfig) {
    // Log technical error to console for developers
    zoon::println!("Error: {}", alert.technical_error);
    
    // Generate unique ID for toast tracking
    let toast_id = TOAST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    alert.id = format!("toast_{}", toast_id);
    
    // Set auto-dismiss timeout from config
    if let Some(dismiss_ms) = app_config.toast_dismiss_ms_actor.signal().to_stream().next().await {
        alert.auto_dismiss_ms = dismiss_ms as u64;
    } else {
        alert.auto_dismiss_ms = 5000; // Default fallback
    }
    
    // Use proper Actor+Relay architecture
    let error_display = get_error_display().await;
    error_display.toast_added_relay.send(alert);
}

/// Log error to browser console only (no toast notification)
/// Use for background operations or non-user-initiated errors
pub fn log_error_console_only(alert: ErrorAlert) {
    // Log technical error to console for developers/debugging
    zoon::println!("Error: {}", alert.technical_error);
}

/// Dismiss an error alert by ID
pub async fn dismiss_error_alert(id: &str) {
    let error_display = get_error_display().await;
    error_display.toast_dismissed_relay.send(id.to_string());
}

/// Get the active toasts signal for UI rendering
pub async fn active_toasts_signal_vec() -> impl zoon::SignalVec<Item = ErrorAlert> {
    let error_display = get_error_display().await;
    error_display.active_toasts.signal_vec()
}

/// Initialize error display system handlers
pub fn init_error_display_system() {
    // Error display system is now ready - toast dismiss time is read from config when needed
}