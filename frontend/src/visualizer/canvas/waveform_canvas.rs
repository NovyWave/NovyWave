use super::rendering::{RenderingParameters, VariableRenderSnapshot, WaveformRenderer};
use crate::config::AppConfig;
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
    _canvas_task: Arc<TaskHandle>,
    pub initialization_status: Mutable<bool>,
    render_state_store: Mutable<Option<TimelineRenderState>>,
    current_theme: Mutable<Theme>,
    canvas_element_store: Mutable<Option<HtmlCanvasElement>>,
    canvas_dimensions: Mutable<(f32, f32)>,
    canvas_ready: Mutable<bool>,
}

impl WaveformCanvas {
    pub async fn new(waveform_timeline: WaveformTimeline, app_config: AppConfig) -> Self {
        let initialization_status = Mutable::new(false);
        let render_state_store = Mutable::new(None);
        let current_theme = Mutable::new(app_config.theme.get_cloned());
        let canvas_element_store: Mutable<Option<HtmlCanvasElement>> = Mutable::new(None);
        let canvas_dimensions = Mutable::new((0.0f32, 0.0f32));
        let canvas_ready = Mutable::new(false);

        let canvas_task = Arc::new(Task::start_droppable({
            let render_state_store = render_state_store.clone();
            let current_theme_store = current_theme.clone();
            let canvas_element_store_task = canvas_element_store.clone();
            let canvas_dimensions_task = canvas_dimensions.clone();
            let canvas_ready_task = canvas_ready.clone();
            let initialization_status_task = initialization_status.clone();
            let timeline = waveform_timeline.clone();
            let app_config_task = app_config.clone();
            async move {
                let mut renderer: Option<WaveformRenderer> = None;
                let mut initialized = false;
                let mut active_theme = current_theme_store.get_cloned();
                let mut cached_dimensions = (0.0f32, 0.0f32);

                let mut render_state_stream = timeline.render_state_actor().signal_cloned().to_stream().fuse();
                let mut theme_stream = app_config_task.theme.signal().to_stream().fuse();
                let mut dimensions_stream = canvas_dimensions_task.signal().dedupe().to_stream().fuse();
                let mut canvas_ready_stream = canvas_ready_task.signal().dedupe().to_stream().fuse();

                loop {
                    select! {
                        canvas_is_ready = canvas_ready_stream.next() => {
                            if let Some(true) = canvas_is_ready {
                                if let Some(canvas_element) = canvas_element_store_task.get_cloned() {
                                    if !initialized {
                                        let mut new_renderer = WaveformRenderer::new();
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
                                        initialization_status_task.set_neq(true);
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
                        dimensions_change = dimensions_stream.next() => {
                            if let Some((width, height)) = dimensions_change {
                                if (width, height) != cached_dimensions && width > 0.0 && height > 0.0 {
                                    cached_dimensions = (width, height);
                                    timeline.set_canvas_dimensions(width, height);
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
                        }
                        theme_change = theme_stream.next() => {
                            if let Some(theme) = theme_change {
                                active_theme = theme;
                                current_theme_store.set(theme);
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
            }
        }));

        Self {
            _canvas_task: canvas_task,
            initialization_status,
            render_state_store,
            current_theme,
            canvas_element_store,
            canvas_dimensions,
            canvas_ready,
        }
    }

    pub fn initialized_signal(&self) -> impl Signal<Item = bool> {
        self.initialization_status.signal()
    }

    pub fn notify_dimensions(&self, width: f32, height: f32) {
        self.canvas_dimensions.set_neq((width, height));
    }

    pub fn notify_canvas_ready(&self) {
        self.canvas_ready.set_neq(true);
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
    let canvas_ref = waveform_canvas.clone();
    let canvas_ref_for_resize = waveform_canvas.clone();
    let render_state_store_click = waveform_canvas.render_state_store.clone();
    let render_state_store_move = waveform_canvas.render_state_store.clone();
    let timeline_for_click = waveform_timeline.clone();
    let timeline_for_hover = waveform_timeline.clone();
    let timeline_for_click_hover = waveform_timeline.clone();
    let timeline_for_move_hover = waveform_timeline.clone();
    let timeline_for_leave = waveform_timeline.clone();
    let canvas_element_store = waveform_canvas.canvas_element_store.clone();
    let canvas_element_store_for_insert = canvas_element_store.clone();

    let theme_signal_for_tooltip = waveform_canvas.current_theme.signal_cloned();

    let canvas_element = Canvas::new()
        .width(1)
        .height(1)
        .s(Width::fill())
        .s(Height::fill())
        .update_raw_el({
            let render_state_store_click = render_state_store_click.clone();
            let render_state_store_move = render_state_store_move.clone();
            let timeline_for_click = timeline_for_click.clone();
            let timeline_for_hover = timeline_for_hover.clone();
            move |raw_el| {
                let raw_el = raw_el
                    .on_resize({
                        let canvas_ref = canvas_ref_for_resize.clone();
                        move |width, height| {
                            if width > 0 && height > 0 {
                                canvas_ref.notify_dimensions(width as f32, height as f32);
                            }
                        }
                    })
                    .event_handler({
                        let render_state_store_click = render_state_store_click.clone();
                        let timeline_for_click = timeline_for_click.clone();
                        let timeline_for_hover = timeline_for_click_hover.clone();
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
                                timeline_for_click.set_cursor_clamped(time);
                                timeline_for_hover.set_pointer_hover(Some(TimelinePointerHover {
                                    normalized_x,
                                    normalized_y,
                                }));
                            }
                        }
                    })
                    .event_handler({
                        let render_state_store_move = render_state_store_move.clone();
                        let timeline_for_hover = timeline_for_move_hover.clone();
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

                                timeline_for_hover.set_zoom_center_follow(Some(time));
                                timeline_for_hover.set_pointer_hover(Some(TimelinePointerHover {
                                    normalized_x,
                                    normalized_y,
                                }));
                            }
                        }
                    })
                    .event_handler({
                        let timeline_for_leave = timeline_for_leave.clone();
                        move |_: PointerLeave| {
                            timeline_for_leave.set_zoom_center_follow(None);
                            timeline_for_leave.set_pointer_hover(None);
                        }
                    });
                raw_el
            }
        })
        .after_insert({
            let canvas_element_store = canvas_element_store_for_insert.clone();
            let canvas_ref = canvas_ref.clone();
            move |canvas: HtmlCanvasElement| {
                let width = canvas.client_width() as f32;
                let height = canvas.client_height() as f32;
                if width > 0.0 && height > 0.0 {
                    canvas_ref.notify_dimensions(width, height);
                }
                canvas_element_store.set(Some(canvas));
                canvas_ref.notify_canvas_ready();
            }
        })
        .unify();

    let tooltip_signal = {
        let tooltip_mutable = waveform_timeline.tooltip_actor();
        let theme_signal = theme_signal_for_tooltip;
        map_ref! {
            let tooltip = tooltip_mutable.signal_cloned(),
            let theme = theme_signal => {
                tooltip.clone().map(|data| (data, *theme))
            }
        }
    };

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

    Stack::new()
        .s(Width::fill())
        .s(Height::fill())
        .layer(canvas_element)
        .layer(tooltip_layer)
}

fn tooltip_view(
    data: TimelineTooltipData,
    theme: Theme,
    canvas_origin: Option<(f64, f64)>,
) -> impl Element {
    let (background, border_color, primary_text, secondary_text) = match theme {
        Theme::Light => (
            "rgba(255, 255, 255, 0.9)",
            "rgba(148, 163, 184, 0.35)",
            "#0f172a",
            "rgba(71, 85, 105, 0.8)",
        ),
        Theme::Dark => (
            "rgba(15, 23, 42, 0.85)",
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
                .update_raw_el(|raw_el| {
                    raw_el
                        .style("white-space", "normal")
                        .style("overflow-wrap", "anywhere")
                })
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

            let mut top = match preferred_alignment {
                TooltipVerticalAlignment::Above => {
                    let candidate = anchor_y - height - POINTER_GAP;
                    if candidate < VIEWPORT_MARGIN {
                        anchor_y + POINTER_GAP
                    } else {
                        candidate
                    }
                }
                TooltipVerticalAlignment::Below => {
                    let candidate = anchor_y + POINTER_GAP;
                    if candidate + height > viewport_height - VIEWPORT_MARGIN {
                        anchor_y - height - POINTER_GAP
                    } else {
                        candidate
                    }
                }
            };

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
