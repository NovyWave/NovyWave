use crate::dataflow::*;
use crate::error_display::{ErrorAlert, NotificationVariant};
use futures::{select, stream::StreamExt};
use moonzoon_novyui::components::icon::{IconColor, IconName, IconSize, icon};
use moonzoon_novyui::tokens::*;
use zoon::events::Click;
use zoon::*;

/// Get the appropriate icon for a notification variant
fn variant_icon(variant: NotificationVariant) -> IconName {
    match variant {
        NotificationVariant::Error => IconName::TriangleAlert,
        NotificationVariant::Info => IconName::Info,
        NotificationVariant::Success => IconName::CircleCheck,
    }
}

/// Get the appropriate icon color for a notification variant
fn variant_icon_color(variant: NotificationVariant) -> IconColor {
    match variant {
        NotificationVariant::Error => IconColor::Error,
        NotificationVariant::Info => IconColor::Primary,
        NotificationVariant::Success => IconColor::Success,
    }
}

/// Wrapper type for variant-specific color signals
#[derive(Clone, Copy)]
struct VariantColors {
    variant: NotificationVariant,
}

impl VariantColors {
    fn new(variant: NotificationVariant) -> Self {
        Self { variant }
    }

    /// Background color (1-level)
    fn background(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(12% 0.03 30)", // error_1 dark
            NotificationVariant::Info => "oklch(20% 0.01 250)", // primary_1 dark
            NotificationVariant::Success => "oklch(12% 0.03 145)", // success_1 dark
        })
    }

    /// Border color (7-level)
    fn border(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(70% 0.21 30)", // error_7 dark
            NotificationVariant::Info => "oklch(65% 0.16 250)", // primary_7 dark
            NotificationVariant::Success => "oklch(70% 0.15 145)", // success_7 dark
        })
    }

    /// Title text color (9-level)
    fn title(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(88% 0.18 30)", // error_9 dark
            NotificationVariant::Info => "oklch(85% 0.14 250)", // primary_9 dark
            NotificationVariant::Success => "oklch(88% 0.13 145)", // success_9 dark
        })
    }

    /// Message text color (8-level)
    fn message(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(80% 0.21 30)", // error_8 dark
            NotificationVariant::Info => "oklch(75% 0.16 250)", // primary_8 dark
            NotificationVariant::Success => "oklch(80% 0.15 145)", // success_8 dark
        })
    }

    /// Progress bar background (3-level)
    fn progress_bg(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(30% 0.09 30)", // error_3 dark
            NotificationVariant::Info => "oklch(30% 0.05 250)", // primary_3 dark
            NotificationVariant::Success => "oklch(30% 0.07 145)", // success_3 dark
        })
    }

    /// Progress bar fill (7-level)
    fn progress_fill(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(70% 0.21 30)", // error_7 dark
            NotificationVariant::Info => "oklch(65% 0.16 250)", // primary_7 dark
            NotificationVariant::Success => "oklch(70% 0.15 145)", // success_7 dark
        })
    }
}

/// Progress percentage for toast auto-dismiss timer (0.0 to 100.0)
type Progress = f32;

pub fn toast_notifications_container(app_config: crate::config::AppConfig) -> impl Element {
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
                .style("pointer-events", "none") // Allow clicks to pass through empty areas
                .style("z-index", "1000")
        })
        .child(
            Column::new()
                .s(Gap::new().y(SPACING_8))
                .s(Width::exact(400))
                .s(Align::new().top().right())
                .update_raw_el(|raw_el| {
                    raw_el.style("pointer-events", "auto") // Re-enable pointer events for toast content
                })
                .items_signal_vec(
                    crate::error_display::active_toasts_signal_vec(app_config.clone()).map({
                        let app_config_for_toast = app_config.clone();
                        move |alert: ErrorAlert| toast_element(alert, app_config_for_toast.clone())
                    }),
                ),
        )
}

