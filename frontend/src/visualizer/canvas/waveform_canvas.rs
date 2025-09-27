use super::rendering::{RenderingParameters, VariableRenderSnapshot, WaveformRenderer};
use crate::config::AppConfig;
use crate::dataflow::*;
use crate::visualizer::timeline::timeline_actor::{
    TimelinePointerHover, TimelineRenderState, TimelineTooltipData, TooltipVerticalAlignment,
    WaveformTimeline,
};
use futures::{select, stream::StreamExt};
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use moonzoon_novyui::*;
use shared::Theme;
use std::sync::Arc;
use web_sys::{HtmlCanvasElement, HtmlElement};
use zoon::events::{PointerDown, PointerLeave, PointerMove};
use zoon::*;

#[derive(Clone)]
pub struct WaveformCanvas {
    pub canvas_actor: Actor,
    pub canvas_initialized_relay: Relay,
    pub redraw_requested_relay: Relay,
    pub canvas_dimensions_changed_relay: Relay<(f32, f32)>,
    pub theme_changed_relay: Relay<Theme>,
    pub initialization_status_actor: Actor<bool>,
    render_state_store: Mutable<Option<TimelineRenderState>>,
    current_theme: Mutable<Theme>,
    canvas_element_store: Mutable<Option<HtmlCanvasElement>>,
    _render_state_forwarder: Actor<()>,
    _theme_forwarder: Actor<()>,
    _resize_forwarder: Actor<()>,
}

