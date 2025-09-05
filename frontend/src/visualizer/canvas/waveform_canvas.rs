use zoon::*;
use crate::visualizer::timeline::timeline_actor::{
    current_cursor_position_seconds, current_viewport, 
    current_ns_per_pixel, set_canvas_dimensions
};
use crate::visualizer::timeline::time_types::{TimeNs, Viewport, NsPerPixel};
use super::rendering::WaveformRenderer;

// Re-export all functions from sub-modules for API compatibility
pub use super::animation::*;
pub use super::timeline::*;
pub use super::navigation::*;

// WASM-safe local renderer instance using RefCell
use std::rc::Rc;
use std::cell::RefCell;
thread_local! {
    static RENDERER: RefCell<Option<Rc<RefCell<WaveformRenderer>>>> = RefCell::new(None);
    static CANVAS_INITIALIZED: RefCell<bool> = RefCell::new(false);
}

/// Main waveform canvas UI component
pub fn waveform_canvas() -> impl Element {
    zoon::println!("🎨 CANVAS: waveform_canvas() called - creating canvas component");
    
    // Initialize renderer instance first
    RENDERER.with(|r| {
        if r.borrow().is_none() {
            zoon::println!("🎨 CANVAS: Creating new WaveformRenderer instance");
            *r.borrow_mut() = Some(Rc::new(RefCell::new(WaveformRenderer::new())));
        }
    });
    
    // Start signal listeners for canvas updates
    setup_canvas_signal_listeners();
    
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(create_canvas_element())
}

/// Canvas element creation with Fast2D integration
fn create_canvas_element() -> impl Element {
    zoon::println!("🎨 CANVAS: create_canvas_element() called - building Canvas::new()");
    
    Canvas::new()
        .width(640)
        .height(480)
        .update_raw_el(move |raw_el| {
            raw_el
                .on_resize(move |width, height| {
                    if width > 0 && height > 0 {
                        set_canvas_dimensions(width as f32, height as f32);
                        zoon::println!("🔧 CANVAS: Resized to {}x{} px", width, height);
                        
                        // Trigger canvas redraw on resize
                        trigger_canvas_redraw_global();
                    }
                })
                .after_insert({
                    move |raw_element| {
                        zoon::println!("🎨 CANVAS: DOM canvas element ready for Fast2D initialization");
                        
                        // Initialize Fast2D rendering with DOM canvas element
                        let canvas_clone = raw_element.clone();
                        Task::start(async move {
                            // Wait a tick for DOM to be fully ready
                            Timer::sleep(10).await;
                            
                            let width = canvas_clone.width();
                            let height = canvas_clone.height();
                            
                            zoon::println!("🔧 CANVAS: Initializing Fast2D with canvas {}x{}", width, height);
                            
                            // Initialize Fast2D canvas with the local renderer
                            RENDERER.with(|r| {
                                if let Some(renderer_rc) = r.borrow().as_ref() {
                                    let renderer_clone = renderer_rc.clone();
                                    Task::start(async move {
                                        zoon::println!("🔧 CANVAS: Starting Fast2D canvas initialization");
                                        
                                        // Initialize Fast2D canvas asynchronously
                                        let fast2d_canvas = fast2d::CanvasWrapper::new_with_canvas(canvas_clone).await;
                                        zoon::println!("✅ CANVAS: Fast2D canvas created successfully");
                                        
                                        // Set canvas on renderer 
                                        match renderer_clone.try_borrow_mut() {
                                            Ok(mut renderer) => {
                                                renderer.set_canvas(fast2d_canvas);
                                                zoon::println!("✅ CANVAS: Fast2D canvas set on renderer successfully");
                                                
                                                // ✅ FIX: Mark initialization complete and trigger initial render
                                                mark_canvas_initialized();
                                                zoon::println!("✅ CANVAS: Canvas initialization complete, ready for renders");
                                            },
                                            Err(_) => {
                                                zoon::println!("❌ CANVAS: Failed to borrow renderer for canvas initialization - this is the problem!");
                                            }
                                        }
                                    });
                                    
                                    // NOTE: Initial render will be triggered automatically by set_canvas() method
                                } else {
                                    zoon::println!("❌ CANVAS: No renderer instance found!");
                                }
                            });
                        });
                    }
                })
        })
}

/// Setup signal listeners to trigger canvas redraws when state changes
fn setup_canvas_signal_listeners() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static LISTENERS_SETUP: AtomicBool = AtomicBool::new(false);
    
    // Only setup listeners once
    if LISTENERS_SETUP.swap(true, Ordering::SeqCst) {
        return;
    }
    
    zoon::println!("🔗 CANVAS: Setting up signal listeners for canvas redraws");
    
    // Listen to timeline state changes
    Task::start(async move {
        crate::visualizer::timeline::timeline_actor::viewport_signal()
            .for_each_sync(|_viewport| {
                zoon::println!("📡 CANVAS: Viewport changed, triggering redraw");
                trigger_canvas_redraw_global();
            }).await;
    });
    
    // Listen to cursor position changes
    Task::start(async move {
        crate::visualizer::timeline::timeline_actor::cursor_position_signal()
            .for_each_sync(|_cursor_pos| {
                zoon::println!("📡 CANVAS: Cursor position changed, triggering redraw");
                trigger_canvas_redraw_global();
            }).await;
    });
    
    // Listen to selected variables changes
    Task::start(async move {
        crate::actors::selected_variables::variables_signal()
            .for_each_sync(|variables| {
                zoon::println!("📡 CANVAS: Variables changed ({} selected), triggering redraw", variables.len());
                trigger_canvas_redraw_global();
            }).await;
    });
    
    // Listen to theme changes
    Task::start(async move {
        crate::config::app_config().theme_actor.signal()
            .for_each_sync(|theme| {
                zoon::println!("📡 CANVAS: Theme changed to {:?}, updating renderer", theme);
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
            }).await;
    });
    
    zoon::println!("✅ CANVAS: Signal listeners setup complete");
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
        zoon::println!("⏳ CANVAS: Canvas not yet initialized, deferring redraw");
        return;
    }
    
    RENDERER.with(|r| {
        if let Some(renderer_rc) = r.borrow().as_ref() {
            zoon::println!("🔄 CANVAS: Triggering global canvas redraw");
            match renderer_rc.try_borrow_mut() {
                Ok(mut renderer) => {
                    zoon::println!("✅ CANVAS: Successfully borrowed renderer for redraw");
                    if renderer.has_canvas() {
                        if renderer.needs_redraw() {
                            zoon::println!("🎨 CANVAS: Rendering frame (needs_redraw = true)");
                            renderer.render_frame();
                        } else {
                            zoon::println!("ℹ️ CANVAS: No redraw needed, skipping");
                        }
                    } else {
                        zoon::println!("⚠️ CANVAS: Renderer has no canvas - this should not happen after initialization");
                    }
                },
                Err(_) => {
                    zoon::println!("⚠️ CANVAS: Failed to borrow renderer - already borrowed, skipping redraw");
                }
            }
        } else {
            zoon::println!("⚠️ CANVAS: No renderer instance found for redraw");
        }
    });
}