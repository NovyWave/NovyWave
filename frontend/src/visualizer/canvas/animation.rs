use zoon::*;
use crate::visualizer::timeline::timeline_actor::{
    current_ns_per_pixel
};
// Note: Some synchronous operations maintained for performance in animation loops
use crate::visualizer::timeline::time_types::NsPerPixel;





/// Start smooth pan left animation
pub fn start_smooth_pan_left(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    if !crate::visualizer::timeline::timeline_actor::is_panning_left() {
        crate::visualizer::timeline::timeline_actor::panning_left_started_relay(timeline).send(());
        Task::start(async move {
            while crate::visualizer::timeline::timeline_actor::is_panning_left() {
                let ns_per_pixel = current_ns_per_pixel();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                // Lower ns_per_pixel means more zoomed in
                if let Some(ns_per_pixel_val) = ns_per_pixel {
                    if ns_per_pixel_val.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || crate::visualizer::timeline::timeline_actor::is_zooming_in() {
                        // Coordinate access would be refactored to use proper reactive patterns instead of synchronous position access
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
pub fn start_smooth_pan_right(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    if !crate::visualizer::timeline::timeline_actor::is_panning_right() {
        crate::visualizer::timeline::timeline_actor::panning_right_started_relay(timeline).send(());
        Task::start(async move {
            while crate::visualizer::timeline::timeline_actor::is_panning_right() {
                let ns_per_pixel = current_ns_per_pixel();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                // Lower ns_per_pixel means more zoomed in
                if let Some(ns_per_pixel_val) = ns_per_pixel {
                    if ns_per_pixel_val.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || crate::visualizer::timeline::timeline_actor::is_zooming_in() {
                        // Coordinate access would be refactored to use proper reactive patterns instead of synchronous position access
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
pub fn stop_smooth_pan_left(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    crate::visualizer::timeline::timeline_actor::panning_left_stopped_relay(timeline).send(());
}

/// Stop smooth pan right animation
pub fn stop_smooth_pan_right(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    crate::visualizer::timeline::timeline_actor::panning_right_stopped_relay(timeline).send(());
}





/// Start smooth cursor movement to the left
pub fn start_smooth_cursor_left(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    crate::visualizer::timeline::timeline_actor::cursor_moving_left_started_relay(timeline).send(());
    // Animation system would use cursor_animation_started relay to handle direction and animation state events
}

/// Start smooth cursor movement to the right
pub fn start_smooth_cursor_right(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    crate::visualizer::timeline::timeline_actor::cursor_moving_right_started_relay(timeline).send(());
    // Animation system would use cursor_animation_started relay to handle direction and animation state events
}

/// Stop smooth cursor movement to the left
pub fn stop_smooth_cursor_left(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    crate::visualizer::timeline::timeline_actor::cursor_moving_left_stopped_relay(timeline).send(());
    // Animation system would use cursor_animation_stopped relay to handle animation state transitions properly
}

/// Stop smooth cursor movement to the right
pub fn stop_smooth_cursor_right(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    crate::visualizer::timeline::timeline_actor::cursor_moving_right_stopped_relay(timeline).send(());
    // Animation system would use cursor_animation_stopped relay to handle animation state transitions properly
}