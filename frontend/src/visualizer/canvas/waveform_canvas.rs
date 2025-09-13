// use crate::visualizer::timeline::timeline_actor::set_canvas_dimensions; // Function does not exist
use zoon::*;
use super::rendering::WaveformRenderer;
use crate::dataflow::*;
use futures::{select, stream::StreamExt};

pub use super::animation::*;
pub use super::navigation::*;
pub use super::timeline::*;

/// Canvas domain - manages waveform rendering and canvas state
#[derive(Clone)]
pub struct WaveformCanvas {
    pub canvas_actor: Actor,
    pub canvas_initialized_relay: Relay,
    pub redraw_requested_relay: Relay,
    pub canvas_dimensions_changed_relay: Relay<(f32, f32)>,
    pub theme_changed_relay: Relay<shared::Theme>,
    pub initialization_status_actor: Actor<bool>,
}

impl WaveformCanvas {
    pub async fn new() -> Self {
        let (canvas_initialized_relay, mut canvas_initialized_stream) = relay();
        let (redraw_requested_relay, mut redraw_stream) = relay();
        let (canvas_dimensions_changed_relay, mut dimensions_stream) = relay::<(f32, f32)>();
        let (theme_changed_relay, mut theme_stream) = relay::<shared::Theme>();
        let (initialization_status_changed_relay, mut initialization_status_stream) = relay();
        let initialization_status_actor = Actor::new(false, async move |state_handle| {
            while let Some(()) = initialization_status_stream.next().await {
                state_handle.set_neq(true); // Always set to true when event received
            }
        });
        
        let canvas_actor = Actor::new((), {
            let initialization_status_changed_relay = initialization_status_changed_relay.clone();
            async move |_state_handle| {
            // State as local variables - no Clone requirement
            let mut renderer: Option<WaveformRenderer> = None;
            let mut initialized = false;
            let mut current_theme = shared::Theme::default();
            let mut canvas_dimensions = (0.0, 0.0);
            
            loop {
                select! {
                    canvas_init = canvas_initialized_stream.next() => {
                        if let Some(()) = canvas_init {
                            if !initialized {
                                let mut new_renderer = WaveformRenderer::new().await;
                                let novyui_theme = match current_theme {
                                    shared::Theme::Dark => moonzoon_novyui::tokens::theme::Theme::Dark,
                                    shared::Theme::Light => moonzoon_novyui::tokens::theme::Theme::Light,
                                };
                                // Theme will be passed through render parameters
                                renderer = Some(new_renderer);
                                initialized = true;
                                initialization_status_changed_relay.send(());
                            }
                        }
                    }
                    redraw_request = redraw_stream.next() => {
                        if let Some(()) = redraw_request {
                            if let Some(ref mut renderer) = renderer {
                                if renderer.has_canvas() {
                                    // Create basic render parameters - will be updated later with real data
                                    let render_params = super::rendering::RenderingParameters {
                                        canvas_width: canvas_dimensions.0 as u32,
                                        canvas_height: canvas_dimensions.1 as u32,
                                        viewport_start: 0.0,
                                        viewport_end: 1000.0,
                                        cursor_position: None,
                                        zoom_center_position: None,
                                        theme: match current_theme {
                                            shared::Theme::Dark => moonzoon_novyui::tokens::theme::Theme::Dark,
                                            shared::Theme::Light => moonzoon_novyui::tokens::theme::Theme::Light,
                                        },
                                        selected_variables: Vec::new(),
                                    };
                                    renderer.render_frame(render_params);
                                }
                            }
                        }
                    }
                    dimensions_change = dimensions_stream.next() => {
                        if let Some((width, height)) = dimensions_change {
                            canvas_dimensions = (width, height);
                            // Dimensions will be passed through render parameters
                        }
                    }
                    theme_change = theme_stream.next() => {
                        if let Some(theme) = theme_change {
                            current_theme = theme;
                            // Theme will be passed through render parameters
                        }
                    }
                }
            }
        }
        });
        
        Self {
            canvas_actor,
            canvas_initialized_relay,
            redraw_requested_relay,
            canvas_dimensions_changed_relay,
            theme_changed_relay,
            initialization_status_actor,
        }
    }
    
    /// Signal indicating if canvas is initialized and ready for rendering
    pub fn initialized_signal(&self) -> impl Signal<Item = bool> {
        self.initialization_status_actor.signal()
    }
}

pub fn waveform_canvas(
    waveform_canvas: &WaveformCanvas,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    // Set up event connections between domains
    setup_canvas_event_connections(waveform_canvas, selected_variables, waveform_timeline, app_config);

    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(create_canvas_element(waveform_canvas, waveform_timeline))
}

fn create_canvas_element(waveform_canvas: &WaveformCanvas, _waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) -> impl Element {
    // Proper Canvas construction with required flags
    Canvas::new()
        .width(800)  // This should set WidthFlag
        .height(600) // This should set HeightFlag
        .update_raw_el({
            let dimensions_relay = waveform_canvas.canvas_dimensions_changed_relay.clone();
            move |raw_el| {
                raw_el
                    .on_resize(move |width, height| {
                        if width > 0 && height > 0 {
                            dimensions_relay.send((width as f32, height as f32));
                        }
                    })
            }
        })
        .after_insert({
            let canvas_initialized_relay = waveform_canvas.canvas_initialized_relay.clone();
            move |_raw_element| {
                canvas_initialized_relay.send(());
            }
        })
}

fn setup_canvas_event_connections(
    waveform_canvas: &WaveformCanvas,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) {
    // Connect domain events to canvas events using relay patterns
    let _event_connector_actor = Actor::new(false, {
        let selected_variables = selected_variables.clone();
        let _waveform_timeline = waveform_timeline.clone();
        let app_config = app_config.clone();
        let waveform_canvas = waveform_canvas.clone();
        async move |state_handle| {
            if !state_handle.get() {
                let mut variables_stream = selected_variables.variables_vec_actor.signal().to_stream().fuse();
                let mut theme_stream = app_config.theme_actor.signal().to_stream().fuse();
                
                state_handle.set(true);
                
                loop {
                    select! {
                        _ = variables_stream.next() => {
                            // Variables changed - trigger redraw
                            waveform_canvas.redraw_requested_relay.send(());
                        }
                        theme_change = theme_stream.next() => {
                            if let Some(theme) = theme_change {
                                // Theme changed - update canvas theme and redraw
                                waveform_canvas.theme_changed_relay.send(theme);
                                waveform_canvas.redraw_requested_relay.send(());
                            }
                        }
                    }
                }
            }
        }
    });
    
    // Connect canvas dimension changes to timeline
    let _dimension_connector_actor = Actor::new(false, {
        let waveform_canvas = waveform_canvas.clone();
        let waveform_timeline = waveform_timeline.clone();
        async move |state_handle| {
            if !state_handle.get() {
                let mut dimensions_stream = waveform_canvas.canvas_dimensions_changed_relay.subscribe();
                
                state_handle.set(true);
                
                loop {
                    select! {
                        dimensions_change = dimensions_stream.next() => {
                            if let Some((width, height)) = dimensions_change {
                                // Forward canvas dimensions to timeline
                                waveform_timeline.canvas_resized_relay.send((width, height));
                                waveform_canvas.redraw_requested_relay.send(());
                            }
                        }
                    }
                }
            }
        }
    });
}