impl WaveformCanvas {
    pub async fn new(waveform_timeline: WaveformTimeline, app_config: AppConfig) -> Self {
        let (canvas_initialized_relay, mut canvas_initialized_stream) = relay();
        let (redraw_requested_relay, mut redraw_stream) = relay();
        let (canvas_dimensions_changed_relay, mut dimensions_stream) = relay::<(f32, f32)>();
        let (theme_changed_relay, mut theme_stream) = relay::<Theme>();
        let (initialization_status_changed_relay, mut initialization_status_stream) = relay();
        let initialization_status_actor = Actor::new(false, async move |state_handle| {
            while let Some(()) = initialization_status_stream.next().await {
                state_handle.set_neq(true);
            }
        });

        let render_state_store = Mutable::new(None);
        let current_theme = Mutable::new(app_config.theme_actor.state.get_cloned());
        let canvas_element_store = Mutable::new(None);

        let (render_state_relay, mut render_state_stream) = relay::<TimelineRenderState>();
        let render_state_forwarder = {
            let timeline = waveform_timeline.clone();
            let relay = render_state_relay.clone();
            Actor::new((), async move |_state| {
                let mut stream = timeline.render_state_actor().signal().to_stream().fuse();
                while let Some(state) = stream.next().await {
                    relay.send(state);
                }
            })
        };

        let theme_forwarder = {
            let relay = theme_changed_relay.clone();
            Actor::new((), async move |_state| {
                let mut stream = app_config.theme_actor.signal().to_stream().fuse();
                while let Some(theme) = stream.next().await {
                    relay.send(theme);
                }
            })
        };

        let resize_forwarder = {
            let mut stream = canvas_dimensions_changed_relay.subscribe().fuse();
            let timeline = waveform_timeline.clone();
            Actor::new((), async move |_state| {
                while let Some((width, height)) = stream.next().await {
                    timeline.canvas_resized_relay.send((width, height));
                }
            })
        };

        let canvas_actor = {
            let render_state_store = render_state_store.clone();
            let current_theme = current_theme.clone();
            let canvas_element_store_actor = canvas_element_store.clone();
            let timeline = waveform_timeline.clone();
            Actor::new((), async move |_state_handle| {
                let mut renderer: Option<WaveformRenderer> = None;
                let mut initialized = false;
                let mut active_theme = current_theme.get_cloned();

                loop {
                    select! {
                        canvas_init = canvas_initialized_stream.next() => {
                            if let Some(()) = canvas_init {
                                if let Some(canvas_element) = canvas_element_store_actor.get_cloned() {
                                    if !initialized {
                                        let mut new_renderer = WaveformRenderer::new().await;
                                        let fast_canvas = fast2d::CanvasWrapper::new_with_canvas(canvas_element)
                                            .await;
                                        new_renderer.set_canvas(fast_canvas);
                                        if let Some(render_state) = render_state_store.get_cloned() {
                                            let params = Self::render_params_from_state(
                                                &render_state,
                                                active_theme,
                                            );
                                            if let Some(duration_ms) = new_renderer.render_frame(params) {
                                                timeline.record_render_duration(duration_ms as f64);
                                            }
                                        }
                                        renderer = Some(new_renderer);
                                        initialized = true;
                                        initialization_status_changed_relay.send(());
                                    } else if let Some(ref mut renderer) = renderer {
                                        let fast_canvas = fast2d::CanvasWrapper::new_with_canvas(canvas_element)
                                            .await;
                                        renderer.set_canvas(fast_canvas);
                                        if let Some(render_state) = render_state_store.get_cloned() {
                                            let params = Self::render_params_from_state(
                                                &render_state,
                                                active_theme,
                                            );
                                            if let Some(duration_ms) = renderer.render_frame(params) {
                                                timeline.record_render_duration(duration_ms as f64);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        redraw_request = redraw_stream.next() => {
                            if let Some(()) = redraw_request {
                                if let (Some(ref mut renderer), Some(render_state)) =
                                    (renderer.as_mut(), render_state_store.get_cloned())
                                {
                                    let params = Self::render_params_from_state(
                                        &render_state,
                                        active_theme,
                                    );
                                    if let Some(duration_ms) = renderer.render_frame(params) {
                                        timeline.record_render_duration(duration_ms as f64);
                                    }
                                }
                            }
                        }
                        dimensions_change = dimensions_stream.next() => {
                            if let Some((width, height)) = dimensions_change {
                                if let (Some(ref mut renderer), Some(render_state)) =
                                    (renderer.as_mut(), render_state_store.get_cloned())
                                {
                                    let params = Self::render_params_from_state(
                                        &render_state,
                                        active_theme,
                                    );
                                    renderer.set_dimensions(width, height);
                                    if let Some(duration_ms) = renderer.render_frame(params) {
                                        timeline.record_render_duration(duration_ms as f64);
                                    }
                                }
                            }
                        }
                        theme_change = theme_stream.next() => {
                            if let Some(theme) = theme_change {
                                active_theme = theme;
                                current_theme.set(theme);
                                if let (Some(ref mut renderer), Some(render_state)) =
                                    (renderer.as_mut(), render_state_store.get_cloned())
                                {
                                    let params = Self::render_params_from_state(
                                        &render_state,
                                        active_theme,
                                    );
                                    renderer.set_theme(Self::map_theme(theme));
                                    if let Some(duration_ms) = renderer.render_frame(params) {
                                        timeline.record_render_duration(duration_ms as f64);
                                    }
                                }
                            }
                        }
                        state_update = render_state_stream.next() => {
                            if let Some(render_state) = state_update {
                                render_state_store.set(Some(render_state.clone()));
                                if let Some(ref mut renderer) = renderer.as_mut() {
                                    let params = Self::render_params_from_state(
                                        &render_state,
                                        active_theme,
                                    );
                                    if let Some(duration_ms) = renderer.render_frame(params) {
                                        timeline.record_render_duration(duration_ms as f64);
                                    }
                                }
                            }
                        }
                    }
                }
            })
        };

        Self {
            canvas_actor,
            canvas_initialized_relay,
            redraw_requested_relay,
            canvas_dimensions_changed_relay,
            theme_changed_relay,
            initialization_status_actor,
            render_state_store,
            current_theme,
            canvas_element_store,
            _render_state_forwarder: render_state_forwarder,
            _theme_forwarder: theme_forwarder,
            _resize_forwarder: resize_forwarder,
        }
    }

    pub fn initialized_signal(&self) -> impl Signal<Item = bool> {
        self.initialization_status_actor.signal()
    }

    fn render_params_from_state(state: &TimelineRenderState, theme: Theme) -> RenderingParameters {
        RenderingParameters {
            canvas_width: state.canvas_width_px,
            canvas_height: state.canvas_height_px,
            viewport_start_ps: state.viewport_start.picoseconds(),
            viewport_end_ps: state.viewport_end.picoseconds(),
            cursor_position_ps: Some(state.cursor.picoseconds()),
            zoom_center_ps: Some(state.zoom_center.picoseconds()),
            theme: Self::map_theme(theme),
            variables: state
                .variables
                .iter()
                .map(|series| VariableRenderSnapshot {
                    unique_id: series.unique_id.clone(),
                    formatter: series.formatter,
                    transitions: Arc::clone(&series.transitions),
                    cursor_value: series.cursor_value.clone(),
                })
                .collect(),
        }
    }

    fn map_theme(theme: Theme) -> NovyUITheme {
        match theme {
            Theme::Dark => NovyUITheme::Dark,
            Theme::Light => NovyUITheme::Light,
        }
    }
}

pub fn waveform_canvas(
    waveform_canvas: &WaveformCanvas,
    waveform_timeline: &WaveformTimeline,
) -> impl Element {
    let canvas_dimensions_relay = waveform_canvas.canvas_dimensions_changed_relay.clone();
    let canvas_initialized_relay = waveform_canvas.canvas_initialized_relay.clone();
    let redraw_relay = waveform_canvas.redraw_requested_relay.clone();
    let theme_relay = waveform_canvas.theme_changed_relay.clone();
    let render_state_store_click = waveform_canvas.render_state_store.clone();
    let render_state_store_move = waveform_canvas.render_state_store.clone();
    let timeline_for_click = waveform_timeline.clone();
    let timeline_for_hover = waveform_timeline.clone();
    let pointer_hover_relay_for_click = waveform_timeline.pointer_hover_relay.clone();
    let pointer_hover_relay_for_move = waveform_timeline.pointer_hover_relay.clone();
    let pointer_hover_relay_for_leave = waveform_timeline.pointer_hover_relay.clone();
    let canvas_element_store = waveform_canvas.canvas_element_store.clone();
    let canvas_element_store_for_insert = canvas_element_store.clone();

    let initial_theme = waveform_canvas.current_theme.get_cloned();
    theme_relay.send(initial_theme);

    let theme_signal_for_tooltip = waveform_canvas.current_theme.signal_cloned();
    let theme_signal_for_hint = waveform_canvas.current_theme.signal_cloned();

    let canvas_element = Canvas::new()
        .width(1)
        .height(1)
        .s(Width::fill())
        .s(Height::fill())
        .update_raw_el({
            let canvas_dimensions_relay = canvas_dimensions_relay.clone();
            let render_state_store_click = render_state_store_click.clone();
            let render_state_store_move = render_state_store_move.clone();
            let timeline_for_click = timeline_for_click.clone();
            let timeline_for_hover = timeline_for_hover.clone();
            move |raw_el| {
                let raw_el = raw_el
                    .on_resize(move |width, height| {
                        if width > 0 && height > 0 {
                            canvas_dimensions_relay.send((width as f32, height as f32));
                        }
                    })
                    .event_handler({
                        let render_state_store_click = render_state_store_click.clone();
                        let timeline_for_click = timeline_for_click.clone();
                        let pointer_hover_relay = pointer_hover_relay_for_click.clone();
                        move |event: PointerDown| {
                            if let Some(state) = render_state_store_click.get_cloned() {
                                let width = state.canvas_width_px.max(1) as f64;
                                let height = state.canvas_height_px.max(1) as f64;
                                let normalized_x =
                                    (event.offset_x() as f64 / width).clamp(0.0, 1.0);
                                let normalized_y =
                                    (event.offset_y() as f64 / height).clamp(0.0, 1.0);
                                let span_ps = state
                                    .viewport_end
                                    .duration_since(state.viewport_start)
                                    .picoseconds();
                                let offset_ps = (span_ps as f64 * normalized_x).round() as u64;
                                let time_ps = state
                                    .viewport_start
                                    .picoseconds()
                                    .saturating_add(offset_ps);
                                let time = crate::visualizer::timeline::time_domain::TimePs::from_picoseconds(
                                    time_ps,
                                );
                                timeline_for_click.cursor_clicked_relay.send(time);
                                pointer_hover_relay.send(Some(TimelinePointerHover {
                                    normalized_x,
                                    normalized_y,
                                }));
                            }
                        }
                    })
                    .event_handler({
                        let render_state_store_move = render_state_store_move.clone();
                        let timeline_for_hover = timeline_for_hover.clone();
                        let pointer_hover_relay = pointer_hover_relay_for_move.clone();
                        move |event: PointerMove| {
                            if let Some(state) = render_state_store_move.get_cloned() {
                                let width = state.canvas_width_px.max(1) as f64;
                                let height = state.canvas_height_px.max(1) as f64;
                                let normalized_x =
                                    (event.offset_x() as f64 / width).clamp(0.0, 1.0);
                                let normalized_y =
                                    (event.offset_y() as f64 / height).clamp(0.0, 1.0);
                                let span_ps = state
                                    .viewport_end
                                    .duration_since(state.viewport_start)
                                    .picoseconds();
                                let offset_ps = (span_ps as f64 * normalized_x).round() as u64;
                                let time_ps = state
                                    .viewport_start
                                    .picoseconds()
                                    .saturating_add(offset_ps);
                                let time = crate::visualizer::timeline::time_domain::TimePs::from_picoseconds(
                                    time_ps,
                                );

                                timeline_for_hover
                                    .zoom_center_follow_mouse_relay
                                    .send(Some(time));
                                pointer_hover_relay.send(Some(TimelinePointerHover {
                                    normalized_x,
                                    normalized_y,
                                }));
                            }
                        }
                    })
                    .event_handler({
                        let timeline_for_hover = timeline_for_hover.clone();
                        let pointer_hover_relay = pointer_hover_relay_for_leave.clone();
                        move |_: PointerLeave| {
                            timeline_for_hover.zoom_center_follow_mouse_relay.send(None);
                            pointer_hover_relay.send(None);
                        }
                    });
                raw_el
            }
        })
        .after_insert({
            let canvas_element_store = canvas_element_store_for_insert.clone();
            move |canvas: HtmlCanvasElement| {
                let width = canvas.client_width() as f32;
                let height = canvas.client_height() as f32;
                if width > 0.0 && height > 0.0 {
                    canvas_dimensions_relay.send((width, height));
                }
                canvas_element_store.set(Some(canvas));
                canvas_initialized_relay.send(());
            }
        })
        .after_remove(move |_| {
            redraw_relay.send(());
        })
        .unify();

    let tooltip_signal = {
        let tooltip_actor = waveform_timeline.tooltip_actor();
        let theme_signal = theme_signal_for_tooltip;
        map_ref! {
            let tooltip = tooltip_actor.state.signal_cloned(),
            let theme = theme_signal => {
                tooltip.clone().map(|data| (data, *theme))
            }
        }
    };

    let tooltip_enabled_signal = waveform_timeline.tooltip_visibility_signal();

    let canvas_element_store_for_tooltip = canvas_element_store.clone();
    let tooltip_layer = El::new()
        .update_raw_el(|raw_el| {
            raw_el
                .style("position", "absolute")
                .style("top", "0")
                .style("left", "0")
                .style("width", "100%")
                .style("height", "100%")
                .style("pointer-events", "none")
        })
        .child_signal({
            let canvas_element_store = canvas_element_store_for_tooltip.clone();
            tooltip_signal.map(move |maybe| {
                maybe.map(|(data, theme)| {
                    let canvas_origin = canvas_element_store.lock_ref().as_ref().map(|canvas| {
                        let rect = canvas.get_bounding_client_rect();
                        (rect.left(), rect.top())
                    });
                    tooltip_view(data, theme, canvas_origin).unify()
                })
            })
        });

    let tooltip_hint_layer = El::new()
        .update_raw_el(|raw_el| {
            raw_el
                .style("position", "absolute")
                .style("top", "12px")
                .style("right", "16px")
                .style("pointer-events", "none")
        })
        .child_signal({
            let theme_signal = theme_signal_for_hint;
            let tooltip_enabled_signal = tooltip_enabled_signal;
            map_ref! {
                let theme = theme_signal,
                let tooltip_enabled = tooltip_enabled_signal => {
                    if *tooltip_enabled {
                        None
                    } else {
                        let (background, border, text_color) = match theme {
                            Theme::Light => (
                                "rgba(15, 23, 42, 0.08)",
                                "rgba(148, 163, 184, 0.45)",
                                "#0f172a",
                            ),
                            Theme::Dark => (
                                "rgba(15, 23, 42, 0.75)",
                                "rgba(148, 163, 184, 0.35)",
                                "#f8fafc",
                            ),
                        };

                        Some(
                            El::new()
                                .s(Padding::new().x(10).y(6))
                                .s(RoundedCorners::all(6))
                                .s(Font::new().size(11))
                                .update_raw_el(|raw_el| {
                                    raw_el
                                        .style("background", background)
                                        .style("border", &format!("1px solid {}", border))
                                        .style("color", text_color)
                                })
                                .child("Tooltips hidden - press T to show")
                                .unify(),
                        )
                    }
                }
            }
        });

    Stack::new()
        .s(Width::fill())
        .s(Height::fill())
        .layer(canvas_element)
        .layer(tooltip_layer)
        .layer(tooltip_hint_layer)
}

fn tooltip_view(
    data: TimelineTooltipData,
    theme: Theme,
    canvas_origin: Option<(f64, f64)>,
) -> impl Element {
    let (background, border_color, primary_text, secondary_text) = match theme {
        Theme::Light => (
            "rgba(255, 255, 255, 0.97)",
            "rgba(148, 163, 184, 0.35)",
            "#0f172a",
            "rgba(71, 85, 105, 0.8)",
        ),
        Theme::Dark => (
            "rgba(15, 23, 42, 0.92)",
            "rgba(148, 163, 184, 0.3)",
            "#f8fafc",
            "rgba(203, 213, 225, 0.7)",
        ),
    };

    let educational = data.educational_message.clone();

    let mut content = Column::new()
        .s(Gap::new().y(4))
        .item(
            El::new()
                .s(Font::new().size(12).weight(FontWeight::SemiBold))
                .child(data.variable_label.clone()),
        )
        .item(
            El::new()
                .s(Font::new().size(11))
                .update_raw_el(|raw_el| raw_el.style("color", secondary_text))
                .child(data.time_label.clone()),
        )
        .item(
            El::new()
                .s(Font::new().size(12).weight(FontWeight::Medium))
                .child(data.value_label.clone()),
        );

    if let Some(message) = educational {
        let educational_block = Column::new()
            .s(Gap::new().y(2))
            .s(Padding::new().top(4))
            .items(message.lines().map(|line| {
                El::new()
                    .s(Font::new().size(10))
                    .update_raw_el(|raw_el| raw_el.style("color", secondary_text))
                    .child(line)
            }));
        content = content.item(educational_block);
    }

    content = content.item(
        El::new()
            .s(Padding::new().top(4))
            .s(Font::new().size(10))
            .update_raw_el(|raw_el| raw_el.style("color", secondary_text))
            .child("Press T to hide tooltip"),
    );

    let (origin_x, origin_y) = canvas_origin.unwrap_or((0.0, 0.0));
    let anchor_x = origin_x + data.screen_x as f64;
    let anchor_y = origin_y + data.screen_y as f64;
    let preferred_alignment = data.vertical_alignment;

    El::new()
        .update_raw_el(move |raw_el| {
            raw_el
                .style("position", "fixed")
                .style("left", "0px")
                .style("top", "0px")
                .style("min-width", "160px")
                .style("max-width", "260px")
                .style("padding", "8px 12px")
                .style("border-radius", "8px")
                .style("background", background)
                .style("border", &format!("1px solid {}", border_color))
                .style("box-shadow", "0 10px 30px rgba(15, 23, 42, 0.35)")
                .style("backdrop-filter", "blur(8px)")
                .style("color", primary_text)
                .style("pointer-events", "none")
                .style("z-index", "15000")
        })
        .after_insert(move |element: HtmlElement| {
            const POINTER_GAP: f64 = 12.0;
            const VIEWPORT_MARGIN: f64 = 8.0;

            let window = match web_sys::window() {
                Some(window) => window,
                None => return,
            };

            let viewport_width = window
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1024.0);
            let viewport_height = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(768.0);

            let rect = element.get_bounding_client_rect();
            let width = rect.width().max(1.0);
            let height = rect.height().max(1.0);

            let mut alignment = preferred_alignment;
            let mut top = match alignment {
                TooltipVerticalAlignment::Above => anchor_y - height - POINTER_GAP,
                TooltipVerticalAlignment::Below => anchor_y + POINTER_GAP,
            };

            if alignment == TooltipVerticalAlignment::Above && top < VIEWPORT_MARGIN {
                alignment = TooltipVerticalAlignment::Below;
                top = anchor_y + POINTER_GAP;
            }

            if alignment == TooltipVerticalAlignment::Below
                && top + height > viewport_height - VIEWPORT_MARGIN
            {
                alignment = TooltipVerticalAlignment::Above;
                top = anchor_y - height - POINTER_GAP;
            }

            if top < VIEWPORT_MARGIN {
                top = VIEWPORT_MARGIN;
            }
            if top + height > viewport_height - VIEWPORT_MARGIN {
                top = (viewport_height - height - VIEWPORT_MARGIN).max(VIEWPORT_MARGIN);
            }

            let mut left = anchor_x - width / 2.0;
            if left + width > viewport_width - VIEWPORT_MARGIN {
                left = (viewport_width - VIEWPORT_MARGIN - width).max(VIEWPORT_MARGIN);
            }
            if left < VIEWPORT_MARGIN {
                left = VIEWPORT_MARGIN;
            }

            let style = element.style();
            let _ = style.set_property("top", &format!("{:.1}px", top));
            let _ = style.set_property("left", &format!("{:.1}px", left));
        })
        .child(content)
}
