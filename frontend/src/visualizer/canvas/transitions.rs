use zoon::*;
use crate::state::LOADED_FILES;
use crate::platform::{Platform, CurrentPlatform};
use shared::{UpMsg, SignalTransitionQuery}; // Removed unused: SelectedVariable, SignalTransition
// use std::collections::HashSet; // Unused




/// Request real signal transitions from backend
#[allow(dead_code)] // Backend communication function - preserve for future backend integration
pub fn request_signal_transitions_from_backend(file_path: &str, scope_path: &str, variable_name: &str, _time_range: (f32, f32)) {
    
    crate::debug_utils::debug_conditional(&format!("Requesting signal transitions for {}/{}", scope_path, variable_name));
    
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
                loaded_file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0).unwrap_or(0.0),
                loaded_file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0).unwrap_or(1000.0)  // Use higher fallback to avoid premature filler
            )
        } else {
            // Don't make request if file isn't loaded yet - prevents race condition
            crate::debug_utils::debug_conditional(&format!("FILE NOT LOADED YET - cannot request transitions for {}", file_path));
            return;
        }
    };
    
    let message = UpMsg::QuerySignalTransitions {
        file_path: file_path.to_string(),
        signal_queries: vec![query],
        time_range: ((file_min * 1_000_000_000.0) as u64, (file_max * 1_000_000_000.0) as u64), // Request entire file range to ensure all transitions available
    };
    
    // Send real backend request
    Task::start(async move {
        let _ = CurrentPlatform::send_message(message).await;
    });
}

/// Trigger canvas redraw when new signal data arrives
#[allow(dead_code)] // Canvas trigger function - preserve for canvas integration
pub fn trigger_canvas_redraw() {
    // Call the global wrapper that has access to the renderer instance
    crate::visualizer::canvas::waveform_canvas::trigger_canvas_redraw_global();
}

/// Extract unique file paths from selected variables
#[allow(dead_code)] // Helper function - preserve for variable path processing
pub fn get_selected_variable_file_paths() -> std::collections::HashSet<String> {
    let selected_vars = crate::actors::selected_variables::current_variables();
    let mut file_paths = std::collections::HashSet::new();
    
    // ITERATION 5: Track file path consistency between calls (simplified approach)
    use std::sync::OnceLock;
    // ❌ ANTIPATTERN: Static state tracking - TODO: Use Cache Current Values pattern in Actor loop
    #[deprecated(note = "Replace static file path tracking with Cache Current Values pattern inside Actor loop")]
    static PREVIOUS_FILE_PATHS: OnceLock<std::sync::Mutex<Option<std::collections::HashSet<String>>>> = OnceLock::new();
    
    for var in selected_vars.iter() {
        // Parse unique_id: "file_path|scope|variable"
        if let Some(file_path) = var.unique_id.split('|').next() {
            file_paths.insert(file_path.to_string());
        }
    }
    
    // ITERATION 5: Check if file paths changed from previous call
    let _file_paths_vec: Vec<String> = file_paths.iter().cloned().collect();
    
    let mutex = PREVIOUS_FILE_PATHS.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(mut prev) = mutex.lock() {
        if let Some(prev_paths) = &*prev {
            if file_paths != *prev_paths {
            } else {
            }
        } else {
        }
        *prev = Some(file_paths.clone());
    }
    
    file_paths
}

/// Clear transition request tracking for removed variables (simplified)
pub fn _clear_transition_tracking_for_variable(_unique_id: &str) {
    // This functionality has been simplified since complex deduplication has been removed
    // Modern batching system handles request efficiency automatically
}

/// Clear all transition request tracking (simplified)
pub fn _clear_all_transition_tracking() {
    // This functionality has been simplified since complex deduplication has been removed
    // Modern batching system handles request efficiency automatically
}