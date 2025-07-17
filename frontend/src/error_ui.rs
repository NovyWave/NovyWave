use zoon::*;
use moonzoon_novyui::components::alert::{error_alert, AlertVariant};
use moonzoon_novyui::tokens::*;
use crate::state::{ErrorAlert, ERROR_ALERTS, TOAST_NOTIFICATIONS};
use crate::error_display::dismiss_error_alert;

/// Global error alerts container that appears at the top of the main view
pub fn error_alerts_container() -> impl Element {
    Column::new()
        .s(Width::fill())
        .s(Gap::new().y(SPACING_8))
        .items_signal_vec(
            ERROR_ALERTS.signal_vec_cloned().map(|alert| {
                create_error_alert_element(alert)
            })
        )
}

/// Toast notifications container for auto-dismissing errors
pub fn toast_notifications_container() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::new().bottom().right())
        .s(Padding::all(SPACING_16))
        .update_raw_el(|raw_el| {
            raw_el
                .style("position", "fixed")
                .style("top", "0")
                .style("left", "0")
                .style("pointer-events", "none")  // Allow clicks to pass through empty areas
                .style("z-index", "1000")
        })
        .child(
            Column::new()
                .s(Gap::new().y(SPACING_8))
                .s(Width::exact(400))
                .s(Align::new().bottom().right())
                .update_raw_el(|raw_el| {
                    raw_el.style("pointer-events", "auto")  // Re-enable pointer events for toast content
                })
                .items_signal_vec(
                    TOAST_NOTIFICATIONS.signal_vec_cloned().map(|alert| {
                        create_toast_element(alert)
                    })
                )
        )
}

/// Create an error alert element using NovyUI Alert component
fn create_error_alert_element(alert: ErrorAlert) -> impl Element {
    let alert_id = alert.id.clone();
    
    error_alert(&alert.message)
        .title(&alert.title)
        .dismissible(true)
        .on_dismiss(move || {
            dismiss_error_alert(&alert_id);
        })
        .build()
}

/// Create a toast notification element with enhanced styling
fn create_toast_element(alert: ErrorAlert) -> impl Element {
    let alert_id = alert.id.clone();
    
    Row::new()
        .s(Width::fill())
        .s(Padding::all(SPACING_12))
        .s(Gap::new().x(SPACING_8))
        .s(Borders::all_signal(error_7().map(|color| Border::new().width(1).color(color))))
        .s(Background::new().color_signal(error_1()))
        .s(RoundedCorners::all(CORNER_RADIUS_8))
        .s(Align::new().center_y())
        .item(
            // Error icon
            El::new()
                .s(Font::new()
                    .size(FONT_SIZE_16)
                    .color_signal(error_9())
                )
                .child(Text::new("❌"))
        )
        .item(
            // Error content
            Column::new()
                .s(Width::fill())
                .s(Gap::new().y(SPACING_4))
                .item(
                    El::new()
                        .s(Font::new()
                            .size(FONT_SIZE_14)
                            .weight(FontWeight::SemiBold)
                            .color_signal(error_9())
                        )
                        .child(Text::new(&alert.title))
                )
                .item(
                    El::new()
                        .s(Font::new()
                            .size(FONT_SIZE_12)
                            .color_signal(error_8())
                        )
                        .child(Text::new(&alert.message))
                )
        )
        .item(
            // Dismiss button
            El::new()
                .s(Font::new()
                    .size(FONT_SIZE_14)
                    .color_signal(error_8())
                )
                .s(Cursor::new(CursorIcon::Pointer))
                .s(Padding::all(SPACING_4))
                .s(RoundedCorners::all(CORNER_RADIUS_4))
                .child(Text::new("✕"))
                .on_click(move || {
                    dismiss_error_alert(&alert_id);
                })
        )
}

/// Enhanced directory error display for TreeView items
pub fn directory_error_element(_path: &str, error: &str) -> impl Element {
    let user_friendly_error = match error.to_lowercase() {
        e if e.contains("permission denied") => "Access denied",
        e if e.contains("not found") => "Directory not found",
        e if e.contains("network") => "Network error",
        _ => "Cannot access directory",
    };
    
    Row::new()
        .s(Width::fill())
        .s(Padding::new().x(SPACING_8).y(SPACING_4))
        .s(Gap::new().x(SPACING_6))
        .s(Align::new().center_y())
        .item(
            El::new()
                .s(Font::new()
                    .size(FONT_SIZE_12)
                    .color_signal(error_8())
                )
                .child(Text::new("⚠️"))
        )
        .item(
            El::new()
                .s(Width::fill())
                .s(Font::new()
                    .size(FONT_SIZE_12)
                    .color_signal(error_9())
                    .italic()
                )
                .child(Text::new(user_friendly_error))
        )
}

/// Error badge for file loading status in file lists
pub fn file_error_badge(error: &str) -> impl Element {
    let user_friendly_error = match error.to_lowercase() {
        e if e.contains("unknown file format") => "Unsupported format",
        e if e.contains("file not found") => "File not found",
        e if e.contains("permission denied") => "Access denied",
        _ => "Error",
    };
    
    El::new()
        .s(Padding::new().x(SPACING_6).y(SPACING_2))
        .s(Background::new().color_signal(error_2()))
        .s(RoundedCorners::all(CORNER_RADIUS_4))
        .s(Font::new()
            .size(FONT_SIZE_12)
            .weight(FontWeight::Medium)
            .color_signal(error_9())
        )
        .child(Text::new(user_friendly_error))
}