use crate::error_display::{ErrorAlert, NotificationVariant};
use moonzoon_novyui::components::icon::{IconColor, IconName, IconSize, icon};
use moonzoon_novyui::tokens::*;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use zoon::events::Click;
use zoon::*;

fn variant_icon(variant: NotificationVariant) -> IconName {
    match variant {
        NotificationVariant::Error => IconName::TriangleAlert,
        NotificationVariant::Info => IconName::Info,
        NotificationVariant::Success => IconName::CircleCheck,
    }
}

fn variant_icon_color(variant: NotificationVariant) -> IconColor {
    match variant {
        NotificationVariant::Error => IconColor::Error,
        NotificationVariant::Info => IconColor::Primary,
        NotificationVariant::Success => IconColor::Success,
    }
}

#[derive(Clone, Copy)]
struct VariantColors {
    variant: NotificationVariant,
}

impl VariantColors {
    fn new(variant: NotificationVariant) -> Self {
        Self { variant }
    }

    fn background(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(12% 0.03 30)",
            NotificationVariant::Info => "oklch(20% 0.01 250)",
            NotificationVariant::Success => "oklch(12% 0.03 145)",
        })
    }

    fn border(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(70% 0.21 30)",
            NotificationVariant::Info => "oklch(65% 0.16 250)",
            NotificationVariant::Success => "oklch(70% 0.15 145)",
        })
    }

    fn title(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(88% 0.18 30)",
            NotificationVariant::Info => "oklch(85% 0.14 250)",
            NotificationVariant::Success => "oklch(88% 0.13 145)",
        })
    }

    fn message(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(80% 0.21 30)",
            NotificationVariant::Info => "oklch(75% 0.16 250)",
            NotificationVariant::Success => "oklch(80% 0.15 145)",
        })
    }

    fn progress_bg(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(30% 0.09 30)",
            NotificationVariant::Info => "oklch(30% 0.05 250)",
            NotificationVariant::Success => "oklch(30% 0.07 145)",
        })
    }

    fn progress_fill(&self) -> impl Signal<Item = &'static str> + 'static {
        let v = self.variant;
        signal::always(()).map(move |_| match v {
            NotificationVariant::Error => "oklch(70% 0.21 30)",
            NotificationVariant::Info => "oklch(65% 0.16 250)",
            NotificationVariant::Success => "oklch(70% 0.15 145)",
        })
    }
}

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
                .style("pointer-events", "none")
                .style("z-index", "1000")
        })
        .child(
            Column::new()
                .s(Gap::new().y(SPACING_8))
                .s(Width::exact(400))
                .s(Align::new().top().right())
                .update_raw_el(|raw_el| {
                    raw_el.style("pointer-events", "auto")
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
    let auto_dismiss_ms = alert.auto_dismiss_ms as f32;
    let has_auto_dismiss = auto_dismiss_ms > 0.0;
    let has_custom_progress = alert.progress.is_some();
    let custom_progress = alert.progress.unwrap_or(0.0);
    let variant = alert.variant;
    let action_label = alert.action_label.clone();

    let error_display = app_config.error_display.clone();
    let alert_id = alert.id.clone();
    let alert_id_for_action = alert.id.clone();
    let alert_id_for_dismiss = alert.id.clone();
    let alert_id_for_click = alert.id.clone();

    let colors = VariantColors::new(variant);

    let is_paused = Rc::new(Cell::new(false));
    let toast_progress = Mutable::new(if has_custom_progress { custom_progress } else { 100.0 });

    let _toast_task: Arc<TaskHandle> = if has_auto_dismiss && !has_custom_progress {
        let progress = toast_progress.clone();
        let error_display = error_display.clone();
        let alert_id = alert_id.clone();
        let is_paused = is_paused.clone();
        Arc::new(Task::start_droppable(async move {
            let mut elapsed_time = 0.0f32;
            let update_interval_ms = 50.0f32;
            loop {
                Timer::sleep(update_interval_ms as u32).await;
                if !is_paused.get() {
                    elapsed_time += update_interval_ms;
                    let remaining_percent = 100.0 - (elapsed_time / auto_dismiss_ms * 100.0);
                    progress.set(remaining_percent.max(0.0));
                    if elapsed_time >= auto_dismiss_ms {
                        error_display.dismiss_toast(&alert_id);
                        break;
                    }
                }
            }
        }))
    } else {
        Arc::new(Task::start_droppable(async {}))
    };

    let action_button = if let Some(label) = action_label {
        let error_display_action = error_display.clone();
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
                    let alert_id = alert_id_for_action.clone();
                    let app_config = app_config.clone();
                    move |raw_el| {
                        raw_el.event_handler(move |event: Click| {
                            event.stop_propagation();
                            handle_notification_action(&alert_id, &app_config);
                            error_display_action.dismiss_toast(&alert_id);
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
        .update_raw_el({
            let is_paused = is_paused.clone();
            move |raw_el| {
                if has_auto_dismiss && !has_custom_progress {
                    raw_el.event_handler(move |_: Click| {
                        is_paused.set(!is_paused.get());
                    })
                } else {
                    raw_el
                }
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
                .item({
                    let error_display = error_display.clone();
                    El::new()
                        .s(Font::new().size(FONT_SIZE_14).color_signal(colors.message()))
                        .s(Cursor::new(CursorIcon::Pointer))
                        .s(Padding::all(SPACING_4))
                        .s(RoundedCorners::all(CORNER_RADIUS_4))
                        .child("✕")
                        .update_raw_el(move |raw_el| {
                            raw_el.event_handler(move |event: Click| {
                                event.stop_propagation();
                                error_display.dismiss_toast(&alert_id_for_dismiss);
                            })
                        })
                }),
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
                            .s(Width::percent_signal(toast_progress.signal()))
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
            drop(_toast_task);
        })
}

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
