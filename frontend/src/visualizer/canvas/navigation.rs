use crate::visualizer::timeline::timeline_actor::set_cursor_position_seconds;
use zoon::*;
// Cursor position would be accessed through reactive signals instead of synchronous position functions
use js_sys;

// Constants for navigation timing and precision
const TRANSITION_NAVIGATION_DEBOUNCE_MS: u64 = 100; // 100ms debounce
const F64_PRECISION_TOLERANCE: f64 = 1e-15;

/// Get current time in nanoseconds for high-precision timing (WASM-compatible)
fn get_current_time_ns() -> u64 {
    // Use performance.now() in WASM which provides high-precision timestamps
    (js_sys::Date::now() * 1_000_000.0) as u64 // Convert milliseconds to nanoseconds
}

// collect_all_transitions function removed - uses deprecated global access patterns
// TODO: Implement transition collection using proper Actor+Relay architecture

/// Jump to the previous transition relative to current cursor position
pub fn jump_to_previous_transition(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST_NAVIGATION_TIME: AtomicU64 = AtomicU64::new(0);

    // Debounce rapid key presses to prevent precision issues
    let now = get_current_time_ns();
    let last_navigation = LAST_NAVIGATION_TIME.load(Ordering::Relaxed);
    if now - last_navigation < TRANSITION_NAVIGATION_DEBOUNCE_MS * 1_000_000 {
        return; // Still within debounce period
    }
    LAST_NAVIGATION_TIME.store(now, Ordering::Relaxed);

    // Validate timeline range exists before attempting transition jump
    // Timeline range validation now handled by MaximumTimelineRange Actor
    // Skip validation for now - proper implementation needs Actor+Relay integration

    // Transition jumping would be implemented through proper Actor+Relay events in waveform_timeline_domain
    let current_cursor = Some(0.0); // Fallback - proper implementation needs Actor+Relay event
    let transitions = Vec::<f64>::new(); // TODO: Implement transition collection using proper Actor+Relay architecture

    if transitions.is_empty() {
        return; // No transitions available
    }

    // Find the largest transition time that's less than current cursor
    let mut previous_transition: Option<f64> = None;

    for &transition_time in transitions.iter() {
        if transition_time < current_cursor.unwrap_or(0.0) - F64_PRECISION_TOLERANCE {
            // f64 precision tolerance
            previous_transition = Some(transition_time);
        } else {
            break; // Transitions are sorted, so we can stop here
        }
    }

    if let Some(prev_time) = previous_transition {
        // Jump to previous transition
        set_cursor_position_seconds(timeline, prev_time);
        // Cursor synchronization would use dedicated relay events in WaveformTimeline Actor
        // timeline.cursor_synced_relay.send((prev_time, prev_time));  // (current, target)
        // Jumped to previous transition
    } else if !transitions.is_empty() {
        // If no previous transition, wrap to the last transition
        let last_transition = transitions[transitions.len() - 1];
        set_cursor_position_seconds(timeline, last_transition);
        // Cursor synchronization would use dedicated relay events in WaveformTimeline Actor
        // timeline.cursor_synced_relay.send((last_transition, last_transition));  // (current, target)
        // Wrapped to last transition
    }
}

/// Jump to the next transition relative to current cursor position
pub fn jump_to_next_transition(timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST_NAVIGATION_TIME: AtomicU64 = AtomicU64::new(0);

    // Debounce rapid key presses to prevent precision issues
    let now = get_current_time_ns();
    let last_navigation = LAST_NAVIGATION_TIME.load(Ordering::Relaxed);
    if now - last_navigation < TRANSITION_NAVIGATION_DEBOUNCE_MS * 1_000_000 {
        return; // Still within debounce period
    }
    LAST_NAVIGATION_TIME.store(now, Ordering::Relaxed);

    // Validate timeline range exists before attempting transition jump
    // Timeline range validation now handled by MaximumTimelineRange Actor
    // Skip validation for now - proper implementation needs Actor+Relay integration

    // Transition jumping would be implemented through proper Actor+Relay events in waveform_timeline_domain
    let current_cursor = Some(0.0); // Fallback - proper implementation needs Actor+Relay event
    let transitions = Vec::<f64>::new(); // TODO: Implement transition collection using proper Actor+Relay architecture

    if transitions.is_empty() {
        return; // No transitions available
    }

    // Find the smallest transition time that's greater than current cursor
    let next_transition = transitions
        .iter()
        .find(|&&transition_time| {
            transition_time > current_cursor.unwrap_or(0.0) + F64_PRECISION_TOLERANCE
        }) // f64 precision tolerance
        .copied();

    if let Some(next_time) = next_transition {
        // Jump to next transition
        set_cursor_position_seconds(timeline, next_time);
        // Cursor synchronization would use dedicated relay events in WaveformTimeline Actor
        // timeline.cursor_synced_relay.send((next_time, next_time));  // (current, target)
        // Jumped to next transition
    } else if !transitions.is_empty() {
        // If no next transition, wrap to the first transition
        let first_transition = transitions[0];
        set_cursor_position_seconds(timeline, first_transition);
        // Cursor synchronization would use dedicated relay events in WaveformTimeline Actor
        // timeline.cursor_synced_relay.send((first_transition, first_transition));  // (current, target)
        // Wrapped to first transition
    }
}
