use futures::StreamExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use zoon::*;

/// Notification variant for styling different types of toasts
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum NotificationVariant {
    /// Red styling for errors
    Error,
    /// Blue styling for informational messages
    Info,
    /// Green styling for success messages
    Success,
}

impl Default for NotificationVariant {
    fn default() -> Self {
        Self::Error
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorAlert {
    pub id: String,
    pub title: String,
    pub message: String,
    pub technical_error: String, // Raw technical error for console logging
    pub auto_dismiss_ms: u64,
    /// Notification variant for styling (defaults to Error for backwards compatibility)
    pub variant: NotificationVariant,
    /// Optional action button label
    pub action_label: Option<String>,
    /// Optional progress percentage (0-100) for download progress, None = use timer-based progress
    pub progress: Option<f32>,
}

impl ErrorAlert {
    pub fn new_file_parsing_error(file_id: String, filename: String, error: String) -> Self {
        let user_friendly_message = make_error_user_friendly(&error);
        Self {
            id: format!("file_error_{}", file_id),
            title: "File Loading Error".to_string(),
            message: format!("{}: {}", filename, user_friendly_message),
            technical_error: format!("Error parsing file {}: {}", file_id, error),
            auto_dismiss_ms: 5000,
            variant: NotificationVariant::Error,
            action_label: None,
            progress: None,
        }
    }

    pub fn new_directory_error(path: String, error: String) -> Self {
        let user_friendly_message = make_error_user_friendly(&error);
        Self {
            id: format!("dir_error_{}", path.replace("/", "_")),
            title: "Directory Access Error".to_string(),
            message: format!("Cannot access {}: {}", path, user_friendly_message),
            technical_error: format!("Error browsing directory {}: {}", path, error),
            auto_dismiss_ms: 5000,
            variant: NotificationVariant::Error,
            action_label: None,
            progress: None,
        }
    }

    pub fn new_connection_error(error: String) -> Self {
        let user_friendly_message = make_error_user_friendly(&error);
        Self {
            id: format!("conn_error_{}", js_sys::Date::now() as u64),
            title: "Connection Error".to_string(),
            message: user_friendly_message,
            technical_error: format!("Connection error: {}", error),
            auto_dismiss_ms: 5000,
            variant: NotificationVariant::Error,
            action_label: None,
            progress: None,
        }
    }

    pub fn new_clipboard_error(error: String) -> Self {
        Self {
            id: format!("clipboard_error_{}", js_sys::Date::now() as u64),
            title: "Clipboard Error".to_string(),
            message: "Failed to copy to clipboard. Your browser may not support clipboard access or you may need to use HTTPS.".to_string(),
            technical_error: format!("Clipboard operation failed: {}", error),
            auto_dismiss_ms: 5000,
            variant: NotificationVariant::Error,
            action_label: None,
            progress: None,
        }
    }

    /// Create an update available notification
    pub fn new_update_available(current_version: String, new_version: String) -> Self {
        Self {
            id: "update_available".to_string(),
            title: "Update Available".to_string(),
            message: format!("v{} → v{}", current_version, new_version),
            technical_error: format!("Update available: {} -> {}", current_version, new_version),
            auto_dismiss_ms: 0, // No auto-dismiss for update notifications
            variant: NotificationVariant::Info,
            action_label: Some("Download".to_string()),
            progress: None,
        }
    }

    /// Create an update downloading notification with progress
    pub fn new_update_downloading(progress_percent: f32) -> Self {
        Self {
            id: "update_downloading".to_string(),
            title: "Downloading Update".to_string(),
            message: format!("{:.0}%", progress_percent),
            technical_error: format!("Downloading update: {}%", progress_percent),
            auto_dismiss_ms: 0, // No auto-dismiss while downloading
            variant: NotificationVariant::Info,
            action_label: None,
            progress: Some(progress_percent),
        }
    }

    /// Create an update ready notification
    pub fn new_update_ready(new_version: String) -> Self {
        Self {
            id: "update_ready".to_string(),
            title: "Update Ready".to_string(),
            message: format!("v{} is ready to install", new_version),
            technical_error: format!("Update {} ready to install", new_version),
            auto_dismiss_ms: 0, // No auto-dismiss, user must click restart
            variant: NotificationVariant::Success,
            action_label: Some("Restart".to_string()),
            progress: None,
        }
    }

    /// Create an update error notification
    pub fn new_update_error(error: String) -> Self {
        Self {
            id: format!("update_error_{}", js_sys::Date::now() as u64),
            title: "Update Failed".to_string(),
            message: make_error_user_friendly(&error),
            technical_error: format!("Update error: {}", error),
            auto_dismiss_ms: 8000,
            variant: NotificationVariant::Error,
            action_label: None,
            progress: None,
        }
    }
}

pub fn make_error_user_friendly(error: &str) -> String {
    let error_lower = error.to_lowercase();

    // Extract file path from error messages in multiple formats:
    // - "Failed to parse waveform file '/path/to/file': error" (quoted format)
    // - "File not found: /path/to/file" (backend format)
    let file_path = if let Some(start) = error.find("'") {
        if let Some(end) = error[start + 1..].find("'") {
            Some(&error[start + 1..start + 1 + end])
        } else {
            None
        }
    } else if error_lower.contains("file not found:") {
        // Extract path after "File not found: "
        if let Some(colon_pos) = error.find("File not found:") {
            let path_start = colon_pos + "File not found:".len();
            Some(error[path_start..].trim())
        } else {
            None
        }
    } else {
        None
    };

    if error_lower.contains("unknown file format")
        || error_lower.contains("only ghw, fst and vcd are supported")
    {
        if let Some(path) = file_path {
            format!(
                "Unsupported file format '{}'. Only VCD, FST, and GHW files are supported.",
                path
            )
        } else {
            "Unsupported file format. Only VCD, FST, and GHW files are supported.".to_string()
        }
    } else if error_lower.contains("file not found") || error_lower.contains("no such file") {
        if let Some(path) = file_path {
            format!(
                "File not found '{}'. Please check if the file exists and try again.",
                path
            )
        } else {
            "File not found. Please check if the file exists and try again.".to_string()
        }
    } else if error_lower.contains("permission denied") || error_lower.contains("access denied") {
        "Can't access this directory".to_string()
    } else if error_lower.contains("connection") || error_lower.contains("network") {
        "Connection error. Please check your network connection.".to_string()
    } else if error_lower.contains("timeout") {
        "Operation timed out. Please try again.".to_string()
    } else {
        // Keep original error but make it more presentable
        error.trim().to_string()
    }
}

// Global toast management
#[allow(dead_code)]
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
        let (toast_dismissed_relay, mut toast_dismissed_stream) =
            crate::dataflow::relay::<String>();

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

        Self {
            active_toasts,
            toast_added_relay,
            toast_dismissed_relay,
        }
    }
}

