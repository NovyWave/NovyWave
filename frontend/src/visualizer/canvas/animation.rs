use zoon::*;
use crate::visualizer::state::timeline_state::{IS_ZOOMING_IN, IS_PANNING_LEFT, IS_PANNING_RIGHT, 
    IS_CURSOR_MOVING_LEFT, IS_CURSOR_MOVING_RIGHT};
use crate::visualizer::timeline::timeline_actor::{
    current_cursor_position_seconds, set_viewport_if_changed,
    current_ns_per_pixel, current_coordinates
};
use crate::visualizer::timeline::time_types::{TimeNs, NsPerPixel};
use crate::visualizer::state::canvas_state::{DIRECT_CURSOR_ANIMATION};
// Removed unused import: js_sys





/// Start smooth pan left animation
pub fn start_smooth_pan_left() {
    if !IS_PANNING_LEFT.get() {
        IS_PANNING_LEFT.set_neq(true);
        Task::start(async move {
            while IS_PANNING_LEFT.get() {
                let ns_per_pixel = current_ns_per_pixel();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                // Lower ns_per_pixel means more zoomed in
                if let Some(ns_per_pixel_val) = ns_per_pixel {
                    if ns_per_pixel_val.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || IS_ZOOMING_IN.get() {
                        // Get current coordinates for pan computation
                        let mut coords = match current_coordinates() {
                            Some(coords) => coords,
                            None => break, // Timeline not initialized yet, stop panning
                        };
                        
                        // Check for Shift key for turbo panning
                        let pan_pixels = if crate::visualizer::state::timeline_state::IS_SHIFT_PRESSED.get() {
                            -10  // Turbo pan with Shift (10 pixels per frame)
                        } else {
                            -2   // Normal smooth pan (2 pixels per frame)
                        };
                        
                        // Store original viewport start for comparison
                        let original_start = coords.viewport_start_ns;
                        
                        // Pan by pixels (negative = pan left) using local coordinates
                        coords.pan_by_pixels(pan_pixels);
                        
                        // Get file bounds and clamp viewport
                        let (file_min, file_max) = super::get_full_file_range();
                        let file_start_ns = TimeNs::from_external_seconds(file_min);
                        let file_end_ns = TimeNs::from_external_seconds(file_max);
                        coords.clamp_viewport(file_start_ns, file_end_ns);
                        
                        // Update global viewport state through domain
                        let new_viewport = coords.viewport();
                        set_viewport_if_changed(new_viewport);
                        
                        // Check if we actually moved (if not, we hit boundary)
                        if coords.viewport_start_ns == original_start {
                            break; // Hit left boundary
                        }
                    }
                } else {
                    break; // Timeline not initialized yet, stop panning
                }
                Timer::sleep(16).await; // 60fps for smooth motion
            }
        });
    }
}

/// Start smooth pan right animation
pub fn start_smooth_pan_right() {
    if !IS_PANNING_RIGHT.get() {
        IS_PANNING_RIGHT.set_neq(true);
        Task::start(async move {
            while IS_PANNING_RIGHT.get() {
                let ns_per_pixel = current_ns_per_pixel();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                // Lower ns_per_pixel means more zoomed in
                if let Some(ns_per_pixel_val) = ns_per_pixel {
                    if ns_per_pixel_val.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || IS_ZOOMING_IN.get() {
                        // Get current coordinates for pan computation
                        let mut coords = match current_coordinates() {
                            Some(coords) => coords,
                            None => break, // Timeline not initialized yet, stop panning
                        };
                        
                        // Check for Shift key for turbo panning
                        let pan_pixels = if crate::visualizer::state::timeline_state::IS_SHIFT_PRESSED.get() {
                            10   // Turbo pan with Shift (10 pixels per frame)
                        } else {
                            2    // Normal smooth pan (2 pixels per frame)
                        };
                        
                        // Store original viewport start for comparison
                        let original_start = coords.viewport_start_ns;
                        
                        // Pan by pixels (positive = pan right) using local coordinates
                        coords.pan_by_pixels(pan_pixels);
                        
                        // Get file bounds and clamp viewport
                        let (file_min, file_max) = super::get_full_file_range();
                        let file_start_ns = TimeNs::from_external_seconds(file_min);
                        let file_end_ns = TimeNs::from_external_seconds(file_max);
                        coords.clamp_viewport(file_start_ns, file_end_ns);
                        
                        // Update global viewport state through domain
                        let new_viewport = coords.viewport();
                        set_viewport_if_changed(new_viewport);
                        
                        // Check if we actually moved (if not, we hit boundary)
                        if coords.viewport_start_ns == original_start {
                            break; // Hit right boundary
                        }
                    }
                } else {
                    // Timeline not initialized yet - skip this pan frame
                }
                Timer::sleep(16).await; // 60fps for smooth motion
            }
        });
    }
}

/// Stop smooth pan left animation
pub fn stop_smooth_pan_left() {
    IS_PANNING_LEFT.set_neq(false);
}

/// Stop smooth pan right animation
pub fn stop_smooth_pan_right() {
    IS_PANNING_RIGHT.set_neq(false);
}





/// Start smooth cursor movement to the left
pub fn start_smooth_cursor_left() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = -1;
    animation.is_animating = true;
    animation.current_position = current_cursor_position_seconds().unwrap_or(0.0);
    IS_CURSOR_MOVING_LEFT.set_neq(true);
}

/// Start smooth cursor movement to the right
pub fn start_smooth_cursor_right() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = 1;
    animation.is_animating = true;
    animation.current_position = current_cursor_position_seconds().unwrap_or(0.0);
    IS_CURSOR_MOVING_RIGHT.set_neq(true);
}

/// Stop smooth cursor movement to the left
pub fn stop_smooth_cursor_left() {
    IS_CURSOR_MOVING_LEFT.set_neq(false);
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    if animation.direction == -1 {
        animation.is_animating = false;
        animation.direction = 0;
    }
}

/// Stop smooth cursor movement to the right
pub fn stop_smooth_cursor_right() {
    IS_CURSOR_MOVING_RIGHT.set_neq(false);
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    if animation.direction == 1 {
        animation.is_animating = false;
        animation.direction = 0;
    }
}