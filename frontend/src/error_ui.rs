use zoon::*;
use moonzoon_novyui::components::icon::{icon, IconName, IconSize, IconColor};
use moonzoon_novyui::tokens::*;
use crate::state::ErrorAlert;
use crate::actors::error_manager::toast_notifications_signal;
use crate::error_display::dismiss_error_alert;


/// Toast notifications container for auto-dismissing errors
pub fn toast_notifications_container() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::new().top().right())
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
                .s(Align::new().top().right())
                .update_raw_el(|raw_el| {
                    raw_el.style("pointer-events", "auto")  // Re-enable pointer events for toast content
                })
                .items_signal_vec(
                    toast_notifications_signal().to_signal_vec().map(|alert| {
                        create_toast_element(alert)
                    })
                )
        )
}


/// Create a toast notification element with enhanced styling and progress bar countdown
fn create_toast_element(alert: ErrorAlert) -> impl Element {
    let alert_id = alert.id.clone();
    let dismiss_alert_id = alert_id.clone();
    
    // Create progress signal for pausable progress bar animation
    let is_progress_paused = Mutable::new(false);
    let progress_signal = {
        let auto_dismiss_ms = alert.auto_dismiss_ms.unwrap_or(10_000);
        let update_interval_ms = 50; // Update every 50ms for smooth animation
        let total_updates = auto_dismiss_ms / update_interval_ms;
        
        let progress = Mutable::new(100.0);
        let progress_clone = progress.clone();
        let is_paused_clone = is_progress_paused.clone();
        
        // Start pausable progress bar animation
        Task::start(async move {
            let mut current_update = 0;
            while current_update <= total_updates {
                // Only update progress bar AND increment counter if not paused
                if !is_paused_clone.get() {
                    let remaining_percent = 100.0 - (current_update as f64 / total_updates as f64 * 100.0);
                    progress_clone.set(remaining_percent.max(0.0));
                    current_update += 1;
                }
                Timer::sleep(update_interval_ms as u32).await;
            }
        });
        
        progress.signal()
    };
    
    // Auto-dismiss countdown that can be stopped by clicking
    if alert.auto_dismiss_ms.is_some() {
        let alert_id_dismiss = alert_id.clone();
        let auto_dismiss_ms = alert.auto_dismiss_ms.unwrap_or(10_000);
        let is_paused_dismiss = is_progress_paused.clone();
        Task::start(async move {
            let update_interval_ms = 50;
            let total_updates = auto_dismiss_ms / update_interval_ms;
            let mut current_update = 0;
            
            while current_update < total_updates {
                // Check if paused - if so, wait until unpaused
                if is_paused_dismiss.get() {
                    Timer::sleep(update_interval_ms as u32).await;
                    continue; // Wait and check again, don't increment counter
                }
                
                current_update += 1;
                Timer::sleep(update_interval_ms as u32).await;
            }
            
            // Only dismiss if countdown completed naturally (not paused)
            if !is_paused_dismiss.get() {
                dismiss_error_alert(&alert_id_dismiss);
            }
        });
    }
    
    Column::new()
        .s(Width::fill())
        .s(Background::new().color_signal(error_1()))
        .s(Borders::all_signal(error_7().map(|color| Border::new().width(1).color(color))))
        .s(RoundedCorners::all(CORNER_RADIUS_8))
        .s(Shadows::new(vec![
            Shadow::new()
                .color(hsluv!(0, 0, 0, 10))
                .x(0)
                .y(2)
                .blur(8)
        ]))
        .s(Cursor::new(CursorIcon::Pointer))
        .update_raw_el({
            let is_paused_tooltip = is_progress_paused.clone();
            move |raw_el| {
                raw_el.attr_signal("title", is_paused_tooltip.signal().map(|is_paused| {
                    if is_paused {
                        "Click to resume auto-dismiss"
                    } else {
                        "Click to pause auto-dismiss"
                    }
                }))
            }
        })
        .on_click({
            let is_paused_click = is_progress_paused.clone();
            move || {
                is_paused_click.set_neq(!is_paused_click.get());
            }
        })
        .item(
            // Main toast content
            Row::new()
                .s(Width::fill())
                .s(Padding::all(SPACING_12))
                .s(Gap::new().x(SPACING_8))
                .s(Align::new().center_y())
                .item(
                    // Error icon
                    icon(IconName::TriangleAlert)
                        .size(IconSize::Medium)
                        .color(IconColor::Error)
                        .build()
                )
                .item(
                    // Error content
                    Column::new()
                        .s(Width::fill())
                        .s(Gap::new().y(SPACING_4))
                        .item(
                            El::new()
                                .s(Font::new()
                                    .size(FONT_SIZE_16)
                                    .weight(FontWeight::SemiBold)
                                    .color_signal(error_9())
                                )
                                .child(Text::new(&alert.title))
                        )
                        .item(
                            El::new()
                                .s(Font::new()
                                    .size(FONT_SIZE_14)
                                    .color_signal(error_8())
                                    .wrap_anywhere()
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
                            dismiss_error_alert(&dismiss_alert_id);
                        })
                )
        )
        .item(
            // Progress bar container
            El::new()
                .s(Width::fill())
                .s(Height::exact(3))
                .s(Background::new().color_signal(error_3()))
                .s(RoundedCorners::new()
                    .bottom_left(CORNER_RADIUS_8)
                    .bottom_right(CORNER_RADIUS_8)
                )
                .child(
                    // Progress bar fill
                    El::new()
                        .s(Height::fill())
                        .s(Width::with_signal_self(progress_signal.map(|progress| Width::percent(progress as u32))))
                        .s(Background::new().color_signal(error_7()))
                        .s(RoundedCorners::new()
                            .bottom_left(CORNER_RADIUS_8)
                            .bottom_right(CORNER_RADIUS_8)
                        )
                        .s(Transitions::new([
                            Transition::property("width").duration(150)
                        ]))
                        .update_raw_el(|raw_el| {
                            raw_el.style("transform-origin", "left")
                        })
                )
        )
}


