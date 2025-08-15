use zoon::*;
use fast2d;
use crate::state::{SELECTED_VARIABLES, LOADED_FILES, TIMELINE_CURSOR_POSITION, CANVAS_WIDTH, CANVAS_HEIGHT, 
    IS_ZOOMING_IN, IS_ZOOMING_OUT, IS_PANNING_LEFT, IS_PANNING_RIGHT, MOUSE_X_POSITION, MOUSE_TIME_POSITION, TIMELINE_ZOOM_LEVEL, 
    TIMELINE_VISIBLE_RANGE_START, TIMELINE_VISIBLE_RANGE_END};
use crate::connection::send_up_msg;
use crate::config::current_theme;
use shared::{SelectedVariable, UpMsg, SignalTransitionQuery, SignalTransition};
use std::rc::Rc;
use std::cell::RefCell;
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::Theme as SharedTheme;
use wasm_bindgen::JsCast;


// Cache for real signal transition data from backend - PUBLIC for connection.rs
pub static SIGNAL_TRANSITIONS_CACHE: Lazy<Mutable<HashMap<String, Vec<SignalTransition>>>> = Lazy::new(|| {
    Mutable::new(HashMap::new())
});
use std::collections::HashMap;

// Clear processed signal cache to force fresh calculation for timeline changes
pub fn clear_processed_signal_cache() {
    // CRITICAL FIX: Don't clear the raw backend data cache!
    // The raw backend data should persist - we only need to force reprocessing
    // For now, we'll remove the cache clearing since the reactive canvas updates
    // already handle timeline changes properly
    
    // TODO: Implement a proper processed data cache separate from raw backend data
    // SIGNAL_TRANSITIONS_CACHE contains raw backend data and should NOT be cleared
}

// Convert shared theme to NovyUI theme
fn convert_theme(shared_theme: &SharedTheme) -> NovyUITheme {
    match shared_theme {
        SharedTheme::Dark => NovyUITheme::Dark,
        SharedTheme::Light => NovyUITheme::Light,
    }
}

// Get current theme colors as RGBA tuples based on current theme
fn get_current_theme_colors(current_theme: &NovyUITheme) -> ThemeColors {
    match current_theme {
        NovyUITheme::Dark => ThemeColors {
            neutral_2: (45, 47, 50, 1.0),     // Dark theme neutral_2
            neutral_3: (52, 54, 58, 1.0),     // Dark theme neutral_3
            neutral_4: (65, 69, 75, 1.0),     // Dark theme neutral_4
            neutral_5: (75, 79, 86, 1.0),     // Dark theme neutral_5
            neutral_12: (253, 253, 253, 1.0), // Dark theme high contrast text
            cursor_color: (59, 130, 246, 0.8), // Bright blue cursor with transparency
        },
        NovyUITheme::Light => ThemeColors {
            neutral_2: (249, 250, 251, 1.0),  // Light theme neutral_2
            neutral_3: (243, 244, 246, 1.0),  // Light theme neutral_3
            neutral_4: (229, 231, 235, 1.0),  // Light theme neutral_4
            neutral_5: (209, 213, 219, 1.0),  // Light theme neutral_5
            neutral_12: (17, 24, 39, 1.0),    // Light theme high contrast text
            cursor_color: (37, 99, 235, 0.8),  // Bright blue cursor with transparency
        },
    }
}

struct ThemeColors {
    neutral_2: (u8, u8, u8, f32),
    neutral_3: (u8, u8, u8, f32),
    neutral_4: (u8, u8, u8, f32),
    neutral_5: (u8, u8, u8, f32),
    neutral_12: (u8, u8, u8, f32),
    cursor_color: (u8, u8, u8, f32),
}

// Helper function to round raw time steps to professional-looking numbers
fn round_to_nice_number(raw: f32) -> f32 {
    if raw <= 0.0 { return 1.0; }
    
    let magnitude = 10.0_f32.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    
    let nice_normalized = if normalized <= 1.0 { 1.0 }
    else if normalized <= 2.0 { 2.0 }
    else if normalized <= 5.0 { 5.0 }
    else { 10.0 };
    
    nice_normalized * magnitude
}

pub fn waveform_canvas() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child_signal(create_canvas_element().into_signal_option())
}

