use crate::visualizer::timeline::time_types::NsPerPixel;
use crate::visualizer::timeline::timeline_actor::{current_ns_per_pixel, current_viewport};
use std::collections::HashSet;

/// Timeline context object that provides domain access for utility functions
#[derive(Clone)]
pub struct TimelineContext {
    pub tracked_files: crate::tracked_files::TrackedFiles,
    pub selected_variables: crate::selected_variables::SelectedVariables,
    pub waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
}

impl TimelineContext {
    pub fn new(
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
        waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    ) -> Self {
        Self {
            tracked_files,
            selected_variables,
            waveform_timeline,
        }
    }
}

/// Minimum range calculation based on maximum zoom level
pub fn get_min_valid_range_ns(canvas_width: u32) -> u64 {
    NsPerPixel::MAX_ZOOM_IN.nanos() * canvas_width as u64
}


impl TimelineContext {
    /// Get the maximum timeline range (full file range regardless of zoom level)
    /// This behaves identically to get_current_timeline_range() when zoom level is 1.0 (unzoomed)
    pub fn get_maximum_timeline_range(&self) -> Option<(f64, f64)> {
        // Always get range from files containing selected variables only (ignore zoom level)
        let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
        let loaded_files: Vec<shared::WaveformFile> = tracked_files
            .iter()
            .filter_map(|tracked_file| match &tracked_file.state {
                shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
                _ => None,
            })
            .collect();

        // Get file paths that contain selected variables
        let selected_file_paths = self.get_selected_variable_file_paths();

        let mut min_time: f64 = f64::MAX;
        let mut max_time: f64 = f64::MIN;
        let mut has_valid_files = false;

        // If no variables are selected, use full file range for viewport initialization
        if selected_file_paths.is_empty() {
            let (file_min, file_max) = self.get_full_file_range();
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
                    if let (Some(file_min), Some(file_max)) = (
                        file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                        file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                    ) {
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
            return None; // Safe fallback
        }

        // Ensure minimum range for coordinate precision (but don't override valid microsecond ranges!)
        let file_range = max_time - min_time;
        let canvas_width = 800_u32; // DEFAULT_CANVAS_WIDTH fallback for calculations
        if file_range < get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0 {
            // Only enforce minimum for truly tiny ranges (< 1 nanosecond)
            let expanded_end = min_time + get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
            if expanded_end.is_finite() {
                return Some((min_time, expanded_end)); // Minimum 1 nanosecond range
            } else {
                return None; // Ultimate fallback
            }
        } else {
            let result = (min_time, max_time);
            // Maximum timeline range determined from file data
            return Some(result); // Use actual range, even if it's microseconds
        }
    }

    /// Get file paths that contain currently selected variables
    pub fn get_selected_variable_file_paths(&self) -> HashSet<String> {
        let selected_vars = self.selected_variables.variables_vec_signal.get_cloned();
        selected_vars
            .iter()
            .filter_map(|var| var.file_path())
            .collect()
    }

    /// Get full file range from loaded files (longest span prioritized)
    pub fn get_full_file_range(&self) -> (f64, f64) {
        // Calculate full file range directly from loaded files without selection dependency
        let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
        let loaded_files: Vec<shared::WaveformFile> = tracked_files
            .iter()
            .filter_map(|tracked_file| match &tracked_file.state {
                shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
                _ => None,
            })
            .collect();

        let mut min_time: f64 = f64::MAX;
        let mut max_time: f64 = f64::MIN;
        let mut has_valid_files = false;

        // Sort files by time span to ensure VCD files take priority over FST
        let mut file_candidates: Vec<_> = loaded_files
            .iter()
            .filter_map(|file| {
                if let (Some(file_min), Some(file_max)) = (
                    file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                    file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
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

        // Use ONLY the longest span file
        if let Some((_file, file_min, file_max, span)) = file_candidates.first() {
            let _is_long_timeline = *span > 100.0;

            // Use ONLY the longest file's range, don't combine with others
            min_time = *file_min;
            max_time = *file_max;
            has_valid_files = true;
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
            (0.0, 1.0) // Minimal 1-second range to prevent division by zero but not interfere with real data
        };

        validate_and_sanitize_range(raw_range.0, raw_range.1)
    }

    /// Get selected variables file range (longest span prioritized)
    pub fn get_selected_variables_file_range(&self) -> (f64, f64) {
        let selected_variables = self.selected_variables.variables_vec_signal.get_cloned();
        let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
        let loaded_files: Vec<shared::WaveformFile> = tracked_files
            .iter()
            .filter_map(|tracked_file| match &tracked_file.state {
                shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
                _ => None,
            })
            .collect();

        // Extract unique file paths from selected variables
        let mut selected_file_paths: HashSet<String> = HashSet::new();
        for var in selected_variables.iter() {
            if let Some(file_path) = var.file_path() {
                selected_file_paths.insert(file_path);
            }
        }

        // If no variables selected, fall back to all files
        if selected_file_paths.is_empty() {
            return self.get_full_file_range();
        }

        let mut min_time: f64 = f64::MAX;
        let mut max_time: f64 = f64::MIN;
        let mut has_valid_files = false;

        // Sort files by time span (longest first) to prioritize VCD over FST
        let mut file_candidates: Vec<_> = loaded_files
            .iter()
            .filter(|file| selected_file_paths.contains(&file.id))
            .filter_map(|file| {
                if let (Some(file_min), Some(file_max)) = (
                    file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                    file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
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

        // Use ONLY the longest span file, don't combine ranges from multiple files
        if let Some((_file, file_min, file_max, span_s)) = file_candidates.first() {
            if *span_s < 0.01 {
                // Short FST file detected
            } else if *span_s > 100.0 {
                // Long VCD file detected
            }

            // Use ONLY the longest file's range, don't combine with others
            min_time = *file_min;
            max_time = *file_max;
            has_valid_files = true;
        }

        if !has_valid_files || min_time == max_time {
            // No valid files with selected variables - fall back to full file range
            return self.get_full_file_range();
        } else {
            let result = (min_time, max_time);
            let _total_span = result.1 - result.0;
            result
        }
    }

    /// Get current timeline range for waveform rendering
    pub fn get_current_timeline_range(&self) -> Option<(f64, f64)> {
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

        let range_start = viewport.start.display_seconds();
        let range_end = viewport.end.display_seconds();

        // CRITICAL: Enforce minimum time range to prevent coordinate precision loss
        let canvas_width = 800_u32; // Fallback canvas width
        let min_zoom_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
        let current_range = range_end - range_start;

        // Validate range is sensible and has sufficient precision
        if range_end > range_start && range_start >= 0.0 {
            // ENHANCED: Additional validation for finite values
            if range_start.is_finite() && range_end.is_finite() {
                if current_range >= min_zoom_range {
                    return Some((range_start, range_end));
                } else if current_range > 0.0 {
                    // If zoom range is too narrow, expand it to minimum viable range
                    let range_center = (range_start + range_end) / 2.0;
                    let half_min_range = min_zoom_range / 2.0;
                    let expanded_start = (range_center - half_min_range).max(0.0);
                    let expanded_end = range_center + half_min_range;

                    // ENHANCED: Validate expanded range is finite
                    if expanded_start.is_finite()
                        && expanded_end.is_finite()
                        && expanded_end > expanded_start
                    {
                        return Some((expanded_start, expanded_end));
                    }
                }
            }
        }

        // STARTUP FIX: Prioritize actual file data when available
        let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
        let loaded_files: Vec<shared::WaveformFile> = tracked_files
            .iter()
            .filter_map(|tracked_file| match &tracked_file.state {
                shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
                _ => None,
            })
            .collect();

        if !loaded_files.is_empty() {
            // Use get_full_file_range() to get actual VCD file bounds
            let (full_file_min, full_file_max) = self.get_full_file_range();
            let file_span = full_file_max - full_file_min;

            // If we have substantial file data, use it immediately
            if file_span > 10.0 {
                return Some((full_file_min, full_file_max));
            }
        }

        // Fall back to selected variables approach
        let (r_key_min, r_key_max) = self.get_selected_variables_file_range();

        // Validate the range is sensible
        if r_key_max > r_key_min && r_key_min >= 0.0 && (r_key_max - r_key_min) > 0.001 {
            return Some((r_key_min, r_key_max));
        }

        None
    }
}

/// Validate and sanitize timeline range to prevent NaN propagation
pub fn validate_and_sanitize_range(start: f64, end: f64) -> (f64, f64) {
    // Check for NaN/Infinity in inputs
    if !start.is_finite() || !end.is_finite() {
        // Return minimal valid range instead of calling context method
        return (0.0, 1.0);
    }

    // Ensure proper ordering
    if start >= end {
        // Return minimal valid range instead of calling context method
        return (0.0, 1.0);
    }

    // Enforce minimum viable range based on maximum zoom level
    let range = end - start;
    let canvas_width = 800_u32; // DEFAULT_CANVAS_WIDTH fallback for calculations
    let min_valid_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
    if range < min_valid_range {
        let center = (start + end) / 2.0;
        let half_range = min_valid_range / 2.0;
        return (center - half_range, center + half_range);
    }

    // Range is valid
    (start, end)
}