#[allow(dead_code)]
pub async fn add_error_alert(mut alert: ErrorAlert, app_config: &crate::config::AppConfig) {
    zoon::println!("Error: {}", alert.technical_error);

    let toast_id = TOAST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    alert.id = format!("toast_{}", toast_id);

    if let Some(dismiss_ms) = app_config
        .toast_dismiss_ms_actor
        .signal()
        .to_stream()
        .next()
        .await
    {
        alert.auto_dismiss_ms = dismiss_ms as u64;
    } else {
        alert.auto_dismiss_ms = 5000;
    }

    app_config.error_display.toast_added_relay.send(alert);
}

/// Log error to browser console only (no toast notification)
/// Use for background operations or non-user-initiated errors
#[allow(dead_code)]
pub fn log_error_console_only(alert: ErrorAlert) {
    // Log technical error to console for developers/debugging
    zoon::println!("Error: {}", alert.technical_error);
}

#[allow(dead_code)]
pub async fn dismiss_error_alert(id: &str, app_config: &crate::config::AppConfig) {
    app_config
        .error_display
        .toast_dismissed_relay
        .send(id.to_string());
}

pub fn active_toasts_signal_vec(
    app_config: crate::config::AppConfig,
) -> impl zoon::SignalVec<Item = ErrorAlert> {
    app_config.error_display.active_toasts.signal_vec()
}

/// Trigger test notifications to demonstrate the notification system.
/// Shows one notification of each variant type (Error, Info, Success).
pub async fn trigger_test_notifications(app_config: &crate::config::AppConfig) {
    zoon::println!("🔔 Triggering test notifications...");

    // Test Error notification
    let error_alert = ErrorAlert {
        id: format!("test_error_{}", js_sys::Date::now() as u64),
        title: "Test Error Notification".to_string(),
        message: "This is a sample error message to demonstrate red styling.".to_string(),
        technical_error: "Test error for demonstration".to_string(),
        auto_dismiss_ms: 5000,
        variant: NotificationVariant::Error,
        action_label: None,
        progress: None,
    };
    add_error_alert(error_alert, app_config).await;

    // Small delay between notifications so they stack nicely
    zoon::Timer::sleep(300).await;

    // Test Info notification (like update available)
    let info_alert = ErrorAlert {
        id: format!("test_info_{}", js_sys::Date::now() as u64),
        title: "Test Info Notification".to_string(),
        message: "This is a sample info message with blue styling.".to_string(),
        technical_error: "Test info for demonstration".to_string(),
        auto_dismiss_ms: 5000,
        variant: NotificationVariant::Info,
        action_label: Some("Action".to_string()),
        progress: None,
    };
    add_error_alert(info_alert, app_config).await;

    // Small delay between notifications
    zoon::Timer::sleep(300).await;

    // Test Success notification (like update ready)
    let success_alert = ErrorAlert {
        id: format!("test_success_{}", js_sys::Date::now() as u64),
        title: "Test Success Notification".to_string(),
        message: "This is a sample success message with green styling.".to_string(),
        technical_error: "Test success for demonstration".to_string(),
        auto_dismiss_ms: 5000,
        variant: NotificationVariant::Success,
        action_label: Some("Confirm".to_string()),
        progress: None,
    };
    add_error_alert(success_alert, app_config).await;

    zoon::println!("✅ Test notifications triggered!");
}
