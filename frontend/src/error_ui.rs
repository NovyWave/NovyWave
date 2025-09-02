use zoon::*;
use zoon::events::Click;
use moonzoon_novyui::components::icon::{icon, IconName, IconSize, IconColor};
use moonzoon_novyui::tokens::*;
use crate::state::ErrorAlert;
use crate::actors::error_manager::toast_notifications_signal_vec;
use crate::error_display::dismiss_error_alert;
use crate::dataflow::*;
use futures::{select, stream::StreamExt};

/// Progress percentage for toast auto-dismiss timer (0.0 to 100.0)
type Progress = f32;


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
                    toast_notifications_signal_vec().map(|alert| {
                        toast_element(alert)
                    })
                )
        )
}


/// Create a toast notification element with proper Actor-based state management
fn toast_element(alert: ErrorAlert) -> impl Element {
    
    let (toast_clicked_relay, mut toast_clicked_stream) = relay();
    let (dismiss_button_clicked_relay, mut dismiss_button_clicked_stream) = relay();
    let auto_dismiss_ms = alert.auto_dismiss_ms as f32;
    
    let toast_actor = Actor::new(100.0 as Progress, async move |state_handle| {
        let mut elapsed_time = 0.0f32;
        let mut is_paused = false;
        let update_interval_ms = 50.0f32;
        
        loop {
            select! {
                // NOTE: .fuse() required due to broken FusedFuture in oneshot::Receiver
                // See: https://github.com/rust-lang/futures-rs/issues/2455
                //      https://github.com/rust-lang/futures-rs/issues/1989
                //      https://github.com/rust-lang/futures-rs/issues/2207
                _ = Timer::sleep(update_interval_ms as u32).fuse() => {
                    if !is_paused {
                        elapsed_time += update_interval_ms;
                        
                        let remaining_percent = 100.0 - (elapsed_time / auto_dismiss_ms * 100.0);
                        let progress = remaining_percent.max(0.0);
                        state_handle.set(progress);
                        
                        if elapsed_time >= auto_dismiss_ms {
                            dismiss_error_alert(&alert.id);
                            break;
                        }
                    }
                }
                event = toast_clicked_stream.next() => {
                    if let Some(()) = event {
                        is_paused = !is_paused;
                    }
                }
                event = dismiss_button_clicked_stream.next() => {
                    if let Some(()) = event {
                        dismiss_error_alert(&alert.id);
                        break;
                    }
                }
            }
        }
    });
    
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
        .update_raw_el(|raw_el| {
            raw_el.attr("title", "Click to pause/resume auto-dismiss")
        })
        .on_click(move || toast_clicked_relay.send(()))
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
                                .child(&alert.title)
                        )
                        .item(
                            El::new()
                                .s(Font::new()
                                    .size(FONT_SIZE_14)
                                    .color_signal(error_8())
                                    .wrap_anywhere()
                                )
                                .child(&alert.message)
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
                        .child("✕")
                        .update_raw_el(move |raw_el| {
                            raw_el.event_handler(move |event: Click| {
                                event.stop_propagation();
                                dismiss_button_clicked_relay.send(());
                            })
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
                        .s(Width::percent_signal(toast_actor.signal()))
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
        .after_remove(move |_| {
            drop(toast_actor);
        })
}


