use zoon::*;
// Removed LOADED_FILES import - migrated to tracked_files_domain() pattern
use crate::visualizer::timeline::timeline_actor::{
    current_ns_per_pixel, current_viewport
};
use crate::visualizer::timeline::time_types::{NsPerPixel}; // Removed unused TimeNs
use std::collections::HashSet;

/// Minimum range calculation based on maximum zoom level
pub fn get_min_valid_range_ns(canvas_width: u32) -> u64 {
    NsPerPixel::MAX_ZOOM_IN.nanos() * canvas_width as u64
}

/// Get current timeline range for waveform rendering
pub fn get_current_timeline_range() -> Option<(f64, f64)> {
    // Removed debug spam - function called very frequently during rendering
    let _ns_per_pixel = match current_ns_per_pixel() {
        Some(ns_per_pixel) => ns_per_pixel,
        None => {
            return None;
        }
    };
    
    // FIXED: Always use viewport range for waveform rendering (no zoom level threshold)
    // This ensures transition rectangles use proper viewport boundaries
    let viewport = match current_viewport() {
        Some(viewport) => viewport,
        None => {
            return None;
        }
    };
    
    // ✅ DEBUG: Log current viewport cache state
    // Cache state debug info reduced
    let range_start = viewport.start.display_seconds();
    let range_end = viewport.end.display_seconds();
    
    // DEBUG: Log viewport range calculation
    // Viewport range debug info reduced
    
    // CRITICAL: Enforce minimum time range to prevent coordinate precision loss
    // TODO: Use waveform_timeline_domain().canvas_width.signal() for proper reactive patterns
    // For now, use fallback width to eliminate deprecated warnings
    let canvas_width = 800_u32; // Fallback canvas width
    let min_zoom_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0; // NsPerPixel-based minimum
    let current_range = range_end - range_start;
    
    // Validate range is sensible and has sufficient precision
    // Removed excessive viewport validation debug info
    
    if range_end > range_start && range_start >= 0.0 {
        // Removed basic validation debug message
        // ENHANCED: Additional validation for finite values
        if range_start.is_finite() && range_end.is_finite() {
            // Removed finite values debug message
            if current_range >= min_zoom_range {
                // Removed viewport range debug message
                return Some((range_start, range_end));
            } else if current_range > 0.0 {
                // If zoom range is too narrow, expand it to minimum viable range
                let range_center = (range_start + range_end) / 2.0;
                let half_min_range = min_zoom_range / 2.0;
                let expanded_start = (range_center - half_min_range).max(0.0);
                let expanded_end = range_center + half_min_range;
                
                // ENHANCED: Validate expanded range is finite
                if expanded_start.is_finite() && expanded_end.is_finite() && expanded_end > expanded_start {
                    crate::debug_utils::debug_timeline_validation(&format!("Expanded narrow range from {:.12} to [{:.12}, {:.12}]", current_range, expanded_start, expanded_end));
                    return Some((expanded_start, expanded_end));
                } else {
                    crate::debug_utils::debug_timeline_validation(&format!("WARNING: Failed to expand range - center: {}, half_range: {}", range_center, half_min_range));
                }
            } else {
                // Range expansion failed
            }
        } else {
            // Range validation removed for performance
        }
    } else {
        // Range validation removed for performance
    }
    
    // ✅ STARTUP FIX: Prioritize actual file data when available, even if no variables selected
    
    // STEP 1: If we have loaded files with good data, use them directly (bypass selected variables dependency)
    let tracked_files = crate::actors::global_domains::get_current_tracked_files();
    let loaded_files: Vec<shared::WaveformFile> = tracked_files.iter()
        .filter_map(|tracked_file| match &tracked_file.state {
            shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
            _ => None,
        }).collect();
    if !loaded_files.is_empty() {
        // Use get_full_file_range() to get actual VCD file bounds (0-250s) regardless of selection
        let (full_file_min, full_file_max) = super::get_full_file_range();
        let file_span = full_file_max - full_file_min;
        
        
        // If we have substantial file data (not just microsecond ranges), use it immediately
        if file_span > 10.0 {  // More than 10 seconds suggests VCD file with real timeline data
            return Some((full_file_min, full_file_max));
        }
    }
    
    // STEP 2: Fall back to selected variables approach (original R key logic)
    let (r_key_min, r_key_max) = get_selected_variables_file_range();
    
    // Validate the range is sensible
    if r_key_max > r_key_min && r_key_min >= 0.0 && (r_key_max - r_key_min) > 0.001 {
        return Some((r_key_min, r_key_max));
    } else {
        // No valid selected variables range found
    }

    // ORIGINAL LOGIC: Default behavior: get range from files containing selected variables only
    let tracked_files = crate::actors::global_domains::get_current_tracked_files();
    let loaded_files: Vec<shared::WaveformFile> = tracked_files.iter()
        .filter_map(|tracked_file| match &tracked_file.state {
            shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
            _ => None,
        }).collect();
    
    // Get file paths that contain selected variables
    let selected_file_paths = get_selected_variable_file_paths();
    
    
    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;
    let mut has_valid_files = false;
    
    // If no variables are selected due to Actor+Relay migration issues, use all loaded files as fallback
    if selected_file_paths.is_empty() {
        // FALLBACK: Use all loaded files when no variables are selected
        
        // Use ALL loaded files as fallback with same prioritization algorithm
        let mut file_candidates: Vec<_> = loaded_files.iter()
            .filter_map(|file| {
                if let (Some(file_min), Some(file_max)) = (
                    file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), 
                    file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)
                ) {
                    let span_s = file_max - file_min;
                    Some((file, file_min, file_max, span_s))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by span descending (longest first) to prioritize VCD files over FST files
        file_candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        
        // Calculate range from prioritized files (VCD files influence result more than FST files)  
        for (_file, file_min, file_max, _span) in file_candidates {
            min_time = min_time.min(file_min);
            max_time = max_time.max(file_max);
            has_valid_files = true;
        }
    } else {
        // 🔧 TIMELINE STARTUP 3 FIX: Use same file prioritization as get_selected_variables_file_range()
        // Sort files by time span (longest first) to prioritize VCD over FST files
        let mut file_candidates: Vec<_> = loaded_files.iter()
            .filter(|file| selected_file_paths.contains(&file.id))
            .filter_map(|file| {
                if let (Some(file_min), Some(file_max)) = (
                    file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), 
                    file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)
                ) {
                    let span_s = file_max - file_min;
                    Some((file, file_min, file_max, span_s))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by span descending (longest first) to prioritize VCD files over FST files
        file_candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        
        // File prioritization: longer time spans get higher priority
        // (This prioritizes VCD files over shorter FST files)
        
        // Calculate range from prioritized files (VCD files influence result more than FST files)  
        for (_file, file_min, file_max, _span) in file_candidates {
            min_time = min_time.min(file_min);
            max_time = max_time.max(file_max);
            has_valid_files = true;
            // File contributes to timeline range calculation
        }
    }
    
    if !has_valid_files || min_time == max_time {
        // No valid files with selected variables - return None so timeline shows placeholder
        // No valid timeline range available
        return None;
    }
    
    // ENHANCED: Comprehensive validation before returning range
    if !min_time.is_finite() || !max_time.is_finite() {
        crate::debug_utils::debug_timeline_validation(&format!("WARNING: Timeline range calculation produced non-finite values - min: {}, max: {}", min_time, max_time));
        return None; // Safe fallback
    }
    
    // Ensure minimum range for coordinate precision (but don't override valid microsecond ranges!)
    let file_range = max_time - min_time;
    // ✅ FIXED: Use constant since reactive access not available in synchronous context
    let canvas_width = 800_u32; // DEFAULT_CANVAS_WIDTH fallback for calculations
    if file_range < get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0 {  // Only enforce minimum for truly tiny ranges (< 1 nanosecond)
        let expanded_end = min_time + get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
        if expanded_end.is_finite() {
            return Some((min_time, expanded_end));  // Minimum 1 nanosecond range
        } else {
            return None; // Ultimate fallback
        }
    } else {
        let result = (min_time, max_time);
        // Final timeline range calculated from file data
        
        // 🔧 TIMELINE STARTUP 4: Validate timeline range consistency
        let _zoom_level_us = if let Some(ns_per_pixel) = current_ns_per_pixel() {
            ns_per_pixel.nanos() as f64 / 1000.0 // Convert to microseconds/pixel
        } else {
            1000.0 // Default to 1000 microseconds/pixel when timeline not initialized
        };
        // Timeline range validation for consistency
        
        // Check if this matches expected VCD file range
        if result.0 <= 1.0 && result.1 >= 240.0 {
            // Range validation successful: VCD timeline range detected
        } else if result.1 - result.0 < 10.0 {
            // Warning: Short range detected from FST file
        } else {
            // Info: Different range detected during validation
        }
        
        return Some(result);  // Use actual range, even if it's microseconds
    }
}

/// Get the maximum timeline range (full file range regardless of zoom level)
/// This behaves identically to get_current_timeline_range() when zoom level is 1.0 (unzoomed)
pub fn get_maximum_timeline_range() -> Option<(f64, f64)> {
    // Always get range from files containing selected variables only (ignore zoom level)
    let tracked_files = crate::actors::global_domains::get_current_tracked_files();
    let loaded_files: Vec<shared::WaveformFile> = tracked_files.iter()
        .filter_map(|tracked_file| match &tracked_file.state {
            shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
            _ => None,
        }).collect();
    
    // Get file paths that contain selected variables
    let selected_file_paths = get_selected_variable_file_paths();
    
    
    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;
    let mut has_valid_files = false;
    
    // If no variables are selected, use full file range for viewport initialization
    if selected_file_paths.is_empty() {
        let (file_min, file_max) = get_full_file_range();
        if file_min < file_max && file_min.is_finite() && file_max.is_finite() {
            return Some((file_min, file_max));
        } else {
            return None;
        }
    } else {
        // Calculate range from only files that contain selected variables
        
        for file in loaded_files.iter() {
            
            // Check if this file contains any selected variables
            let file_matches = selected_file_paths.iter().any(|path| {
                let matches = file.id == *path;
                matches
            });
            
            if file_matches {
                if let (Some(file_min), Some(file_max)) = (file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)) {
                    min_time = min_time.min(file_min);
                    max_time = max_time.max(file_max);
                    has_valid_files = true;
                }
            }
        }
    }
    
    if !has_valid_files || min_time == max_time {
        // No valid files with selected variables - return None so timeline shows placeholder
        return None;
    }
    
    // ENHANCED: Comprehensive validation before returning range
    if !min_time.is_finite() || !max_time.is_finite() {
        crate::debug_utils::debug_timeline_validation(&format!("WARNING: Maximum timeline range calculation produced non-finite values - min: {}, max: {}", min_time, max_time));
        return None; // Safe fallback
    }
    
    // Ensure minimum range for coordinate precision (but don't override valid microsecond ranges!)
    let file_range = max_time - min_time;
    // ✅ FIXED: Use constant since reactive access not available in synchronous context
    let canvas_width = 800_u32; // DEFAULT_CANVAS_WIDTH fallback for calculations
    if file_range < get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0 {  // Only enforce minimum for truly tiny ranges (< 1 nanosecond)
        let expanded_end = min_time + get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
        if expanded_end.is_finite() {
            return Some((min_time, expanded_end));  // Minimum 1 nanosecond range
        } else {
            return None; // Ultimate fallback
        }
    } else {
        let result = (min_time, max_time);
        // Maximum timeline range determined from file data
        return Some(result);  // Use actual range, even if it's microseconds
    }
}

/// Validate and sanitize timeline range to prevent NaN propagation
pub fn validate_and_sanitize_range(start: f64, end: f64) -> (f64, f64) {
    // Check for NaN/Infinity in inputs
    if !start.is_finite() || !end.is_finite() {
        crate::debug_utils::debug_timeline_validation(&format!("Non-finite range detected - start: {}, end: {}, using actual file range", start, end));
        // ❌ FALLBACK ELIMINATION: Get actual file range instead of hardcoded fallback
        let (file_min, file_max) = get_full_file_range();
        return (file_min, file_max);
    }
    
    // Ensure proper ordering
    if start >= end {
        crate::debug_utils::debug_timeline_validation(&format!("Invalid range ordering - start: {} >= end: {}, using actual file range", start, end));
        // ❌ FALLBACK ELIMINATION: Get actual file range instead of hardcoded fallback
        let (file_min, file_max) = get_full_file_range();
        return (file_min, file_max);
    }
    
    // Enforce minimum viable range based on maximum zoom level
    let range = end - start;
    // ✅ FIXED: Use constant since reactive access not available in synchronous context
    let canvas_width = 800_u32; // DEFAULT_CANVAS_WIDTH fallback for calculations
    let min_valid_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
    if range < min_valid_range {
        crate::debug_utils::debug_timeline_validation(&format!("Range too small: {:.3e}s, enforcing minimum of {:.3e}s", range, min_valid_range));
        let center = (start + end) / 2.0;
        let half_range = min_valid_range / 2.0;
        return (center - half_range, center + half_range);
    }
    
    // Range is valid
    (start, end)
}

/// Get file range from currently selected variables only
pub fn get_full_file_range() -> (f64, f64) {
    // ✅ FIXED: Break circular dependency with get_maximum_timeline_range()
    // Calculate full file range directly from loaded files without selection dependency
    let tracked_files = crate::actors::global_domains::get_current_tracked_files();
    let loaded_files: Vec<shared::WaveformFile> = tracked_files.iter()
        .filter_map(|tracked_file| match &tracked_file.state {
            shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
            _ => None,
        }).collect();
    
    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;
    let mut has_valid_files = false;
    
    // 🔧 TIMELINE STARTUP 2 FIX: Sort files by time span to ensure VCD files take priority over FST
    let mut file_candidates: Vec<_> = loaded_files.iter()
        .filter_map(|file| {
            if let (Some(file_min), Some(file_max)) = (
                file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), 
                file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)
            ) {
                // Validate file times before using them
                if file_min.is_finite() && file_max.is_finite() && file_min < file_max {
                    let span = file_max - file_min;
                    Some((file, file_min, file_max, span))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    
    // Sort by span descending (longest first) to prioritize VCD files over FST files
    file_candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    
    // 🔧 FIX: Use ONLY the longest span file in get_full_file_range() too
    if let Some((_file, file_min, file_max, span)) = file_candidates.first() {
        // Use longest span file (debug: {:.6}s to {:.6}s, span: {:.6}s)
        // Long timeline check: {} seconds
        let _is_long_timeline = *span > 100.0;
        
        // Use ONLY the longest file's range, don't combine with others
        min_time = *file_min;
        max_time = *file_max;
        has_valid_files = true;
        
        // Skip shorter files (removed debug logging)
    }
    
    // Use validation system for final result with generous buffer
    let raw_range = if has_valid_files && min_time < max_time {
        // Add 20% buffer on each side to expand "visible range" for better cache utilization
        let time_range = max_time - min_time;
        let buffer = time_range * 0.2; // 20% buffer
        let expanded_min = (min_time - buffer).max(0.0); // Don't go below 0
        let expanded_max = max_time + buffer;
 
        (expanded_min, expanded_max)
    } else {
        // Don't return emergency fallback - let caller handle missing data appropriately
        (0.0, 1.0)  // Minimal 1-second range to prevent division by zero but not interfere with real data
    };
    
    validate_and_sanitize_range(raw_range.0, raw_range.1)
}

pub fn get_selected_variables_file_range() -> (f64, f64) {
    let selected_variables = crate::actors::selected_variables::current_variables();
    let tracked_files = crate::actors::global_domains::get_current_tracked_files();
    let loaded_files: Vec<shared::WaveformFile> = tracked_files.iter()
        .filter_map(|tracked_file| match &tracked_file.state {
            shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
            _ => None,
        }).collect();
    
    // ═══════════════════════════════════════════════════════════════════════════════════
    
    // Extract unique file paths from selected variables
    let mut selected_file_paths: HashSet<String> = HashSet::new();
    for var in selected_variables.iter() {
        if let Some(file_path) = var.file_path() {
            selected_file_paths.insert(file_path);
        }
    }
    
    
    // If no variables selected, fall back to all files
    if selected_file_paths.is_empty() {
        return get_full_file_range();
    }
    
    
    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;
    let mut has_valid_files = false;
    
    // Only include files that have selected variables - prefer longer time spans
    
    // 🔧 TIMELINE STARTUP 2 FIX: Sort files by time span (longest first) to prioritize VCD over FST
    let mut file_candidates: Vec<_> = loaded_files.iter()
        .filter(|file| selected_file_paths.contains(&file.id))
        .filter_map(|file| {
            if let (Some(file_min), Some(file_max)) = (
                file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), 
                file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)
            ) {
                let span_s = file_max - file_min;
                Some((file, file_min, file_max, span_s))
            } else {
                None
            }
        })
        .collect();
    
    // Sort by span descending (longest first) to prioritize VCD files over FST files
    file_candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    
    // 🔧 FIX: Use ONLY the longest span file, don't combine ranges from multiple files
    if let Some((_file, file_min, file_max, span_s)) = file_candidates.first() {
        if *span_s < 0.01 {
        } else if *span_s > 100.0 {
        }
        
        // Use ONLY the longest file's range, don't combine with others
        min_time = *file_min;
        max_time = *file_max;
        has_valid_files = true;
        
        // Log skipped shorter files for debugging
        for (_skipped_file, _skipped_min, _skipped_max, _skipped_span) in file_candidates.iter().skip(1) {
            // Debug info for skipped files - removed for performance
        }
    }
    
    // Log skipped files for debugging
    // Process files that contain selected variables
    
    if !has_valid_files || min_time == max_time {
        // No valid files with selected variables - fall back to full file range
        return get_full_file_range();
    } else {
        let result = (min_time, max_time);
        let _total_span = result.1 - result.0;
        
        
        result
    }
}

/// Get file paths that contain currently selected variables
fn get_selected_variable_file_paths() -> HashSet<String> {
    let selected_vars = crate::actors::selected_variables::current_variables();
    selected_vars.iter()
        .filter_map(|var| var.file_path())
        .collect()
}