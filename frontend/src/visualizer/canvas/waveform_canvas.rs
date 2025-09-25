use super::rendering::{RenderingParameters, VariableRenderSnapshot, WaveformRenderer};
use crate::config::AppConfig;
use crate::dataflow::*;
use crate::visualizer::timeline::timeline_actor::{TimelineRenderState, WaveformTimeline};
use futures::{select, stream::StreamExt};
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::{SignalValue, Theme};
use web_sys::HtmlCanvasElement;
use zoon::events::{PointerDown, PointerLeave, PointerMove};
use zoon::*;

fn special_state_tooltip(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_uppercase().as_str() {
        "Z" => Some(
            "High-Impedance (Z)\nSignal is disconnected or floating.\nCommon in tri-state buses and disabled outputs.",
        ),
        "X" => Some(
            "Unknown (X)\nSignal value cannot be determined.\nOften caused by timing violations or uninitialized logic.",
        ),
        "U" => Some(
            "Uninitialized (U)\nSignal has not been assigned a value.\nTypically seen during power-up or before reset.",
        ),
        _ => None,
    }
}

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
                                            new_renderer.render_frame(params);
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
                                            renderer.render_frame(params);
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
                                    renderer.render_frame(params);
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
                                    renderer.render_frame(params);
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
                                    renderer.render_frame(params);
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
                                    renderer.render_frame(params);
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
            viewport_start_ns: state.viewport_start.nanos(),
            viewport_end_ns: state.viewport_end.nanos(),
            cursor_position_ns: Some(state.cursor.nanos()),
            zoom_center_ns: Some(state.zoom_center.nanos()),
            theme: Self::map_theme(theme),
            variables: state
                .variables
                .iter()
                .map(|series| VariableRenderSnapshot {
                    unique_id: series.unique_id.clone(),
                    formatter: series.formatter,
                    transitions: series.transitions.clone(),
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
    let canvas_element_store = waveform_canvas.canvas_element_store.clone();
    let canvas_element_store_for_handlers = canvas_element_store.clone();

    let initial_theme = waveform_canvas.current_theme.get_cloned();
    theme_relay.send(initial_theme);

    Canvas::new()
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
            let canvas_element_store_for_handlers = canvas_element_store_for_handlers.clone();
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
                        move |event: PointerDown| {
                            if let Some(state) = render_state_store_click.get_cloned() {
                                let width = state.canvas_width_px.max(1) as f32;
                                let normalized = (event.offset_x() as f32 / width).clamp(0.0, 1.0);
                                let span = state
                                    .viewport_end
                                    .duration_since(state.viewport_start)
                                    .nanos();
                                let offset = (span as f32 * normalized).round() as u64;
                                let time =
                                    crate::visualizer::timeline::time_domain::TimeNs::from_nanos(
                                        state.viewport_start.nanos().saturating_add(offset),
                                    );
                                timeline_for_click.cursor_clicked_relay.send(time);
                            }
                        }
                    })
                    .event_handler({
                        let render_state_store_move = render_state_store_move.clone();
                        let timeline_for_hover = timeline_for_hover.clone();
                        let canvas_element_store_move = canvas_element_store_for_handlers.clone();
                        move |event: PointerMove| {
                            if let Some(state) = render_state_store_move.get_cloned() {
                                let width = state.canvas_width_px.max(1) as f32;
                                let normalized = (event.offset_x() as f32 / width).clamp(0.0, 1.0);
                                let span = state
                                    .viewport_end
                                    .duration_since(state.viewport_start)
                                    .nanos();
                                let offset = (span as f32 * normalized).round() as u64;
                                let time_ns = state.viewport_start.nanos().saturating_add(offset);
                                let time =
                                    crate::visualizer::timeline::time_domain::TimeNs::from_nanos(
                                        time_ns,
                                    );

                                timeline_for_hover
                                    .zoom_center_follow_mouse_relay
                                    .send(Some(time));

                                if let Some(canvas_el) = canvas_element_store_move.get_cloned() {
                                    let total_rows = (state.variables.len() + 1).max(1) as f32;
                                    let row_height =
                                        (state.canvas_height_px.max(1) as f32) / total_rows;
                                    let pointer_row =
                                        (event.offset_y() as f32 / row_height).floor().max(0.0)
                                            as usize;

                                    if pointer_row < state.variables.len() {
                                        let variable = &state.variables[pointer_row];
                                        let mut current_value: Option<&str> = None;
                                        for transition in &variable.transitions {
                                            if transition.time_ns > time_ns {
                                                break;
                                            }
                                            current_value = Some(transition.value.as_str());
                                        }

                                        if current_value.is_none() {
                                            if let Some(SignalValue::Present(raw)) =
                                                variable.cursor_value.as_ref()
                                            {
                                                current_value = Some(raw.as_str());
                                            }
                                        }

                                        if let Some(value) = current_value {
                                            if let Some(tooltip) = special_state_tooltip(value) {
                                                let _ = canvas_el.set_attribute("title", tooltip);
                                            } else {
                                                let _ = canvas_el.remove_attribute("title");
                                            }
                                        } else {
                                            let _ = canvas_el.remove_attribute("title");
                                        }
                                    } else {
                                        let _ = canvas_el.remove_attribute("title");
                                    }
                                }
                            }
                        }
                    })
                    .event_handler({
                        let timeline_for_hover = timeline_for_hover.clone();
                        let canvas_element_store_leave = canvas_element_store_for_handlers.clone();
                        move |_: PointerLeave| {
                            timeline_for_hover.zoom_center_follow_mouse_relay.send(None);
                            if let Some(canvas_el) = canvas_element_store_leave.get_cloned() {
                                let _ = canvas_el.remove_attribute("title");
                            }
                        }
                    });
                raw_el
            }
        })
        .after_insert(move |canvas: HtmlCanvasElement| {
            let width = canvas.client_width() as f32;
            let height = canvas.client_height() as f32;
            if width > 0.0 && height > 0.0 {
                canvas_dimensions_relay.send((width, height));
            }
            canvas_element_store.set(Some(canvas));
            canvas_initialized_relay.send(());
        })
        .after_remove(move |_| {
            redraw_relay.send(());
        })
}
