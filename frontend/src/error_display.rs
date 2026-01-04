use std::sync::atomic::{AtomicUsize, Ordering};
use zoon::*;

/// Notification variant for styling different types of toasts
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum NotificationVariant {
    Error,
    Info,
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
    pub technical_error: String,
    pub auto_dismiss_ms: u64,
    pub variant: NotificationVariant,
    pub action_label: Option<String>,
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

    pub fn new_update_available(current_version: String, new_version: String) -> Self {
        Self {
            id: "update_available".to_string(),
            title: "Update Available".to_string(),
            message: format!("v{} → v{}", current_version, new_version),
            technical_error: format!("Update available: {} -> {}", current_version, new_version),
            auto_dismiss_ms: 0,
            variant: NotificationVariant::Info,
            action_label: Some("Download".to_string()),
            progress: None,
        }
    }

    pub fn new_update_downloading(progress_percent: f32) -> Self {
        Self {
            id: "update_downloading".to_string(),
            title: "Downloading Update".to_string(),
            message: format!("{:.0}%", progress_percent),
            technical_error: format!("Downloading update: {}%", progress_percent),
            auto_dismiss_ms: 0,
            variant: NotificationVariant::Info,
            action_label: None,
            progress: Some(progress_percent),
        }
    }

    pub fn new_update_ready(new_version: String) -> Self {
        Self {
            id: "update_ready".to_string(),
            title: "Update Ready".to_string(),
            message: format!("v{} is ready to install", new_version),
            technical_error: format!("Update {} ready to install", new_version),
            auto_dismiss_ms: 0,
            variant: NotificationVariant::Success,
            action_label: Some("Restart".to_string()),
            progress: None,
        }
    }

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

    let file_path = if let Some(start) = error.find("'") {
        if let Some(end) = error[start + 1..].find("'") {
            Some(&error[start + 1..start + 1 + end])
        } else {
            None
        }
    } else if error_lower.contains("file not found:") {
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
        error.trim().to_string()
    }
}

#[allow(dead_code)]
static TOAST_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Error Display domain - simple MutableVec with direct methods (no Relay/Task)
#[derive(Clone)]
pub struct ErrorDisplay {
    pub active_toasts: MutableVec<ErrorAlert>,
}

impl ErrorDisplay {
    pub fn new() -> Self {
        Self {
            active_toasts: MutableVec::new(),
        }
    }

    /// Add a toast notification directly
    pub fn add_toast(&self, alert: ErrorAlert) {
        self.active_toasts.lock_mut().push_cloned(alert);
    }

    /// Dismiss a toast by ID directly
    pub fn dismiss_toast(&self, id: &str) {
        self.active_toasts.lock_mut().retain(|alert| alert.id != id);
    }
}

impl Default for ErrorDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub async fn add_error_alert(mut alert: ErrorAlert, app_config: &crate::config::AppConfig) {
    zoon::println!("Error: {}", alert.technical_error);

    let toast_id = TOAST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    alert.id = format!("toast_{}", toast_id);

    if let Some(dismiss_ms) = app_config
        .toast_dismiss_ms
        .signal()
        .to_stream()
        .next()
        .await
    {
        alert.auto_dismiss_ms = dismiss_ms as u64;
    } else {
        alert.auto_dismiss_ms = 5000;
    }

    app_config.error_display.add_toast(alert);
}

#[allow(dead_code)]
pub fn log_error_console_only(alert: ErrorAlert) {
    zoon::println!("Error: {}", alert.technical_error);
}

#[allow(dead_code)]
pub fn dismiss_error_alert(id: &str, app_config: &crate::config::AppConfig) {
    app_config.error_display.dismiss_toast(id);
}

pub fn active_toasts_signal_vec(
    app_config: crate::config::AppConfig,
) -> impl zoon::SignalVec<Item = ErrorAlert> {
    app_config.error_display.active_toasts.signal_vec_cloned()
}

pub async fn trigger_test_notifications(app_config: &crate::config::AppConfig) {
    zoon::println!("🔔 Triggering test notifications...");

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

    zoon::Timer::sleep(300).await;

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

    zoon::Timer::sleep(300).await;

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