async fn create_canvas_element() -> impl Element {
    let mut zoon_canvas = Canvas::new()
        .width(0)
        .height(0)
        .s(Width::fill())
        .s(Height::fill());

    let dom_canvas = zoon_canvas.raw_el_mut().dom_element();
    let mut canvas_wrapper = fast2d::CanvasWrapper::new_with_canvas(dom_canvas).await;

    // Initialize with default dark theme (theme reactivity will update it)
    canvas_wrapper.update_objects(|objects| {
        let selected_vars = SELECTED_VARIABLES.lock_ref();
        *objects = create_waveform_objects_with_theme(&selected_vars, &NovyUITheme::Dark);
    });

    // Wrap canvas_wrapper in Rc<RefCell> for sharing
    let canvas_wrapper_shared = Rc::new(RefCell::new(canvas_wrapper));
    let canvas_wrapper_for_signal = canvas_wrapper_shared.clone();

    // Add reactive updates when SELECTED_VARIABLES changes
    Task::start(async move {
        SELECTED_VARIABLES.signal_vec_cloned().for_each(move |_| {
            let canvas_wrapper_for_signal = canvas_wrapper_for_signal.clone();
            async move {
                canvas_wrapper_for_signal.borrow_mut().update_objects(|objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    // Use dark theme as fallback - theme handler will update with correct theme
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &NovyUITheme::Dark, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when theme changes
    let canvas_wrapper_for_theme = canvas_wrapper_shared.clone();
    Task::start(async move {
        current_theme().for_each(move |theme_value| {
            let canvas_wrapper_for_theme = canvas_wrapper_for_theme.clone();
            async move {
                canvas_wrapper_for_theme.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    let novyui_theme = convert_theme(&theme_value);
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when zoom state changes
    let canvas_wrapper_for_zoom = canvas_wrapper_shared.clone();
    Task::start(async move {
        crate::state::TIMELINE_ZOOM_LEVEL.signal().for_each(move |_| {
            let canvas_wrapper_for_zoom = canvas_wrapper_for_zoom.clone();
            async move {
                canvas_wrapper_for_zoom.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    // Use dark theme as fallback - theme handler will update with correct theme
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &NovyUITheme::Dark, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when cursor position changes (for new signal data)
    let canvas_wrapper_for_cursor = canvas_wrapper_shared.clone();
    Task::start(async move {
        TIMELINE_CURSOR_POSITION.signal().for_each(move |_| {
            let canvas_wrapper_for_cursor = canvas_wrapper_for_cursor.clone();
            async move {
                canvas_wrapper_for_cursor.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    // Use dark theme as fallback - theme handler will update with correct theme
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &NovyUITheme::Dark, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when signal cache changes (for new backend data)
    let canvas_wrapper_for_cache = canvas_wrapper_shared.clone();
    Task::start(async move {
        SIGNAL_TRANSITIONS_CACHE.signal_ref(|_| ()).for_each(move |_| {
            let canvas_wrapper_for_cache = canvas_wrapper_for_cache.clone();
            async move {
                canvas_wrapper_for_cache.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    // Use dark theme as fallback - theme handler will update with correct theme
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &NovyUITheme::Dark, cursor_pos);
                });
            }
        }).await;
    });

    // Clear cache and redraw when timeline range changes (critical for zoom operations)
    let canvas_wrapper_for_timeline_changes = canvas_wrapper_shared.clone();
    Task::start(async move {
        // Combined signal for any timeline range change
        map_ref! {
            let start = TIMELINE_VISIBLE_RANGE_START.signal(),
            let end = TIMELINE_VISIBLE_RANGE_END.signal(),
            let zoom = TIMELINE_ZOOM_LEVEL.signal()
            => (*start, *end, *zoom)
        }
        .dedupe() // Prevent duplicate triggers
        .for_each(move |_| {
            let canvas_wrapper = canvas_wrapper_for_timeline_changes.clone();
            async move {
                // CRITICAL: Clear all cached processed data when timeline changes
                clear_processed_signal_cache();
                
                canvas_wrapper.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &NovyUITheme::Dark, cursor_pos);
                });
            }
        }).await;
    });

    let canvas_wrapper_for_resize = canvas_wrapper_shared.clone();
    zoon_canvas.update_raw_el(move |raw_el| {
        raw_el.on_resize(move |width, height| {
            
            // Store canvas dimensions for click calculations
            CANVAS_WIDTH.set(width as f32);
            CANVAS_HEIGHT.set(height as f32);
            
            canvas_wrapper_for_resize.borrow_mut().resized(width, height);
            // Re-create objects with new dimensions
            canvas_wrapper_for_resize.borrow_mut().update_objects(move |objects| {
                let selected_vars = SELECTED_VARIABLES.lock_ref();
                let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                // Use dark theme as fallback - theme handler will update with correct theme
                *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, width as f32, height as f32, &NovyUITheme::Dark, cursor_pos);
            });
        })
        .event_handler({
            let canvas_wrapper_for_click = canvas_wrapper_shared.clone();
            move |event: events::Click| {
                // Handle click to move cursor position
                let page_click_x = event.x() as f32;
                
                // Get canvas element's position relative to page
                let canvas_element = event.target().unwrap();
                let canvas_rect = canvas_element.dyn_into::<web_sys::Element>()
                    .unwrap().get_bounding_client_rect();
                let canvas_left = canvas_rect.left() as f32;
                
                // Calculate click position relative to canvas
                let click_x = page_click_x - canvas_left;
                
                // Use stored canvas width
                let canvas_width = CANVAS_WIDTH.get();
                let canvas_height = CANVAS_HEIGHT.get();
                
                // Calculate time from click position using consistent timeline range
                let (min_time, max_time) = get_current_timeline_range();
                let time_range = max_time - min_time;
                let clicked_time = min_time + (click_x / canvas_width) * time_range;
                
                // Clamp to valid range
                let clicked_time = clicked_time.max(min_time).min(max_time);
                
                // Update cursor position
                TIMELINE_CURSOR_POSITION.set(clicked_time);
                
                // Immediately redraw canvas with new cursor position
                canvas_wrapper_for_click.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let novyui_theme = NovyUITheme::Dark;
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, clicked_time);
                });
            }
        })
        .event_handler(move |event: events::PointerMove| {
            // Track mouse position for zoom center calculations
            let page_mouse_x = event.x() as f32;
            
            // Get canvas element's position relative to page
            let canvas_element = event.target().unwrap();
            let canvas_rect = canvas_element.dyn_into::<web_sys::Element>()
                .unwrap().get_bounding_client_rect();
            let canvas_left = canvas_rect.left() as f32;
            
            // Calculate mouse position relative to canvas
            let mouse_x = page_mouse_x - canvas_left;
            MOUSE_X_POSITION.set_neq(mouse_x);
            
            // Convert mouse X to timeline time
            let canvas_width = CANVAS_WIDTH.get();
            let (min_time, max_time) = get_current_timeline_range();
            let time_range = max_time - min_time;
            let mouse_time = min_time + (mouse_x / canvas_width) * time_range;
            
            // Clamp to valid range and update mouse time position
            let mouse_time = mouse_time.max(min_time).min(max_time);
            MOUSE_TIME_POSITION.set_neq(mouse_time);
        })
    })
}




// Get signal transitions for a variable within time range
fn get_signal_transitions_for_variable(var: &SelectedVariable, time_range: (f32, f32)) -> Vec<(f32, String)> {
    // Parse unique_id: "/path/file.ext|scope|variable"
    let parts: Vec<&str> = var.unique_id.split('|').collect();
    if parts.len() < 3 {
        return vec![(time_range.0, "0".to_string())];
    }
    
    let file_path = parts[0];
    let scope_path = parts[1]; 
    let variable_name = parts[2];
    
    // Create cache key for raw backend data (without timeline range since we'll process fresh each time)
    let raw_cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    
    // Check if we have real backend data cached
    let cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
    if let Some(transitions) = cache.get(&raw_cache_key) {
        
        // Convert real backend data to canvas format with proper waveform logic
        // Include ALL transitions to determine proper rectangle boundaries
        let mut canvas_transitions: Vec<(f32, String)> = transitions.iter()
            .map(|t| (t.time_seconds as f32, t.value.clone()))
            .collect();
            
        // Sort by time to ensure proper ordering
        canvas_transitions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // CRITICAL FIX: Always add initial value continuation at timeline start
        // Find what value should be active at the beginning of the visible timeline
        let mut initial_value = "X".to_string(); // Default unknown state
        
        // Find the most recent transition before the visible timeline starts
        for transition in transitions.iter() {
            if transition.time_seconds <= time_range.0 as f64 {
                initial_value = transition.value.clone();
            } else {
                break; // Transitions should be in time order
            }
        }
        
        // Check if we need to add initial continuation rectangle
        let needs_initial_continuation = canvas_transitions.is_empty() || 
            canvas_transitions[0].0 > time_range.0;
        
        if needs_initial_continuation {
            // Insert initial value at timeline start
            canvas_transitions.insert(0, (time_range.0, initial_value.clone()));
        }
        
        // Handle empty transitions case (keep existing logic for compatibility)
        if canvas_transitions.is_empty() && !transitions.is_empty() {
            canvas_transitions.push((time_range.0, initial_value.clone()));
            canvas_transitions.push((time_range.1, initial_value));
        }
        
        // Backend now provides proper signal termination transitions - no frontend workaround needed
        
        return canvas_transitions;
    }
    drop(cache);
    
    // No cached data - request real data from backend
    request_signal_transitions_from_backend(file_path, scope_path, variable_name, time_range);
    
    // Return empty data while waiting for real backend response
    // This prevents premature filler rectangles from covering actual values
    vec![]
}

// Get the actual end time of the waveform file for a variable
fn get_actual_signal_end_time(var: &SelectedVariable) -> Option<f32> {
    // Parse file path from unique_id: "/path/file.ext|scope|variable"
    let file_path = var.unique_id.split('|').next()?;
    
    // Find the corresponding WaveformFile in LOADED_FILES
    let loaded_files = crate::state::LOADED_FILES.lock_ref();
    for waveform_file in loaded_files.iter() {
        // Check if this file's path matches the variable's file path
        if file_path.ends_with(&waveform_file.filename) || file_path == waveform_file.filename {
            return waveform_file.max_time.map(|t| t as f32);
        }
    }
    
    None
}

// Request real signal transitions from backend
fn request_signal_transitions_from_backend(file_path: &str, scope_path: &str, variable_name: &str, _time_range: (f32, f32)) {
    
    let query = SignalTransitionQuery {
        scope_path: scope_path.to_string(),
        variable_name: variable_name.to_string(),
    };
    
    // Request wider time range to get transitions that affect visible area
    // Include entire file range to get proper rectangle boundaries
    let (file_min, file_max) = {
        let loaded_files = LOADED_FILES.lock_ref();
        if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path || file_path.ends_with(&f.filename)) {
            (
                loaded_file.min_time.unwrap_or(0.0) as f64,
                loaded_file.max_time.unwrap_or(1000.0) as f64  // Use higher fallback to avoid premature filler
            )
        } else {
            // Don't make request if file isn't loaded yet - prevents race condition
            return;
        }
    };
    
    let message = UpMsg::QuerySignalTransitions {
        file_path: file_path.to_string(),
        signal_queries: vec![query],
        time_range: (file_min, file_max), // Request entire file range
    };
    
    // Send real backend request
    send_up_msg(message);
}

// Trigger canvas redraw when new signal data arrives
pub fn trigger_canvas_redraw() {
    // Trigger canvas redraw without disturbing variable state
    // Use cursor position signal to force redraw - set_neq forces signal even with same value
    let current_cursor = TIMELINE_CURSOR_POSITION.get();
    TIMELINE_CURSOR_POSITION.set_neq(current_cursor);
}

// ROCK-SOLID coordinate transformation system with zoom reliability
fn get_current_timeline_range() -> (f32, f32) {
    let zoom_level = crate::state::TIMELINE_ZOOM_LEVEL.get();
    
    // If zoomed in, return the visible range with validation
    if zoom_level > 1.0 {
        let range_start = crate::state::TIMELINE_VISIBLE_RANGE_START.get();
        let range_end = crate::state::TIMELINE_VISIBLE_RANGE_END.get();
        
        // CRITICAL: Enforce minimum time range to prevent coordinate precision loss
        let min_zoom_range = 0.001; // Minimum 1ms range prevents division by near-zero
        let current_range = range_end - range_start;
        
        // Validate range is sensible and has sufficient precision
        if range_end > range_start && range_start >= 0.0 && current_range >= min_zoom_range {
            return (range_start, range_end);
        }
        
        // If zoom range is too narrow, expand it to minimum viable range
        if current_range > 0.0 && current_range < min_zoom_range {
            let range_center = (range_start + range_end) / 2.0;
            let half_min_range = min_zoom_range / 2.0;
            let expanded_start = (range_center - half_min_range).max(0.0);
            let expanded_end = range_center + half_min_range;
            return (expanded_start, expanded_end);
        }
        
        // Fall through to full range if zoom range is invalid
    }
    
    // Default behavior: get full file range with validation
    let loaded_files = LOADED_FILES.lock_ref();
    
    // Get timeline range from ALL loaded files, not just selected variables
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
    let mut has_valid_files = false;
    
    for file in loaded_files.iter() {
        if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
            min_time = min_time.min(file_min as f32);
            max_time = max_time.max(file_max as f32);
            has_valid_files = true;
        }
    }
    
    if !has_valid_files || min_time == max_time {
        // Reasonable default for empty/invalid files
        (0.0, 100.0)
    } else {
        // Ensure minimum range for coordinate precision
        let file_range = max_time - min_time;
        if file_range < 0.001 {
            (min_time, min_time + 0.001)
        } else {
            (min_time, max_time)
        }
    }
}

// Smooth zoom functions with mouse-centered behavior
pub fn start_smooth_zoom_in() {
    if !IS_ZOOMING_IN.get() {
        IS_ZOOMING_IN.set_neq(true);
        Task::start(async move {
            while IS_ZOOMING_IN.get() {
                let current = TIMELINE_ZOOM_LEVEL.get();
                let new_zoom = (current * 1.02).min(16.0); // Smaller increments for smoothness
                if new_zoom != current {
                    update_zoom_with_mouse_center(new_zoom);
                } else {
                    break; // Hit zoom limit
                }
                Timer::sleep(16).await; // 60fps updates
            }
        });
    }
}

pub fn start_smooth_zoom_out() {
    if !IS_ZOOMING_OUT.get() {
        IS_ZOOMING_OUT.set_neq(true);
        Task::start(async move {
            while IS_ZOOMING_OUT.get() {
                let current = TIMELINE_ZOOM_LEVEL.get();
                let new_zoom = (current / 1.02).max(1.0); // Smaller increments for smoothness
                if new_zoom != current {
                    update_zoom_with_mouse_center(new_zoom);
                } else {
                    break; // Hit zoom limit
                }
                Timer::sleep(16).await; // 60fps updates
            }
        });
    }
}

pub fn stop_smooth_zoom_in() {
    IS_ZOOMING_IN.set_neq(false);
}

pub fn stop_smooth_zoom_out() {
    IS_ZOOMING_OUT.set_neq(false);
}

// Smooth pan functions
pub fn start_smooth_pan_left() {
    if !IS_PANNING_LEFT.get() {
        IS_PANNING_LEFT.set_neq(true);
        Task::start(async move {
            while IS_PANNING_LEFT.get() {
                let zoom_level = TIMELINE_ZOOM_LEVEL.get();
                if zoom_level > 1.0 {  // Only pan when zoomed in
                    let current_start = TIMELINE_VISIBLE_RANGE_START.get();
                    let current_end = TIMELINE_VISIBLE_RANGE_END.get();
                    let visible_range = current_end - current_start;
                    
                    // Smooth pan: 2% of visible range per frame
                    let pan_distance = visible_range * 0.02;
                    
                    // Get file bounds for clamping
                    let (file_min, _file_max) = get_full_file_range();
                    
                    // Calculate new positions
                    let new_start = (current_start - pan_distance).max(file_min);
                    let new_end = new_start + visible_range;
                    
                    // Update if changed (pan left succeeded)
                    if new_start != current_start {
                        TIMELINE_VISIBLE_RANGE_START.set_neq(new_start);
                        TIMELINE_VISIBLE_RANGE_END.set_neq(new_end);
                    } else {
                        break; // Hit left boundary
                    }
                }
                Timer::sleep(16).await; // 60fps for smooth motion
            }
        });
    }
}

pub fn start_smooth_pan_right() {
    if !IS_PANNING_RIGHT.get() {
        IS_PANNING_RIGHT.set_neq(true);
        Task::start(async move {
            while IS_PANNING_RIGHT.get() {
                let zoom_level = TIMELINE_ZOOM_LEVEL.get();
                if zoom_level > 1.0 {  // Only pan when zoomed in
                    let current_start = TIMELINE_VISIBLE_RANGE_START.get();
                    let current_end = TIMELINE_VISIBLE_RANGE_END.get();
                    let visible_range = current_end - current_start;
                    
                    // Smooth pan: 2% of visible range per frame
                    let pan_distance = visible_range * 0.02;
                    
                    // Get file bounds for clamping
                    let (_file_min, file_max) = get_full_file_range();
                    
                    // Calculate new positions
                    let new_end = (current_end + pan_distance).min(file_max);
                    let new_start = new_end - visible_range;
                    
                    // Update if changed (pan right succeeded)
                    if new_end != current_end {
                        TIMELINE_VISIBLE_RANGE_START.set_neq(new_start);
                        TIMELINE_VISIBLE_RANGE_END.set_neq(new_end);
                    } else {
                        break; // Hit right boundary
                    }
                }
                Timer::sleep(16).await; // 60fps for smooth motion
            }
        });
    }
}

pub fn stop_smooth_pan_left() {
    IS_PANNING_LEFT.set_neq(false);
}

pub fn stop_smooth_pan_right() {
    IS_PANNING_RIGHT.set_neq(false);
}

// Legacy zoom functions for button compatibility
pub fn zoom_in() {
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    let new_zoom = (current_zoom * 1.5).min(16.0);
    if new_zoom != current_zoom {
        update_zoom_with_mouse_center(new_zoom);
    }
}

pub fn zoom_out() {
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    let new_zoom = (current_zoom / 1.5).max(1.0);
    if new_zoom != current_zoom {
        update_zoom_with_mouse_center(new_zoom);
    }
}

fn update_zoom_level_and_visible_range(new_zoom: f32, center_time: f32) {
    // Set the new zoom level
    crate::state::TIMELINE_ZOOM_LEVEL.set_neq(new_zoom);
    
    if new_zoom <= 1.0 {
        // Full zoom - use entire file range
        let (file_min, file_max) = get_full_file_range();
        crate::state::TIMELINE_VISIBLE_RANGE_START.set_neq(file_min);
        crate::state::TIMELINE_VISIBLE_RANGE_END.set_neq(file_max);
    } else {
        // Zoomed in - calculate visible range centered on cursor
        let (file_min, file_max) = get_full_file_range();
        let full_range = file_max - file_min;
        let visible_range = full_range / new_zoom;
        
        // Center the visible range on the cursor position
        let half_visible = visible_range / 2.0;
        let mut range_start = center_time - half_visible;
        let mut range_end = center_time + half_visible;
        
        // Clamp to file bounds
        if range_start < file_min {
            let offset = file_min - range_start;
            range_start = file_min;
            range_end = (range_end + offset).min(file_max);
        } else if range_end > file_max {
            let offset = range_end - file_max;
            range_end = file_max;
            range_start = (range_start - offset).max(file_min);
        }
        
        crate::state::TIMELINE_VISIBLE_RANGE_START.set_neq(range_start);
        crate::state::TIMELINE_VISIBLE_RANGE_END.set_neq(range_end);
    }
}

// Mouse-centered zoom function
fn update_zoom_with_mouse_center(new_zoom: f32) {
    let mouse_time = MOUSE_TIME_POSITION.get();
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    
    // Set the new zoom level
    TIMELINE_ZOOM_LEVEL.set_neq(new_zoom);
    
    if new_zoom <= 1.0 {
        // Full zoom - use entire file range
        let (file_min, file_max) = get_full_file_range();
        TIMELINE_VISIBLE_RANGE_START.set_neq(file_min);
        TIMELINE_VISIBLE_RANGE_END.set_neq(file_max);
    } else {
        // Zoomed in - calculate visible range centered on mouse position
        let (current_start, current_end) = get_current_timeline_range();
        let current_range = current_end - current_start;
        
        // Calculate new range
        let new_range = current_range * (current_zoom / new_zoom);
        
        // Position mouse time as the center of zoom
        let mouse_ratio = if current_range > 0.0 {
            (mouse_time - current_start) / current_range
        } else {
            0.5 // Default to center if range is invalid
        };
        let new_start = mouse_time - (new_range * mouse_ratio);
        let new_end = new_start + new_range;
        
        // Clamp to file bounds
        let (file_min, file_max) = get_full_file_range();
        let mut clamped_start = new_start.max(file_min);
        let mut clamped_end = new_end.min(file_max);
        
        // Ensure minimum range if we hit bounds
        if clamped_end - clamped_start < new_range {
            if new_start < file_min {
                clamped_end = (clamped_start + new_range).min(file_max);
            } else if new_end > file_max {
                clamped_start = (clamped_end - new_range).max(file_min);
            }
        }
        
        TIMELINE_VISIBLE_RANGE_START.set_neq(clamped_start);
        TIMELINE_VISIBLE_RANGE_END.set_neq(clamped_end);
    }
}

fn get_full_file_range() -> (f32, f32) {
    let loaded_files = LOADED_FILES.lock_ref();
    
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
    let mut has_valid_files = false;
    
    for file in loaded_files.iter() {
        if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
            min_time = min_time.min(file_min as f32);
            max_time = max_time.max(file_max as f32);
            has_valid_files = true;
        }
    }
    
    if !has_valid_files || min_time == max_time {
        (0.0, 100.0)
    } else {
        (min_time, max_time)
    }
}

fn create_waveform_objects_with_theme(selected_vars: &[SelectedVariable], theme: &NovyUITheme) -> Vec<fast2d::Object2d> {
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let canvas_width = CANVAS_WIDTH.get();
    let canvas_height = CANVAS_HEIGHT.get();
    create_waveform_objects_with_dimensions_and_theme(selected_vars, canvas_width, canvas_height, theme, cursor_pos)
}

fn create_waveform_objects_with_dimensions_and_theme(selected_vars: &[SelectedVariable], canvas_width: f32, canvas_height: f32, theme: &NovyUITheme, cursor_position: f32) -> Vec<fast2d::Object2d> {
    let mut objects = Vec::new();
    
    
    // Get current theme colors
    let theme_colors = get_current_theme_colors(theme);
    
    // Calculate row layout according to specs
    let total_rows = selected_vars.len() + 1; // variables + timeline
    let row_height = if total_rows > 0 { canvas_height / total_rows as f32 } else { canvas_height };
    
    // Create alternating row backgrounds for variable rows
    for (index, var) in selected_vars.iter().enumerate() {
        let y_position = index as f32 * row_height;
        let is_even_row = index % 2 == 0;
        
        // Theme-aware alternating backgrounds using current theme colors
        let background_color = if is_even_row {
            theme_colors.neutral_2
        } else {
            theme_colors.neutral_3
        };
        
        
        objects.push(
            fast2d::Rectangle::new()
                .position(0.0, y_position)
                .size(canvas_width, row_height)
                .color(background_color.0, background_color.1, background_color.2, background_color.3)
                .into()
        );
        
        // Create value rectangles based on live data from selected variables
        let _variable_name = var.unique_id.split('|').last().unwrap_or("Unknown");
        
        // Get the user's selected format for this variable
        let format = var.formatter.unwrap_or_default();
        
        // Phase 7: Multi-file support - get data based on variable's source file
        // Parse file path from unique_id: "/path/file.ext|scope|variable"
        let file_path = var.unique_id.split('|').next().unwrap_or("");
        let _file_name = file_path.split('/').last().unwrap_or("unknown");
        
        let current_time_range = get_current_timeline_range();
        
        
        let time_value_pairs = get_signal_transitions_for_variable(var, current_time_range);
        
        // Timeline range already calculated in get_current_timeline_range()
        
        
        // Get visible time range for proper clipping
        let (min_time, max_time) = get_current_timeline_range();
        
        for (rect_index, (start_time, binary_value)) in time_value_pairs.iter().enumerate() {
            // Calculate end time for this rectangle (next transition time or view window end)
            let end_time = if rect_index + 1 < time_value_pairs.len() {
                time_value_pairs[rect_index + 1].0 // Next transition time
            } else {
                // Last rectangle: extend to view window end for proper visual coverage
                // Backend provides proper filler transitions, so this is safe
                max_time
            };
            
            // Skip rectangles completely outside visible range
            if end_time <= min_time || *start_time >= max_time {
                continue;
            }
            
            // Clip rectangle to visible time range
            let visible_start_time = start_time.max(min_time);
            let visible_end_time = end_time.min(max_time);
            
            // GRAPHICS-LEVEL FIX: Robust coordinate transformation with precision handling
            let time_range = max_time - min_time;
            
            // Prevent division by zero and handle degenerate cases
            if time_range <= 0.0 || canvas_width <= 0.0 {
                continue; // Skip rendering in degenerate cases
            }
            
            // High-precision coordinate calculation with explicit bounds checking
            let time_to_pixel_ratio = canvas_width / time_range;
            let rect_start_x = (visible_start_time - min_time) * time_to_pixel_ratio;
            let rect_end_x = (visible_end_time - min_time) * time_to_pixel_ratio;
            
            // CRITICAL: Enforce minimum visible width to prevent zero rectangles
            let raw_rect_width = rect_end_x - rect_start_x;
            let min_visible_width = 1.0; // Minimum 1 pixel width for visibility
            let rect_width = raw_rect_width.max(min_visible_width);
            
            // Bounds checking: ensure rectangle fits within canvas
            let rect_start_x = rect_start_x.max(0.0).min(canvas_width - rect_width);
            let rect_end_x = rect_start_x + rect_width;
            
            // Debug logging for zero rectangle detection (throttled)
            if raw_rect_width <= 0.0 {
                zoon::println!("ZERO RECT FIXED: time_range={:.6}, raw_width={:.6}, fixed_width={:.6}, zoom={:.1}", 
                    time_range, raw_rect_width, rect_width, crate::state::TIMELINE_ZOOM_LEVEL.get());
            }
            
            // Validate final dimensions before creating Fast2D objects
            if rect_width <= 0.0 || rect_start_x >= canvas_width || rect_end_x <= 0.0 {
                continue; // Skip invalid rectangles
            }
            
            let is_even_rect = rect_index % 2 == 0;
            
            // Theme-aware alternating rectangle colors using current theme colors
            let rect_color = if is_even_rect {
                theme_colors.neutral_4
            } else {
                theme_colors.neutral_5
            };
            
            // Create value rectangle with actual time-based width
            objects.push(
                fast2d::Rectangle::new()
                    .position(rect_start_x, y_position + 2.0)
                    .size(rect_width, row_height - 4.0)
                    .color(rect_color.0, rect_color.1, rect_color.2, rect_color.3)
                    .into()
            );
            
            // Format the binary value using the user's selected format (without prefix)
            let formatted_value = format.format(binary_value);
            
            // Add formatted value text with robust positioning
            let text_color = theme_colors.neutral_12; // High contrast text
            let text_padding = 5.0;
            let text_width = (rect_width - (text_padding * 2.0)).max(0.0);
            let text_height = (row_height / 2.0).max(8.0); // Minimum readable height
            
            // Only render text if there's sufficient space
            if text_width >= 10.0 && text_height >= 8.0 {
                objects.push(
                    fast2d::Text::new()
                        .text(formatted_value)
                        .position(rect_start_x + text_padding, y_position + row_height / 3.0)
                        .size(text_width, text_height)
                        .color(text_color.0, text_color.1, text_color.2, text_color.3)
                        .font_size(12.0)
                        .family(fast2d::Family::name("Fira Code")) // FiraCode monospace font
                        .into()
                );
            }
        }
    }
    
    // Create timeline row background (last row) using theme-aware color
    if total_rows > 0 {
        let timeline_y = (total_rows - 1) as f32 * row_height;
        
        let timeline_bg_color = theme_colors.neutral_2; // Consistent with alternating backgrounds
        objects.push(
            fast2d::Rectangle::new()
                .position(0.0, timeline_y)
                .size(canvas_width, row_height)
                .color(timeline_bg_color.0, timeline_bg_color.1, timeline_bg_color.2, timeline_bg_color.3)
                .into()
        );
        
        // Get consistent timeline range
        let (min_time, max_time) = get_current_timeline_range();
        let time_range = max_time - min_time;
        
        // Phase 9: Pixel-based spacing algorithm for professional timeline
        let target_tick_spacing = 80.0; // Target 80 pixels between ticks
        let max_tick_count = (canvas_width / target_tick_spacing).floor() as i32;
        let tick_count = max_tick_count.max(2).min(8); // Ensure 2-8 ticks
        
        // Calculate round time intervals
        let raw_time_step = time_range / (tick_count - 1) as f32;
        let time_step = round_to_nice_number(raw_time_step);
        
        // Find the first tick that's >= min_time (aligned to step boundaries)
        let first_tick = (min_time / time_step).ceil() * time_step;
        let last_tick = max_time;
        let actual_tick_count = ((last_tick - first_tick) / time_step).ceil() as i32 + 1;
        
        for tick_index in 0..actual_tick_count {
            let time_value = first_tick + (tick_index as f32 * time_step);
            let time_value = time_value.min(max_time);
            let x_position = ((time_value - min_time) / time_range) * canvas_width;
            
            // Skip edge labels to prevent cutoff (10px margin on each side)
            let label_margin = 10.0;
            let should_show_label = x_position >= label_margin && x_position <= (canvas_width - label_margin);
            
            // Create vertical tick mark with theme-aware color
            let tick_color = theme_colors.neutral_12; // High contrast for visibility
            objects.push(
                fast2d::Rectangle::new()
                    .position(x_position, timeline_y + row_height - 8.0)
                    .size(1.0, 8.0) // Thin vertical line
                    .color(tick_color.0, tick_color.1, tick_color.2, tick_color.3)
                    .into()
            );
            
            // Add time label with actual time units and theme-aware color (only if not cut off)
            if should_show_label {
                let time_label = format!("{}s", time_value as u32);
                let label_color = theme_colors.neutral_12; // High contrast text
                objects.push(
                    fast2d::Text::new()
                        .text(time_label)
                        .position(x_position - 10.0, timeline_y + 5.0)
                        .size(20.0, row_height - 15.0)
                        .color(label_color.0, label_color.1, label_color.2, label_color.3)
                        .font_size(10.0)
                        .family(fast2d::Family::name("Inter")) // Standard UI font for timeline
                        .into()
                );
            }
        }
    }
    
    // Add timeline cursor line spanning all rows
    if total_rows > 0 {
        // Use consistent timeline range
        let (min_time, max_time) = get_current_timeline_range();
        let time_range = max_time - min_time;
        
        // Calculate cursor x position only if cursor is within visible range
        if cursor_position >= min_time && cursor_position <= max_time {
            let cursor_x = ((cursor_position - min_time) / time_range) * canvas_width;
            
            // Draw vertical cursor line spanning all rows (including timeline)
            let cursor_color = theme_colors.cursor_color;
            objects.push(
                fast2d::Rectangle::new()
                    .position(cursor_x - 1.0, 0.0) // Center the 3px line
                    .size(3.0, canvas_height) // 3px thick line spanning full height
                    .color(cursor_color.0, cursor_color.1, cursor_color.2, cursor_color.3)
                    .into()
            );
            
        }
    }
    
    objects
}