fn toast_element(alert: ErrorAlert, app_config: crate::config::AppConfig) -> impl Element {
    let (toast_clicked_relay, mut toast_clicked_stream) = relay();
    let (dismiss_button_clicked_relay, mut dismiss_button_clicked_stream) = relay();
    let (action_button_clicked_relay, mut action_button_clicked_stream) = relay();

    let auto_dismiss_ms = alert.auto_dismiss_ms as f32;
    let has_auto_dismiss = auto_dismiss_ms > 0.0;
    let has_custom_progress = alert.progress.is_some();
    let custom_progress = alert.progress.unwrap_or(0.0);
    let variant = alert.variant;
    let action_label = alert.action_label.clone();

    let error_display = app_config.error_display.clone();
    let alert_id = alert.id.clone();
    let alert_id_for_action = alert.id.clone();

    let colors = VariantColors::new(variant);

    let toast_actor = if has_custom_progress {
        Actor::new(custom_progress as Progress, async move |_state_handle| {
            loop {
                select! {
                    event = dismiss_button_clicked_stream.next() => {
                        if let Some(()) = event {
                            error_display.toast_dismissed_relay.send(alert_id.clone());
                            break;
                        }
                    }
                    event = action_button_clicked_stream.next() => {
                        if let Some(()) = event {
                            break;
                        }
                    }
                }
            }
        })
    } else if has_auto_dismiss {
        Actor::new(100.0 as Progress, async move |state_handle| {
            let mut elapsed_time = 0.0f32;
            let mut is_paused = false;
            let update_interval_ms = 50.0f32;

            loop {
                select! {
                    // NOTE: .fuse() required due to broken FusedFuture in oneshot::Receiver
                    // See: https://github.com/rust-lang/futures-rs/issues/2455
                    _ = Timer::sleep(update_interval_ms as u32).fuse() => {
                        if !is_paused {
                            elapsed_time += update_interval_ms;
                            let remaining_percent = 100.0 - (elapsed_time / auto_dismiss_ms * 100.0);
                            let progress = remaining_percent.max(0.0);
                            state_handle.set(progress);

                            if elapsed_time >= auto_dismiss_ms {
                                error_display.toast_dismissed_relay.send(alert_id.clone());
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
                            error_display.toast_dismissed_relay.send(alert_id.clone());
                            break;
                        }
                    }
                    event = action_button_clicked_stream.next() => {
                        if let Some(()) = event {
                            break;
                        }
                    }
                }
            }
        })
    } else {
        Actor::new(100.0 as Progress, async move |_state_handle| {
            loop {
                select! {
                    event = dismiss_button_clicked_stream.next() => {
                        if let Some(()) = event {
                            error_display.toast_dismissed_relay.send(alert_id.clone());
                            break;
                        }
                    }
                    event = action_button_clicked_stream.next() => {
                        if let Some(()) = event {
                            break;
                        }
                    }
                }
            }
        })
    };

    let action_button = if let Some(label) = action_label {
        Some(
            El::new()
                .s(Font::new()
                    .size(FONT_SIZE_14)
                    .weight(FontWeight::SemiBold)
                    .color_signal(colors.title()))
                .s(Cursor::new(CursorIcon::Pointer))
                .s(Padding::new().x(SPACING_12).y(SPACING_4))
                .s(RoundedCorners::all(CORNER_RADIUS_4))
                .s(Background::new().color_signal(colors.border()))
                .child(label)
                .update_raw_el({
                    let action_relay = action_button_clicked_relay.clone();
                    let alert_id = alert_id_for_action.clone();
                    let app_config = app_config.clone();
                    move |raw_el| {
                        raw_el.event_handler(move |event: Click| {
                            event.stop_propagation();
                            handle_notification_action(&alert_id, &app_config);
                            action_relay.send(());
                        })
                    }
                }),
        )
    } else {
        None
    };

    let show_progress_bar = has_auto_dismiss || has_custom_progress;

    Column::new()
        .s(Width::fill())
        .s(Background::new().color_signal(colors.background()))
        .s(Borders::all_signal(
            colors.border().map(|color| Border::new().width(1).color(color)),
        ))
        .s(RoundedCorners::all(CORNER_RADIUS_8))
        .s(Shadows::new(vec![
            Shadow::new().color(hsluv!(0, 0, 0, 10)).x(0).y(2).blur(8),
        ]))
        .s(Cursor::new(CursorIcon::Pointer))
        .update_raw_el(|raw_el| {
            if has_auto_dismiss && !has_custom_progress {
                raw_el.attr("title", "Click to pause/resume auto-dismiss")
            } else {
                raw_el
            }
        })
        .on_click(move || {
            if has_auto_dismiss && !has_custom_progress {
                toast_clicked_relay.send(())
            }
        })
        .item(
            Row::new()
                .s(Width::fill())
                .s(Padding::all(SPACING_12))
                .s(Gap::new().x(SPACING_8))
                .s(Align::new().center_y())
                .item(
                    icon(variant_icon(variant))
                        .size(IconSize::Medium)
                        .color(variant_icon_color(variant))
                        .build(),
                )
                .item(
                    Column::new()
                        .s(Width::fill())
                        .s(Gap::new().y(SPACING_4))
                        .item(
                            El::new()
                                .s(Font::new()
                                    .size(FONT_SIZE_16)
                                    .weight(FontWeight::SemiBold)
                                    .color_signal(colors.title()))
                                .child(&alert.title),
                        )
                        .item(
                            El::new()
                                .s(Font::new()
                                    .size(FONT_SIZE_14)
                                    .color_signal(colors.message())
                                    .wrap_anywhere())
                                .child(&alert.message),
                        ),
                )
                .items(action_button)
                .item(
                    El::new()
                        .s(Font::new().size(FONT_SIZE_14).color_signal(colors.message()))
                        .s(Cursor::new(CursorIcon::Pointer))
                        .s(Padding::all(SPACING_4))
                        .s(RoundedCorners::all(CORNER_RADIUS_4))
                        .child("✕")
                        .update_raw_el(move |raw_el| {
                            raw_el.event_handler(move |event: Click| {
                                event.stop_propagation();
                                dismiss_button_clicked_relay.send(());
                            })
                        }),
                ),
        )
        .items(if show_progress_bar {
            let colors = VariantColors::new(variant);
            Some(
                El::new()
                    .s(Width::fill())
                    .s(Height::exact(3))
                    .s(Background::new().color_signal(colors.progress_bg()))
                    .s(RoundedCorners::new()
                        .bottom_left(CORNER_RADIUS_8)
                        .bottom_right(CORNER_RADIUS_8))
                    .child(
                        El::new()
                            .s(Height::fill())
                            .s(Width::percent_signal(toast_actor.signal()))
                            .s(Background::new().color_signal(colors.progress_fill()))
                            .s(RoundedCorners::new()
                                .bottom_left(CORNER_RADIUS_8)
                                .bottom_right(CORNER_RADIUS_8))
                            .s(Transitions::new([
                                Transition::property("width").duration(150)
                            ]))
                            .update_raw_el(|raw_el| raw_el.style("transform-origin", "left")),
                    ),
            )
        } else {
            None
        })
        .after_remove(move |_| {
            drop(toast_actor);
        })
}

/// Handle notification action button clicks based on alert ID
fn handle_notification_action(alert_id: &str, _app_config: &crate::config::AppConfig) {
    match alert_id {
        "update_available" => {
            crate::platform::request_update_download();
        }
        "update_ready" => {
            crate::platform::request_app_restart();
        }
        _ => {}
    }
}
