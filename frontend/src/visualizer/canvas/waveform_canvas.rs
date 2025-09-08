use crate::visualizer::timeline::timeline_actor::set_canvas_dimensions;
use zoon::*;
use super::rendering::WaveformRenderer;

// Re-export all functions from sub-modules for API compatibility
pub use super::animation::*;
pub use super::navigation::*;
pub use super::timeline::*;

// WASM-safe local renderer instance using RefCell
use std::cell::RefCell;
use std::rc::Rc;
thread_local! {
    static RENDERER: RefCell<Option<Rc<RefCell<WaveformRenderer>>>> = RefCell::new(None);
    static CANVAS_INITIALIZED: RefCell<bool> = RefCell::new(false);
}

/// Main waveform canvas UI component
pub fn waveform_canvas(
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    // Initialize renderer instance first
    RENDERER.with(|r| {
        if r.borrow().is_none() {
            *r.borrow_mut() = Some(Rc::new(RefCell::new(WaveformRenderer::new())));
        }
    });

    // Start signal listeners for canvas updates
    setup_canvas_signal_listeners(selected_variables, waveform_timeline, app_config);

    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(create_canvas_element())
}

/// Canvas element creation with Fast2D integration
fn create_canvas_element() -> impl Element {
    Canvas::new()
        // ✅ RESPONSIVE: Use default dimensions, actual size managed by resize handler
        .width(800) // Default width, will be updated by on_resize handler
        .height(600) // Default height, will be updated by on_resize handler
        .update_raw_el(move |raw_el| {
            raw_el
                .on_resize(move |width, height| {
                    if width > 0 && height > 0 {
                        crate::visualizer::timeline::timeline_actor::set_canvas_dimensions_temporary(width as f32, height as f32);

                        // Trigger canvas redraw on resize
                        trigger_canvas_redraw_global();
                    }
                })
                .after_insert({
                    move |raw_element| {
                        // Initialize Fast2D rendering with DOM canvas element
                        let canvas_clone = raw_element.clone();
                        Task::start(async move {
                            Task::next_macro_tick().await;

                            let _width = canvas_clone.width();
                            let _height = canvas_clone.height();

                            // Initialize Fast2D canvas with the local renderer
                            RENDERER.with(|r| {
                                if let Some(renderer_rc) = r.borrow().as_ref() {
                                    let renderer_clone = renderer_rc.clone();
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
                                                mark_canvas_initialized();
                                            }
                                            Err(_) => {}
                                        }
                                    });

                                    // NOTE: Initial render will be triggered automatically by set_canvas() method
                                } else {
                                }
                            });
                        });
                    }
                })
        })
}

/// Setup signal listeners to trigger canvas redraws when state changes
fn setup_canvas_signal_listeners(
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

    // Listen to timeline state changes
    Task::start({
        let waveform_timeline = waveform_timeline.clone();
        async move {
            // TODO: Use actual waveform_timeline viewport signal when available
            // waveform_timeline.viewport_signal()
            //     .for_each_sync(|_viewport| {
            //         trigger_canvas_redraw_global();
            //     })
            //     .await;
        }
    });

    // Listen to cursor position changes  
    Task::start({
        let waveform_timeline = waveform_timeline.clone();
        async move {
            // TODO: Use actual waveform_timeline cursor position signal when available
            // waveform_timeline.cursor_position_signal()
            //     .for_each_sync(|_cursor_pos| {
            //         trigger_canvas_redraw_global();
            //     })
            //     .await;
        }
    });

    // Listen to selected variables changes
    Task::start({
        let selected_variables = selected_variables.clone();
        async move {
            selected_variables.variables_vec_signal.signal_cloned()
                .for_each_sync(|_variables| {
                    trigger_canvas_redraw_global();
                })
                .await;
        }
    });

    // Listen to theme changes
    Task::start({
        let app_config = app_config.clone();
        async move {
            app_config
                .theme_actor
                .signal()
                .for_each_sync(|theme| {
                let novyui_theme = match theme {
                    shared::Theme::Dark => moonzoon_novyui::tokens::theme::Theme::Dark,
                    shared::Theme::Light => moonzoon_novyui::tokens::theme::Theme::Light,
                };
                // Update theme on renderer and trigger redraw
                RENDERER.with(|r| {
                    if let Some(renderer_rc) = r.borrow().as_ref() {
                        if let Ok(mut renderer) = renderer_rc.try_borrow_mut() {
                            renderer.set_theme(novyui_theme);
                        }
                    }
                });
                trigger_canvas_redraw_global();
            })
            .await;
        }
    });
}

/// Mark canvas as fully initialized and ready for rendering
fn mark_canvas_initialized() {
    CANVAS_INITIALIZED.with(|initialized| {
        *initialized.borrow_mut() = true;
    });
}

/// Check if canvas is fully initialized
fn is_canvas_initialized() -> bool {
    CANVAS_INITIALIZED.with(|initialized| *initialized.borrow())
}

/// Global wrapper for trigger_canvas_redraw that accesses the renderer instance
pub fn trigger_canvas_redraw_global() {
    // ✅ FIX: Only attempt renders after canvas is fully initialized
    if !is_canvas_initialized() {
        return;
    }

    RENDERER.with(|r| {
        if let Some(renderer_rc) = r.borrow().as_ref() {
            match renderer_rc.try_borrow_mut() {
                Ok(mut renderer) => {
                    if renderer.has_canvas() {
                        if renderer.needs_redraw() {
                            // TODO: In Actor+Relay architecture, get selected_variables from domain parameter
                            let empty_variables: Vec<shared::SelectedVariable> = vec![];
                            renderer.render_frame(&empty_variables);
                        } else {
                        }
                    } else {
                    }
                }
                Err(_) => {}
            }
        } else {
        }
    });
}
