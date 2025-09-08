/// Copy text to clipboard using Actor+Relay architecture
pub fn copy_to_clipboard(text: String, app_config: &crate::config::AppConfig) {
    app_config.clipboard_copy_requested_relay.send(text);
}

/// User-facing convenience function for copying variable values
pub fn copy_variable_value(value: &str, app_config: &crate::config::AppConfig) {
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
    
    copy_to_clipboard(filtered_value, app_config);
}