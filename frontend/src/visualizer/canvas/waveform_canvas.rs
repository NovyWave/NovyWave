use crate::visualizer::timeline::timeline_actor::set_canvas_dimensions;
use zoon::*;
use super::rendering::WaveformRenderer;

// Re-export all functions from sub-modules for API compatibility
pub use super::animation::*;
pub use super::navigation::*;
pub use super::timeline::*;

// ✅ FIXED: Replaced thread_local! with Atom for local UI state (Actor+Relay architecture)
use std::cell::RefCell;
use std::rc::Rc;

/// Canvas state using Atom for local UI state management
#[derive(Clone)]
pub struct CanvasState {
    /// Renderer instance for Fast2D graphics
    pub renderer: Atom<Option<Rc<RefCell<WaveformRenderer>>>>,
    /// Canvas initialization status
    pub initialized: Atom<bool>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            renderer: Atom::new(None),
            initialized: Atom::new(false),
        }
    }
}

/// Main waveform canvas UI component
pub fn waveform_canvas(
    canvas_state: &CanvasState,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    // Initialize renderer instance first (using Atom instead of thread_local)
    if canvas_state.renderer.get().is_none() {
        canvas_state.renderer.set(Some(Rc::new(RefCell::new(WaveformRenderer::new()))));
    }

    // Start signal listeners for canvas updates
    setup_canvas_signal_listeners(canvas_state, selected_variables, waveform_timeline, app_config);

    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(create_canvas_element(canvas_state, waveform_timeline))
}

/// Canvas element creation with Fast2D integration
fn create_canvas_element(canvas_state: &CanvasState, waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) -> impl Element {
    Canvas::new()
        // ✅ RESPONSIVE: Use reactive dimensions from timeline domain
        .width_signal(waveform_timeline.canvas_width.signal().map(|width| *width as u32))
        .height_signal(waveform_timeline.canvas_height.signal().map(|height| *height as u32))
        .update_raw_el({
            let canvas_state_for_resize = canvas_state.clone();
            move |raw_el| {
                raw_el
                    .on_resize(move |width, height| {
                        if width > 0 && height > 0 {
                            crate::visualizer::timeline::timeline_actor::set_canvas_dimensions_temporary(width as f32, height as f32);

                            // Trigger canvas redraw on resize (using Atom-based state)
                            trigger_canvas_redraw(&canvas_state_for_resize);
                        }
                    })
            }
        })
        .after_insert({
                    let canvas_state_for_insert = canvas_state.clone();
                    move |raw_element| {
                        // Initialize Fast2D rendering with DOM canvas element
                        let canvas_clone = raw_element.clone();
                        let canvas_state_clone = canvas_state_for_insert.clone();
                        Task::start(async move {
                            Task::next_macro_tick().await;

                            let _width = canvas_clone.width();
                            let _height = canvas_clone.height();

                            // Initialize Fast2D canvas with the local renderer (using Atom instead of thread_local)
                            if let Some(renderer_rc) = canvas_state_clone.renderer.get() {
                                let renderer_clone = renderer_rc.clone();
                                let canvas_state_for_task = canvas_state_clone.clone();
                                Task::start(async move {
                                    // Initialize Fast2D canvas asynchronously
                                    let fast2d_canvas =
                                        fast2d::CanvasWrapper::new_with_canvas(canvas_clone)
                                            .await;

                                    // Set canvas on renderer
                                    match renderer_clone.try_borrow_mut() {
                                        Ok(mut renderer) => {
                                            // TODO: In Actor+Relay architecture, get selected_variables from domain parameter
                                            let empty_variables: Vec<shared::SelectedVariable> = vec![];
                                            renderer.set_canvas(fast2d_canvas, &empty_variables);

                                            // ✅ FIX: Mark initialization complete and trigger initial render
                                            canvas_state_for_task.initialized.set(true);
                                        }
                                        Err(_) => {}
                                    }
                                });

                                // NOTE: Initial render will be triggered automatically by set_canvas() method
                            }
                        });
                    }
                })
}

/// Setup signal listeners to trigger canvas redraws when state changes
fn setup_canvas_signal_listeners(
    canvas_state: &CanvasState,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static LISTENERS_SETUP: AtomicBool = AtomicBool::new(false);

    // Only setup listeners once
    if LISTENERS_SETUP.swap(true, Ordering::SeqCst) {
        return;
    }

    // TODO: Listen to timeline state changes when proper viewport signal is available
    // TODO: Listen to cursor position changes when proper signal is available

    // Listen to selected variables changes
    Task::start({
        let selected_variables = selected_variables.clone();
        let canvas_state_for_variables = canvas_state.clone();
        async move {
            selected_variables.variables_vec_actor.signal()
                .for_each_sync(move |_variables| {
                    trigger_canvas_redraw(&canvas_state_for_variables);
                })
                .await;
        }
    });

    // Listen to theme changes
    Task::start({
        let app_config = app_config.clone();
        let canvas_state_for_theme = canvas_state.clone();
        async move {
            app_config
                .theme_actor
                .signal()
                .for_each_sync(move |theme| {
                let novyui_theme = match theme {
                    shared::Theme::Dark => moonzoon_novyui::tokens::theme::Theme::Dark,
                    shared::Theme::Light => moonzoon_novyui::tokens::theme::Theme::Light,
                };
                // Update theme on renderer and trigger redraw (using Atom instead of thread_local)
                if let Some(renderer_rc) = canvas_state_for_theme.renderer.get() {
                    if let Ok(mut renderer) = renderer_rc.try_borrow_mut() {
                        renderer.set_theme(novyui_theme);
                    }
                }
                trigger_canvas_redraw(&canvas_state_for_theme);
            })
            .await;
        }
    });
}

// ✅ FIXED: mark_canvas_initialized and is_canvas_initialized functions eliminated
// Canvas initialization now managed directly through CanvasState.initialized Atom

/// Canvas redraw function using Atom-based state (Actor+Relay architecture)
pub fn trigger_canvas_redraw(canvas_state: &CanvasState) {
    // ✅ FIX: Only attempt renders after canvas is fully initialized (using Atom)
    if !canvas_state.initialized.get() {
        return;
    }

    if let Some(renderer_rc) = canvas_state.renderer.get() {
        match renderer_rc.try_borrow_mut() {
            Ok(mut renderer) => {
                if renderer.has_canvas() {
                    if renderer.needs_redraw() {
                        // TODO: In Actor+Relay architecture, get selected_variables from domain parameter
                        let empty_variables: Vec<shared::SelectedVariable> = vec![];
                        renderer.render_frame(&empty_variables);
                    }
                }
            }
            Err(_) => {}
        }
    }
}
