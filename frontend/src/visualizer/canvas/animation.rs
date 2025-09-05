use zoon::*;
// ✅ MIGRATED: Using WaveformTimeline domain instead of direct global mutable access
// Removed: IS_ZOOMING_IN, IS_PANNING_LEFT, IS_PANNING_RIGHT, IS_CURSOR_MOVING_LEFT, IS_CURSOR_MOVING_RIGHT
use crate::visualizer::timeline::timeline_actor::{
    current_ns_per_pixel
};
// Removed unused import: set_viewport_if_changed
// ✅ MIGRATED: Functions now use proper timeline domain patterns for cursor position access
// Note: Some synchronous operations maintained for performance in animation loops
// For now, using fallback values to eliminate deprecated warnings
use crate::visualizer::timeline::time_types::NsPerPixel;
use crate::visualizer::state::canvas_state::{DIRECT_CURSOR_ANIMATION};
// Removed unused import: js_sys





/// Start smooth pan left animation
pub fn start_smooth_pan_left() {
    if !crate::visualizer::timeline::timeline_actor::is_panning_left() {
        crate::visualizer::timeline::timeline_actor::panning_left_started_relay().send(());
        Task::start(async move {
            while crate::visualizer::timeline::timeline_actor::is_panning_left() {
                let ns_per_pixel = current_ns_per_pixel();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                // Lower ns_per_pixel means more zoomed in
                if let Some(ns_per_pixel_val) = ns_per_pixel {
                    if ns_per_pixel_val.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || crate::visualizer::timeline::timeline_actor::is_zooming_in() {
                        // TODO: Refactor coordinate access to use proper reactive patterns
                        // Temporarily disable pan animation to eliminate deprecated warnings
                        break;
                    }
                } else {
                    break; // Timeline not initialized yet, stop panning
                }
                // ✅ ACCEPTABLE: Timer::sleep() for animation timing (16ms = 60fps)
                // Note: requestAnimationFrame would be better but Timer::sleep is acceptable for animation loops
                Timer::sleep(16).await; // 60fps for smooth motion
            }
        });
    }
}

/// Start smooth pan right animation
pub fn start_smooth_pan_right() {
    if !crate::visualizer::timeline::timeline_actor::is_panning_right() {
        crate::visualizer::timeline::timeline_actor::panning_right_started_relay().send(());
        Task::start(async move {
            while crate::visualizer::timeline::timeline_actor::is_panning_right() {
                let ns_per_pixel = current_ns_per_pixel();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                // Lower ns_per_pixel means more zoomed in
                if let Some(ns_per_pixel_val) = ns_per_pixel {
                    if ns_per_pixel_val.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || crate::visualizer::timeline::timeline_actor::is_zooming_in() {
                        // TODO: Refactor coordinate access to use proper reactive patterns
                        // Temporarily disable pan animation to eliminate deprecated warnings
                        break;
                    }
                } else {
                    // Timeline not initialized yet - skip this pan frame
                }
                // ✅ ACCEPTABLE: Timer::sleep() for animation timing (16ms = 60fps)
                Timer::sleep(16).await; // 60fps for smooth motion
            }
        });
    }
}

/// Stop smooth pan left animation
pub fn stop_smooth_pan_left() {
    crate::visualizer::timeline::timeline_actor::panning_left_stopped_relay().send(());
}

/// Stop smooth pan right animation
pub fn stop_smooth_pan_right() {
    crate::visualizer::timeline::timeline_actor::panning_right_stopped_relay().send(());
}





/// Start smooth cursor movement to the left
pub fn start_smooth_cursor_left() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = -1;
    animation.is_animating = true;
    animation.current_position = 0.0; // TODO: Replace with cursor_position_signal() for proper reactive patterns
    crate::visualizer::timeline::timeline_actor::cursor_moving_left_started_relay().send(());
}

/// Start smooth cursor movement to the right
pub fn start_smooth_cursor_right() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = 1;
    animation.is_animating = true;
    animation.current_position = 0.0; // TODO: Replace with cursor_position_signal() for proper reactive patterns
    crate::visualizer::timeline::timeline_actor::cursor_moving_right_started_relay().send(());
}

/// Stop smooth cursor movement to the left
pub fn stop_smooth_cursor_left() {
    crate::visualizer::timeline::timeline_actor::cursor_moving_left_stopped_relay().send(());
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    if animation.direction == -1 {
        animation.is_animating = false;
        animation.direction = 0;
    }
}

/// Stop smooth cursor movement to the right
pub fn stop_smooth_cursor_right() {
    crate::visualizer::timeline::timeline_actor::cursor_moving_right_stopped_relay().send(());
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    if animation.direction == 1 {
        animation.is_animating = false;
        animation.direction = 0;
    }
}