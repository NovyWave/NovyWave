use wasm_bindgen_futures::spawn_local;
use web_sys::window;
use crate::error_display::add_error_alert;
use crate::state::ErrorAlert;

/// Copy text to clipboard with modern API support
pub fn copy_to_clipboard(text: String) {
    spawn_local(async move {
        if let Some(window) = window() {
            let navigator = window.navigator();
            
            #[cfg(web_sys_unstable_apis)]
            {
                let clipboard = navigator.clipboard();
                match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text)).await {
                    Ok(_) => {
                        // Clipboard copy successful - no logging needed for normal operation
                    }
                    Err(e) => {
                        let error_alert = ErrorAlert::new_clipboard_error(format!("{:?}", e));
                        add_error_alert(error_alert).await;
                        // Could implement fallback here if needed
                    }
                }
            }
            
            #[cfg(not(web_sys_unstable_apis))]
            {
                let error_alert = ErrorAlert::new_clipboard_error("Clipboard API requires unstable APIs flag".to_string());
                add_error_alert(error_alert).await;
            }
        }
    });
}

/// User-facing convenience function for copying variable values
pub fn copy_variable_value(value: &str) {
    // Clean up the value - remove control characters but keep newlines/tabs
    let filtered_bytes: Vec<u8> = value
        .bytes()
        .filter(|&b| b.is_ascii() && (!b.is_ascii_control() || b == b'\n' || b == b'\t'))
        .collect();
    
    let filtered_value = String::from_utf8(filtered_bytes)
        .unwrap_or_else(|_| {
            // Fallback: strip all non-ASCII and try again
            value.chars()
                .filter(|c| c.is_ascii() && (!c.is_control() || *c == '\n' || *c == '\t'))
                .collect()
        })
        .trim()
        .to_string();
    
    copy_to_clipboard(filtered_value);
}