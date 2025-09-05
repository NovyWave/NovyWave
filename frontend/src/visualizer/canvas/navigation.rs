use zoon::*;
use crate::visualizer::timeline::timeline_actor::{
    current_cursor_position_seconds, set_cursor_position_seconds, 
    set_ns_per_pixel_if_changed, set_viewport_if_changed, set_zoom_center_follow_mouse
};
use crate::visualizer::timeline::time_types::{TimeNs, NsPerPixel, Viewport};
use crate::visualizer::state::canvas_state::{DIRECT_CURSOR_ANIMATION, LAST_TRANSITION_NAVIGATION_TIME};
use js_sys;

// Constants for navigation timing and precision
const TRANSITION_NAVIGATION_DEBOUNCE_MS: u64 = 100; // 100ms debounce
const F64_PRECISION_TOLERANCE: f64 = 1e-15;

/// Get current time in nanoseconds for high-precision timing (WASM-compatible)
fn get_current_time_ns() -> u64 {
    // Use performance.now() in WASM which provides high-precision timestamps
    (js_sys::Date::now() * 1_000_000.0) as u64  // Convert milliseconds to nanoseconds
}

/// Collect all transitions from currently selected variables and sort by time
pub fn collect_all_transitions() -> Vec<f64> {
    let selected_vars = crate::actors::selected_variables::current_variables();
    let mut all_transitions = Vec::new();
    
    for var in selected_vars.iter() {
        // Parse unique_id: "/path/file.ext|scope|variable"
        let parts: Vec<&str> = var.unique_id.split('|').collect();
        if parts.len() < 3 {
            continue;
        }
        
        let file_path = parts[0];
        let scope_path = parts[1]; 
        let variable_name = parts[2];
        
        // Create cache key for transition data
        let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
        
        // Get transitions from cache
        if let Some(transitions) = crate::visualizer::timeline::timeline_service::UnifiedTimelineService::get_raw_transitions(&cache_key) {
            // Extract time points and convert to f64
            for transition in &transitions {
                // Only include transitions within reasonable bounds
                if transition.time_ns as f64 / 1_000_000_000.0 >= 0.0 {
                    all_transitions.push(transition.time_ns as f64 / 1_000_000_000.0);
                }
            }
        }
    }
    
    // Remove duplicates and sort by time
    all_transitions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Use f32-appropriate tolerance instead of f64 precision
    // f64 precision eliminates the tolerance issues we had with f32
    all_transitions.dedup_by(|a, b| (*a - *b).abs() < F64_PRECISION_TOLERANCE); // Remove near-duplicate times with f64 precision
    
    all_transitions
}

/// Jump to the previous transition relative to current cursor position
pub fn jump_to_previous_transition() {
    // Debounce rapid key presses to prevent precision issues
    let now = get_current_time_ns();
    let last_navigation = LAST_TRANSITION_NAVIGATION_TIME.get();
    if now - last_navigation < TRANSITION_NAVIGATION_DEBOUNCE_MS * 1_000_000 {
        return; // Still within debounce period
    }
    LAST_TRANSITION_NAVIGATION_TIME.set_neq(now);
    
    // Validate timeline range exists before attempting transition jump
    if super::get_current_timeline_range().is_none() {
        return; // No valid timeline range available
    }
    
    let current_cursor = current_cursor_position_seconds();
    let transitions = collect_all_transitions();
    
    if transitions.is_empty() {
        return; // No transitions available
    }
    
    // Find the largest transition time that's less than current cursor
    let mut previous_transition: Option<f64> = None;
    
    for &transition_time in transitions.iter() {
        if transition_time < current_cursor.unwrap_or(0.0) - F64_PRECISION_TOLERANCE { // f64 precision tolerance
            previous_transition = Some(transition_time);
        } else {
            break; // Transitions are sorted, so we can stop here
        }
    }
    
    if let Some(prev_time) = previous_transition {
        // Jump to previous transition
        set_cursor_position_seconds(prev_time);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = prev_time;
        animation.target_position = prev_time;
        crate::debug_utils::debug_conditional(&format!("Jumped to previous transition at {:.9}s", prev_time));
    } else if !transitions.is_empty() {
        // If no previous transition, wrap to the last transition
        let last_transition = transitions[transitions.len() - 1];
        set_cursor_position_seconds(last_transition);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = last_transition;
        animation.target_position = last_transition;
        crate::debug_utils::debug_conditional(&format!("Wrapped to last transition at {:.9}s", last_transition));
    }
}

/// Jump to the next transition relative to current cursor position
pub fn jump_to_next_transition() {
    // Debounce rapid key presses to prevent precision issues
    let now = get_current_time_ns();
    let last_navigation = LAST_TRANSITION_NAVIGATION_TIME.get();
    if now - last_navigation < TRANSITION_NAVIGATION_DEBOUNCE_MS * 1_000_000 {
        return; // Still within debounce period
    }
    LAST_TRANSITION_NAVIGATION_TIME.set_neq(now);
    
    // Validate timeline range exists before attempting transition jump
    if super::get_current_timeline_range().is_none() {
        return; // No valid timeline range available
    }
    
    let current_cursor = current_cursor_position_seconds();
    let transitions = collect_all_transitions();
    
    if transitions.is_empty() {
        return; // No transitions available
    }
    
    // Find the smallest transition time that's greater than current cursor
    let next_transition = transitions.iter()
        .find(|&&transition_time| transition_time > current_cursor.unwrap_or(0.0) + F64_PRECISION_TOLERANCE) // f64 precision tolerance
        .copied();
    
    if let Some(next_time) = next_transition {
        // Jump to next transition
        set_cursor_position_seconds(next_time);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = next_time;
        animation.target_position = next_time;
        crate::debug_utils::debug_conditional(&format!("Jumped to next transition at {:.9}s", next_time));
    } else if !transitions.is_empty() {
        // If no next transition, wrap to the first transition
        let first_transition = transitions[0];
        set_cursor_position_seconds(first_transition);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = first_transition;
        animation.target_position = first_transition;
        crate::debug_utils::debug_conditional(&format!("Wrapped to first transition at {:.9}s", first_transition));
    }
}

/// Reset zoom to fit all data in view (recovery function for broken zoom states)
pub fn reset_zoom_to_fit_all() {
    zoon::println!("🔧 RESET_ZOOM_TO_FIT_ALL called - analyzing mixed file ranges...");
    
    // Reset zoom to 1x
    set_ns_per_pixel_if_changed(NsPerPixel::MEDIUM_ZOOM);
    
    // Get range for files with selected variables only
    let (file_min, file_max) = crate::visualizer::canvas::timeline::get_selected_variables_file_range();
    
    // 🔧 DEBUG: Check for mixed file ranges affecting zoom
    let span = file_max - file_min;
    
    let viewport = Viewport::new(
        TimeNs::from_external_seconds(file_min),
        TimeNs::from_external_seconds(file_max)
    );
    set_viewport_if_changed(viewport);
    
    // Reset cursor to a reasonable position
    let middle_time = (file_min + file_max) / 2.0;
    set_cursor_position_seconds(middle_time);
    
    // Synchronize direct animation to prevent jumps when using Q/E after zoom reset
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.current_position = middle_time as f64;
    animation.target_position = middle_time as f64;
    
    // Debug logging to verify correct range calculation
    let selected_variables = crate::actors::selected_variables::current_variables();
    crate::debug_utils::debug_conditional("=== ZOOM RESET DEBUG ===");
    crate::debug_utils::debug_conditional(&format!("Selected variables count: {}", selected_variables.len()));
    for var in selected_variables.iter() {
        crate::debug_utils::debug_conditional(&format!("  Variable: {}", var.unique_id));
    }
    crate::debug_utils::debug_conditional(&format!("Reset range: {:.9}s to {:.9}s (span: {:.9}s)", file_min, file_max, file_max - file_min));
    crate::debug_utils::debug_conditional(&format!("Cursor positioned at: {:.9}s", middle_time));
}

