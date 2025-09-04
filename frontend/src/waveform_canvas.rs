use zoon::*;
use fast2d;
use crate::state::{LOADED_FILES, IS_LOADING, IS_ZOOMING_IN, IS_ZOOMING_OUT, IS_PANNING_LEFT, IS_PANNING_RIGHT, 
    IS_CURSOR_MOVING_LEFT, IS_CURSOR_MOVING_RIGHT, ZOOM_CENTER_NS, MOUSE_TIME_NS, MOUSE_X_POSITION};
use crate::actors::waveform_timeline::{
    zoom_in_started_relay, zoom_center_ns_signal
};
// Most other functions removed as unused, but these are needed for zoom functionality
// MIGRATED: Canvas dimensions, zoom/pan flags, mouse tracking now from actors/waveform_timeline.rs
use crate::actors::waveform_timeline::{current_cursor_position, current_cursor_position_seconds, current_viewport,
    set_cursor_position, set_cursor_position_if_changed, set_cursor_position_seconds,
    set_viewport_if_changed, viewport_signal,
    current_ns_per_pixel, set_ns_per_pixel_if_changed, ns_per_pixel_signal,
    current_coordinates, current_canvas_width, current_canvas_height, set_canvas_dimensions,
    current_zoom_center_seconds};
// MIGRATED: WaveformTimeline domain now initialized through actors/waveform_timeline.rs
use crate::actors::global_domains::waveform_timeline_domain;
use crate::time_types::{TimeNs, Viewport, NsPerPixel, TimelineCoordinates};
use crate::platform::{Platform, CurrentPlatform};
use crate::config::app_config;
use shared::{SelectedVariable, UpMsg, SignalTransitionQuery, SignalTransition};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::Theme as SharedTheme;
use wasm_bindgen::JsCast;
use js_sys;
// Note: Only import colors that are actually used in the canvas rendering
use palette::{Oklch, Srgb, IntoColor};

// High-performance cursor movement constants
const PIXELS_PER_FRAME: f64 = 20.0;      // Consistent 20-pixel movement per frame
// Minimum range is now based on maximum zoom level (1 ns/pixel) 
// For any canvas: minimum range = MAX_ZOOM_IN * canvas_width
// Example: 1000px canvas at 1ns/pixel = 1000ns minimum range
fn get_min_valid_range_ns(canvas_width: u32) -> u64 {
    NsPerPixel::MAX_ZOOM_IN.nanos() * canvas_width as u64
}
// ═══════════════════════════════════════════════════════════════════════════════════
// TIMELINE STARTUP 1: Debug fallback values - these should NOT be used during startup!
// ═══════════════════════════════════════════════════════════════════════════════════
// ❌ FALLBACK ELIMINATION: Removed SAFE_FALLBACK constants - use actual file ranges instead
const MOVEMENT_FRAME_MS: u32 = 16;       // 60fps animation frame timing
const _MAX_FAILURES: u32 = 10;           // Circuit breaker threshold

// High-precision timing for smooth cursor animation (nanoseconds)
#[allow(dead_code)]
const ANIMATION_FRAME_NS: u64 = 16_666_666; // 16.666ms = 60fps in nanoseconds

// REMOVED: SIGNAL_TRANSITIONS_CACHE - now handled by unified_timeline_service
// Raw signal transitions are now stored in UNIFIED_TIMELINE_CACHE.raw_transitions

// Simplified request tracking - just a pending flag to prevent overlapping requests
// MIGRATED: Pending request tracking → use has_pending_request_signal() from waveform_timeline
static _HAS_PENDING_REQUEST: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Note: Old complex deduplication system removed - now using simple throttling + batching


/// Clean up old request timestamps to prevent memory leaks
// Old complex deduplication functions removed - now using simple throttling + batching

// Cache for processed canvas transitions - prevents redundant processing and backend requests
// MIGRATED: Canvas cache → use processed_canvas_cache_signal() from waveform_timeline
// pub static PROCESSED_CANVAS_CACHE: Lazy<Mutable<HashMap<String, Vec<(f32, shared::SignalValue)>>>> = Lazy::new(|| Mutable::new(HashMap::new()));

/// Request transitions only for variables that haven't been requested yet
/// This prevents the O(N²) request flood when adding multiple variables
fn request_transitions_for_new_variables_only(time_range: Option<(f64, f64)>) {
    // Simplified: delegate to the optimized batched function
    // The batching and throttling will handle efficiency
    crate::debug_utils::debug_conditional("New variables request: delegating to optimized batched function");
    request_transitions_for_all_variables(time_range);
}

/// Force request transitions for all variables (use for timeline range changes)
fn request_transitions_for_all_variables(time_range: Option<(f64, f64)>) {
    let Some((min_time, max_time)) = time_range else { return; };
    
    // Get variable data using domain synchronous access
    let variables_to_request: Vec<String> = {
        let current_vars = crate::actors::selected_variables::current_variables();
        current_vars.iter().map(|var| var.unique_id.clone()).collect()
    };
    
    if variables_to_request.is_empty() {
        return;
    }
    
    // NEW: Use unified SignalDataService instead of old query system
    let signal_requests: Vec<crate::unified_timeline_service::SignalRequest> = variables_to_request
        .into_iter()
        .filter_map(|unique_id| {
            // Parse unique_id: "/path/file.ext|scope|variable"
            let parts: Vec<&str> = unique_id.split('|').collect();
            if parts.len() >= 3 {
                Some(crate::unified_timeline_service::SignalRequest {
                    file_path: parts[0].to_string(),
                    scope_path: parts[1].to_string(),
                    variable_name: parts[2].to_string(),
                    time_range_ns: Some(((min_time * 1_000_000_000.0) as u64, (max_time * 1_000_000_000.0) as u64)),
                    max_transitions: Some(10000), // Match timeline needs
                    format: shared::VarFormat::Hexadecimal, // Default for timeline
                })
            } else {
                None
            }
        })
        .collect();
    
    if signal_requests.is_empty() {
        return;
    }
    
    // Request data through unified service 
    let signal_ids: Vec<String> = signal_requests.iter()
        .map(|req| format!("{}|{}|{}", req.file_path, req.scope_path, req.variable_name))
        .collect();
    let cursor_time_ns = match current_cursor_position() {
        Some(cursor) => cursor,
        None => return, // Timeline not initialized yet
    };
    let _request_count = signal_requests.len(); // logging removed
    crate::unified_timeline_service::UnifiedTimelineService::request_cursor_values(signal_ids, cursor_time_ns);
    
}

/// Clear transition request tracking for removed variables (simplified)
pub fn _clear_transition_tracking_for_variable(_unique_id: &str) {
    // Old complex tracking system removed - now just clear pending flag if needed
    _HAS_PENDING_REQUEST.set(false);
    crate::debug_utils::debug_conditional("Cleared transition tracking (simplified)");
}

/// Clear all transition request tracking (simplified)
pub fn _clear_all_transition_tracking() {
    _HAS_PENDING_REQUEST.set(false);
    crate::debug_utils::debug_conditional("Cleared all transition tracking (simplified)");
}





// Store current theme for synchronous access
static CURRENT_THEME_CACHE: Lazy<Mutable<NovyUITheme>> = Lazy::new(|| {
    Mutable::new(NovyUITheme::Dark) // Default to dark
});

// Store hover information for tooltip display
static HOVER_INFO: Lazy<Mutable<Option<HoverInfo>>> = Lazy::new(|| {
    Mutable::new(None)
});

// Dedicated counter to force canvas redraws when incremented
// MIGRATED: Force redraw → use force_redraw_signal() from waveform_timeline
// static FORCE_REDRAW: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

// Throttle canvas redraws to prevent excessive backend requests
// MIGRATED: Last redraw time → use last_redraw_time_signal() from waveform_timeline
// static LAST_REDRAW_TIME: Lazy<Mutable<f64>> = Lazy::new(|| Mutable::new(0.0));
#[allow(dead_code)]
const REDRAW_THROTTLE_MS: f64 = 16.0; // Max 60fps redraws

// High-performance direct cursor animation state
#[derive(Clone, Debug)]
struct DirectCursorAnimation {
    current_position: f64,     // Current position in seconds (high precision)
    target_position: f64,      // Target position in seconds
    velocity_pixels_per_frame: f64, // Movement speed in pixels per frame
    is_animating: bool,        // Animation active flag
    direction: i8,             // -1 for left, 1 for right, 0 for stopped
    _last_frame_time: u64,      // Last animation frame timestamp (nanoseconds)
}

impl Default for DirectCursorAnimation {
    fn default() -> Self {
        Self {
            current_position: 0.0,
            target_position: 0.0,
            velocity_pixels_per_frame: PIXELS_PER_FRAME,
            is_animating: false,
            direction: 0,
            _last_frame_time: 0,
        }
    }
}

// Direct cursor animation state - no Tweened overhead
static DIRECT_CURSOR_ANIMATION: Lazy<Mutable<DirectCursorAnimation>> = Lazy::new(|| {
    Mutable::new(DirectCursorAnimation::default())
});

// Canvas update debouncing to reduce redraw overhead
// MIGRATED: Last canvas update → use last_canvas_update_signal() from waveform_timeline
// static LAST_CANVAS_UPDATE: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(0));
static PENDING_CANVAS_UPDATE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Debouncing for transition navigation to prevent rapid key press issues
static LAST_TRANSITION_NAVIGATION_TIME: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(0));
const TRANSITION_NAVIGATION_DEBOUNCE_MS: u64 = 100; // 100ms debounce

// f64 precision tolerance for transition navigation (much more precise than f32)
const F64_PRECISION_TOLERANCE: f64 = 1e-15;

// PERFORMANCE FIX: Incremental Canvas Object Cache
// Caches rendered objects by variable unique_id to avoid full recreation
static VARIABLE_OBJECT_CACHE: Lazy<Mutable<HashMap<String, Vec<fast2d::Object2d>>>> = 
    Lazy::new(|| Mutable::new(HashMap::new()));

// Track last known variables to detect changes
static LAST_VARIABLES_STATE: Lazy<Mutable<Vec<SelectedVariable>>> = 
    Lazy::new(|| Mutable::new(Vec::new()));

// Track last canvas dimensions for full redraw detection
static LAST_CANVAS_DIMENSIONS: Lazy<Mutable<(f32, f32)>> = 
    Lazy::new(|| Mutable::new((0.0, 0.0)));


#[derive(Clone, Debug, PartialEq)]
struct HoverInfo {
    mouse_x: f32,
    mouse_y: f32,
    time: f32,
    variable_name: String,
    value: String,
}


// Time unit detection for intelligent timeline formatting
#[derive(Debug, Clone, Copy)]
enum TimeUnit {
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
}

impl TimeUnit {
    fn suffix(&self) -> &'static str {
        match self {
            TimeUnit::Nanosecond => "ns",
            TimeUnit::Microsecond => "μs",  // Proper microsecond symbol
            TimeUnit::Millisecond => "ms",
            TimeUnit::Second => "s",
        }
    }
    
    fn scale_factor(&self) -> f32 {
        match self {
            TimeUnit::Nanosecond => 1e9,
            TimeUnit::Microsecond => 1e6,
            TimeUnit::Millisecond => 1e3,
            TimeUnit::Second => 1.0,
        }
    }
}

// Determine appropriate time unit based on time range
fn get_time_unit_for_range(min_time: f64, max_time: f64) -> TimeUnit {
    let range = max_time - min_time;
    if range < 1e-6 {  // Less than 1 microsecond - use nanoseconds
        TimeUnit::Nanosecond
    } else if range < 1e-3 {  // Less than 1 millisecond - use microseconds
        TimeUnit::Microsecond
    } else if range < 1.0 {   // Less than 1 second - use milliseconds
        TimeUnit::Millisecond
    } else {
        TimeUnit::Second
    }
}

// Format time value with appropriate unit and precision
fn format_time_with_unit(time_seconds: f32, unit: TimeUnit) -> String {
    let scaled_value = time_seconds * unit.scale_factor();
    match unit {
        TimeUnit::Nanosecond => {
            // For nanoseconds, show integers
            format!("{}{}", scaled_value.round() as i32, unit.suffix())
        }
        TimeUnit::Microsecond => {
            // For microseconds, show clean integers
            format!("{}{}", scaled_value.round() as i32, unit.suffix())
        }
        _ => {
            // Milliseconds and seconds use integer formatting
            format!("{}{}", scaled_value.round() as i32, unit.suffix())
        }
    }
}

// OKLCH to RGB conversion utility
fn oklch_to_rgb(l: f32, c: f32, h: f32) -> (u8, u8, u8, f32) {
    let oklch = Oklch::new(l, c, h);
    let rgb: Srgb<f32> = oklch.into_color();
    (
        (rgb.red * 255.0).round() as u8,
        (rgb.green * 255.0).round() as u8,
        (rgb.blue * 255.0).round() as u8,
        1.0
    )
}

// Convert NovyUI theme tokens to canvas RGB values
fn get_theme_token_rgb(theme: &NovyUITheme, token: &str) -> (u8, u8, u8, f32) {
    match (theme, token) {
        // Dark theme conversions
        (NovyUITheme::Dark, "neutral_1") => oklch_to_rgb(0.12, 0.025, 255.0),  // Panel background
        (NovyUITheme::Dark, "neutral_2") => oklch_to_rgb(0.15, 0.025, 255.0),  // Subtle background - footer color  
        (NovyUITheme::Dark, "neutral_3") => oklch_to_rgb(0.30, 0.045, 255.0),  // Medium background
        (NovyUITheme::Dark, "neutral_4") => oklch_to_rgb(0.22, 0.025, 255.0),  // Darker neutral
        (NovyUITheme::Dark, "neutral_5") => oklch_to_rgb(0.28, 0.025, 255.0),  // Medium neutral
        (NovyUITheme::Dark, "neutral_12") => oklch_to_rgb(0.95, 0.025, 255.0), // High contrast text
        // Contrasting value rectangle colors
        (NovyUITheme::Dark, "value_light_blue") => (18, 25, 40, 1.0),   // Lighter blue for better contrast
        (NovyUITheme::Dark, "value_dark_gray") => (8, 8, 12, 1.0),      // Dark gray for alternating
        (NovyUITheme::Dark, "primary_1") => oklch_to_rgb(0.16, 0.02, 250.0),   // Very dark primary
        (NovyUITheme::Dark, "primary_2") => oklch_to_rgb(0.18, 0.03, 250.0),   // Darker primary
        (NovyUITheme::Dark, "primary_3") => oklch_to_rgb(0.30, 0.05, 250.0),   // Subtle primary
        (NovyUITheme::Dark, "primary_4") => oklch_to_rgb(0.35, 0.07, 250.0),   // Medium primary
        
        // Light theme conversions
        (NovyUITheme::Light, "neutral_1") => oklch_to_rgb(0.99, 0.025, 255.0),
        (NovyUITheme::Light, "neutral_2") => oklch_to_rgb(0.97, 0.025, 255.0),
        (NovyUITheme::Light, "neutral_3") => oklch_to_rgb(0.92, 0.045, 255.0),
        (NovyUITheme::Light, "neutral_4") => oklch_to_rgb(0.90, 0.025, 255.0),
        (NovyUITheme::Light, "neutral_5") => oklch_to_rgb(0.85, 0.025, 255.0),
        (NovyUITheme::Light, "neutral_12") => oklch_to_rgb(0.15, 0.025, 255.0),
        // Light theme value rectangles - prettier original colors
        (NovyUITheme::Light, "value_light_blue") => (219, 234, 254, 1.0),   // Light bluish background
        (NovyUITheme::Light, "value_dark_gray") => (191, 219, 254, 1.0),    // Slightly darker light blue
        (NovyUITheme::Light, "primary_3") => oklch_to_rgb(0.90, 0.05, 250.0),
        (NovyUITheme::Light, "primary_4") => oklch_to_rgb(0.85, 0.07, 250.0),
        
        // Fallback for unknown tokens
        _ => (128, 128, 128, 1.0),
    }
}

// Clear processed signal cache to force fresh calculation for timeline changes
pub fn clear_processed_signal_cache() {
    // ✅ CRITICAL FIX: Clear the processed canvas cache when zoom/viewport changes  
    // This forces fresh coordinate calculations with new zoom level/viewport
    // Clearing canvas cache due to state change
    
    // Trigger a redraw using the proper relay - this forces canvas to re-render with fresh data
    crate::actors::waveform_timeline::redraw_requested_relay().send(());
    // Canvas redraw triggered
    
    // Also manually force a data request for all currently selected variables with new viewport
    let current_range = get_current_timeline_range();
    request_transitions_for_all_variables(current_range);
    // Viewport data refresh requested
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
            neutral_2: get_theme_token_rgb(current_theme, "neutral_2"),     // Proper neutral_2 for timeline footer
            neutral_3: get_theme_token_rgb(current_theme, "neutral_3"),     // Proper neutral_3 
            neutral_12: get_theme_token_rgb(current_theme, "neutral_12"),   // OKLCH text color (no hardcode)
            cursor_color: (37, 99, 235, 0.9), // Primary_6 cursor for consistency
            value_color_1: get_theme_token_rgb(current_theme, "value_light_blue"), // Lighter blue for contrast
            value_color_2: get_theme_token_rgb(current_theme, "value_dark_gray"), // Dark gray for alternating
            grid_color: (75, 79, 86, 0.15), // More subtle grid lines
            separator_color: (75, 79, 86, 0.2), // Very subtle row separators
            hover_panel_bg: (20, 22, 25, 0.95), // Very dark like select dropdown
            hover_panel_text: get_theme_token_rgb(current_theme, "neutral_12"), // OKLCH text (no hardcode)
        },
        NovyUITheme::Light => ThemeColors {
            neutral_2: get_theme_token_rgb(current_theme, "neutral_2"),     // Proper neutral_2 for timeline footer
            neutral_3: get_theme_token_rgb(current_theme, "neutral_3"),     // Proper neutral_3 
            neutral_12: get_theme_token_rgb(current_theme, "neutral_12"),   // OKLCH text color
            cursor_color: (37, 99, 235, 0.9), // Primary_6 cursor for consistency
            value_color_1: get_theme_token_rgb(current_theme, "value_light_blue"), // Light blue for contrast
            value_color_2: get_theme_token_rgb(current_theme, "value_dark_gray"), // Light gray for alternating
            grid_color: (209, 213, 219, 0.4), // Subtle grid lines for light theme
            separator_color: (209, 213, 219, 0.3), // Very subtle row separators
            hover_panel_bg: (250, 251, 252, 0.95), // Almost white like select dropdown
            hover_panel_text: get_theme_token_rgb(current_theme, "neutral_12"), // OKLCH text
        },
    }
}

struct ThemeColors {
    neutral_2: (u8, u8, u8, f32),
    neutral_3: (u8, u8, u8, f32),
    neutral_12: (u8, u8, u8, f32),
    cursor_color: (u8, u8, u8, f32),
    value_color_1: (u8, u8, u8, f32),  // Primary color for value rectangles
    value_color_2: (u8, u8, u8, f32),  // Secondary color for value rectangles
    grid_color: (u8, u8, u8, f32),     // Grid lines color
    separator_color: (u8, u8, u8, f32), // Row separator color
    hover_panel_bg: (u8, u8, u8, f32), // Bluish background for hover panel
    hover_panel_text: (u8, u8, u8, f32), // High contrast text for hover panel
}

// Helper function to round raw time steps to professional-looking numbers
fn round_to_nice_number(raw: f32) -> f32 {
    if raw <= 0.0 { return 1.0; }
    
    // Special handling for very small values (microsecond and nanosecond ranges)
    if raw < 1e-8 {
        // Nanosecond range - use 0.1, 0.2, 0.5, 1.0, 2.0, 5.0 nanosecond steps
        let magnitude = 1e-9; // 1 nanosecond
        let normalized = raw / magnitude;
        let nice_normalized = if normalized <= 0.1 { 0.1 }
        else if normalized <= 0.2 { 0.2 }
        else if normalized <= 0.5 { 0.5 }
        else if normalized <= 1.0 { 1.0 }
        else if normalized <= 2.0 { 2.0 }
        else if normalized <= 5.0 { 5.0 }
        else { 10.0 };
        return nice_normalized * magnitude;
    } else if raw < 1e-5 {
        // Microsecond range - use 0.1, 0.2, 0.5, 1.0, 2.0, 5.0 microsecond steps
        let magnitude = 1e-6; // 1 microsecond
        let normalized = raw / magnitude;
        let nice_normalized = if normalized <= 0.1 { 0.1 }
        else if normalized <= 0.2 { 0.2 }
        else if normalized <= 0.5 { 0.5 }
        else if normalized <= 1.0 { 1.0 }
        else if normalized <= 2.0 { 2.0 }
        else if normalized <= 5.0 { 5.0 }
        else { 10.0 };
        return nice_normalized * magnitude;
    }
    
    // Standard 1-2-5 scaling for larger values
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

/// Validate and recover initial timeline state on startup  
fn validate_startup_state_with_canvas_width(actual_canvas_width: f32) {
    // Debug canvas width and zoom calculation - triggered by validation
    let cursor_pos = match current_cursor_position_seconds() {
        Some(pos) => pos,
        None => return, // Timeline not initialized yet
    };
    let ns_per_pixel = match current_ns_per_pixel() {
        Some(ns_per_pixel) => ns_per_pixel,
        None => return, // Timeline not initialized yet
    };
    let viewport = match current_viewport() {
        Some(viewport) => viewport,
        None => return, // Timeline not initialized yet
    };
    let start = viewport.start.display_seconds();
    let end = viewport.end.display_seconds();
    
    // CRITICAL: Use actual canvas width from DOM, not cached Actor value  
    let canvas_width = actual_canvas_width as u32;
    let viewport_range_ns = viewport.end.nanos() - viewport.start.nanos();
    // Fix: Use proper rounding instead of truncated integer division
    let calculated_ns_per_pixel = NsPerPixel((viewport_range_ns + canvas_width as u64 / 2) / canvas_width as u64);
    
    
    // Check if any values are invalid
    let min_valid_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
    if !cursor_pos.is_finite() || !start.is_finite() || !end.is_finite() || 
       ns_per_pixel.nanos() == 0 || start >= end || (end - start) < min_valid_range {
        
        crate::debug_utils::debug_timeline_validation("STARTUP: Invalid timeline state detected, applying recovery");
        // Timeline startup: Invalid state detected, attempting recovery
        
        // STARTUP FIX: Try to use actual file data instead of hardcoded 1s fallback
        let recovery_viewport = if let Some((file_min, file_max)) = get_current_timeline_range() {
            let file_span = file_max - file_min;
            if file_span > 10.0 {  // Substantial file data (VCD with 250s span)
                // Recovery: Using available file data for timeline bounds
                Viewport::new(
                    TimeNs::from_external_seconds(file_min), 
                    TimeNs::from_external_seconds(file_max)
                )
            } else {
                zoon::println!("⚠️ File data too small detected, but NO FALLBACKS rule - keeping existing viewport");
                match current_viewport() {
                    Some(viewport) => viewport,
                    None => return, // Can't recover without viewport - timeline not initialized
                }
            }
        } else {
            zoon::println!("⚠️ No file data available detected, but NO FALLBACKS rule - keeping existing viewport");
            match current_viewport() {
                Some(viewport) => viewport,
                None => return, // Can't recover without viewport - timeline not initialized
            }
        };
        
        set_viewport_if_changed(recovery_viewport);
        
        // Set cursor to middle of range
        let cursor_time = (recovery_viewport.start.display_seconds() + recovery_viewport.end.display_seconds()) / 2.0;
        set_cursor_position_if_changed(TimeNs::from_external_seconds(cursor_time));
        
        // Calculate proper zoom ratio based on viewport range and canvas width
        let viewport_range_ns = recovery_viewport.end.nanos() - recovery_viewport.start.nanos();
        // Fix: Use proper rounding instead of truncated integer division
        let calculated_ns_per_pixel = NsPerPixel((viewport_range_ns + canvas_width as u64 / 2) / canvas_width as u64);
        zoon::println!("🔍 RECOVERY ZOOM CALCULATION:");
        zoon::println!("   viewport_range_ns: {} ns", viewport_range_ns);
        zoon::println!("   canvas_width: {} px", canvas_width);
        zoon::println!("   calculated_ns_per_pixel: {} ns/px", calculated_ns_per_pixel.0);
        zoon::println!("   Display format: {}", calculated_ns_per_pixel);
        set_ns_per_pixel_if_changed(calculated_ns_per_pixel);
        
        // Note: Timeline coordinates will be automatically updated through the domain bridge
        // as the cursor, viewport, and ns_per_pixel values are set above
        
        // FIXED: Initialize zoom center to cursor position using Actor+Relay system
        let cursor_position = match current_cursor_position_seconds() {
            Some(pos) => pos,
            None => return, // Timeline not initialized yet
        };
        // Use Actor+Relay system instead of legacy ZOOM_CENTER_NS
        crate::actors::waveform_timeline::set_zoom_center_follow_mouse(TimeNs::from_external_seconds(cursor_position));
    } else {
        crate::debug_utils::debug_timeline_validation("STARTUP: Timeline state validation passed");
        
        // Ensure zoom ratio is properly calculated even for valid state
        let viewport = match current_viewport() {
            Some(viewport) => viewport,
            None => return, // Timeline not initialized yet
        };
        let viewport_range_ns = viewport.end.nanos() - viewport.start.nanos();
        // Fix: Use proper rounding instead of truncated integer division
        let current_calculated_ns_per_pixel = NsPerPixel((viewport_range_ns + canvas_width as u64 / 2) / canvas_width as u64);
        
        // ═══════════════════════════════════════════════════════════════════════════════════
        // TIMELINE STARTUP 1: Debug zoom calculation using wrong viewport range
        // ═══════════════════════════════════════════════════════════════════════════════════
        // Conditional startup zoom analysis - only when debug flag enabled  
        if crate::debug_utils::is_startup_zoom_debug_enabled() {
            // Analyzing startup zoom calculation state
            
            // Analyze the discrepancy
            let expected_range_s = 250.0;
            let expected_ns = (expected_range_s * 1_000_000_000.0) as u64;
            // Fix: Use proper rounding instead of truncated integer division
            let expected_zoom = NsPerPixel((expected_ns + canvas_width as u64 / 2) / canvas_width as u64);
            
            if viewport.end.display_seconds() == 1.0 {
                // Problem: Using minimal 1s fallback instead of VCD file range
                
                // STARTUP FIX: Replace 1s fallback viewport with actual file data
                if let Some((file_min, file_max)) = get_current_timeline_range() {
                    let file_span = file_max - file_min;
                    if file_span > 10.0 {  // Substantial file data available
                        let corrected_viewport = Viewport::new(
                            TimeNs::from_external_seconds(file_min), 
                            TimeNs::from_external_seconds(file_max)
                        );
                        set_viewport_if_changed(corrected_viewport);
                        
                        // Also update cursor to middle of file range
                        let cursor_time = (file_min + file_max) / 2.0;
                        set_cursor_position_if_changed(TimeNs::from_external_seconds(cursor_time));
                        
                        // CRITICAL: Also update zoom ratio to match the new viewport range  
                        let canvas_width = actual_canvas_width as u32;
                        let corrected_viewport_range_ns = corrected_viewport.end.nanos() - corrected_viewport.start.nanos();
                        // Fix: Use proper rounding instead of truncated integer division
                        let correct_ns_per_pixel = NsPerPixel((corrected_viewport_range_ns + canvas_width as u64 / 2) / canvas_width as u64);
                        set_ns_per_pixel_if_changed(correct_ns_per_pixel);
                        
                        // Successfully updated viewport to show VCD timeline on startup
                    } else {
                        // File data too small to replace fallback
                    }
                } else {
                    // No file data available to replace fallback
                }
            } else if viewport.end.display_seconds() == 250.0 {
                // Viewport correct: Using proper file range
            } else {
                zoon::println!("   ⚠️  UNEXPECTED RANGE: Neither 1s fallback nor 250s file range");
            }
            
            // Update zoom ratio if it doesn't match the calculated value
            if current_calculated_ns_per_pixel != ns_per_pixel {
                zoon::println!("   🔄 UPDATING zoom from {} to {}", ns_per_pixel, current_calculated_ns_per_pixel);
                zoon::println!("   📈 This change should make timeline footer and zoom level consistent");
            } else {
                zoon::println!("   ✅ Zoom already matches viewport calculation");
            }
        } // End of debug conditional block
        
        // Keep functional code outside debug conditional - update zoom if needed  
        if current_calculated_ns_per_pixel != ns_per_pixel {
            set_ns_per_pixel_if_changed(current_calculated_ns_per_pixel);
        }
    }
}

async fn create_canvas_element() -> impl Element {
    // Skip early zoom validation - will be done after canvas dimensions are available
    
    // Wait a moment and test our canvas width debugging
    
    let mut zoon_canvas = Canvas::new()
        .width(100)  // Minimal default - will be updated to actual container size
        .height(100) // Minimal default - will be updated to actual container size
        .s(Width::fill())
        .s(Height::fill());

    let dom_canvas = zoon_canvas.raw_el_mut().dom_element();
    let mut canvas_wrapper = fast2d::CanvasWrapper::new_with_canvas(dom_canvas.clone()).await;

    // Initialize with current theme (theme reactivity will update it)
    canvas_wrapper.update_objects(|objects| {
        let selected_vars = crate::actors::selected_variables::current_variables();
        let novyui_theme = CURRENT_THEME_CACHE.get();
        *objects = create_waveform_objects_with_theme(&selected_vars, &novyui_theme);
    });

    // Wrap canvas_wrapper in Rc<RefCell> for sharing
    let canvas_wrapper_shared = Rc::new(RefCell::new(canvas_wrapper));
    
    // Canvas dimensions will be set dynamically when actual canvas is created
    // No hardcoded dimensions - wait for real canvas size from DOM

    // Initialize direct cursor animation with current cursor position
    let current_cursor = current_cursor_position_seconds();
    if let Some(cursor) = current_cursor {
        DIRECT_CURSOR_ANIMATION.lock_mut().current_position = cursor;
        DIRECT_CURSOR_ANIMATION.lock_mut().target_position = cursor;
    }
    let canvas_wrapper_for_signal = canvas_wrapper_shared.clone();


    // UNIFIED CANVAS UPDATE SIGNAL: Combine all triggers to prevent cascade amplification
    // This replaces 8 separate signal handlers with a single efficient unified handler
    let canvas_wrapper_unified = canvas_wrapper_shared.clone();
    Task::start(async move {
        let timeline_domain = waveform_timeline_domain();
        map_ref! {
            let theme_value = app_config().theme_actor.signal(),
            let zoom_state = ns_per_pixel_signal(),
            let cursor_pos = timeline_domain.cursor_position_signal(),
            let zoom_center = timeline_domain.zoom_center_signal(),
            let _cache_trigger = crate::unified_timeline_service::UnifiedTimelineService::cache_signal(),
            let _hover_trigger = HOVER_INFO.signal_ref(|_| ()),
            let _force_redraw = crate::actors::waveform_timeline::force_redraw_signal(),
            let _variables_changed = crate::actors::selected_variables::variables_signal() => {
                // Return tuple of all values that should trigger canvas updates
                (convert_theme(&theme_value), *zoom_state, *cursor_pos, *zoom_center)
            }
        }.dedupe_cloned().for_each(move |(novyui_theme, _zoom_state, _cursor_pos, _zoom_center)| {
            let canvas_wrapper_unified = canvas_wrapper_unified.clone();
            async move {
                // PROPER ASYNC COORDINATION: Yield to event loop to ensure data availability
                // This allows file loading and variable selection signals to complete before rendering
                Task::next_macro_tick().await;
                
                // Update theme cache for other components
                CURRENT_THEME_CACHE.set_neq(novyui_theme.clone());
                
                canvas_wrapper_unified.borrow_mut().update_objects(move |objects| {
                    let canvas_width = match current_canvas_width() {
                        Some(width) => width,
                        None => return, // Timeline not initialized yet
                    };
                    let canvas_height = current_canvas_height();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    // Check data availability before rendering
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    if selected_vars.is_empty() {
                        // No variables selected - render empty canvas
                        *objects = Vec::new();
                        return;
                    }
                    
                    // Verify timeline range is available for selected variables
                    if get_maximum_timeline_range().is_none() {
                        // File data not yet available - skip this update
                        // Canvas will be updated again when data becomes available through signals
                        zoon::println!("⏳ Canvas update skipped - file data not yet available for selected variables");
                        return;
                    }
                    
                    // ✅ CRITICAL FIX: Use signal values from unified handler, not static cache
                    let cursor_pos = _cursor_pos.display_seconds(); // Use the actual signal value
                    let zoom_center_pos = _zoom_center.display_seconds(); // Use the domain actor zoom center signal
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos, zoom_center_pos);
                });
            }
        }).await;
    });


    // High-performance direct cursor animation with smart debouncing
    start_direct_cursor_animation_loop();

    // Update timeline range when selected variables change - OPTIMIZED to prevent O(N²) requests  
    Task::start(async move {
        // Combine variables signal with config loaded state for reactive pattern
        map_ref! {
            let _variables = crate::actors::selected_variables::variables_signal_vec().to_signal_cloned().dedupe_cloned(),
            let config_loaded = app_config().is_loaded_actor.signal() => {
                // Calculate new range from selected variables 
                if let Some((min_time, max_time)) = get_current_timeline_range() {
                    let range_viewport = crate::time_types::Viewport::new(
                        TimeNs::from_external_seconds(min_time),
                        TimeNs::from_external_seconds(max_time)
                    );
                    set_viewport_if_changed(range_viewport);
                    
                    // DEDUPLICATION: Only request transitions if main.rs handler won't handle it
                    // main.rs handler has condition: CONFIG_LOADED.get() && !IS_LOADING.get()
                    // So this handler should request ONLY when that condition is false
                    if !config_loaded || IS_LOADING.get() {
                        request_transitions_for_new_variables_only(Some((min_time, max_time)));
                    }
                    
                    trigger_canvas_redraw();
                } else {
                    // ✅ FALLBACK ELIMINATED: NO FALLBACKS rule - don't set 1s viewport
                    zoon::println!("ℹ️ No selected variables detected, but NOT setting fallback viewport (NO FALLBACKS rule)");
                    // The timeline should continue using the correct 250s viewport from VCD data
                }
            }
        }.for_each(|_| async {}).await;
    });

    // ✅ NO STARTUP FIX NEEDED - Canvas only renders when real VCD data is available

    // Clear cache and redraw when timeline range changes (critical for zoom operations)
    let canvas_wrapper_for_timeline_changes = canvas_wrapper_shared.clone();
    Task::start(async move {
        // Combined signal for any timeline range change
        map_ref! {
            let viewport = viewport_signal(),
            let ns_per_pixel = ns_per_pixel_signal()
            => (viewport.start.display_seconds(), viewport.end.display_seconds(), *ns_per_pixel)
        }
        .dedupe() // Prevent duplicate triggers
        .for_each(move |_| {
            let canvas_wrapper = canvas_wrapper_for_timeline_changes.clone();
            async move {
                // CRITICAL: Clear all cached processed data when timeline changes
                clear_processed_signal_cache();
                
                // When timeline range changes (zoom/pan), request new data for ALL variables in new range
                let current_range = get_current_timeline_range();
                request_transitions_for_all_variables(current_range);
                
                canvas_wrapper.borrow_mut().update_objects(move |objects| {
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = match current_cursor_position_seconds() {
                        Some(pos) => pos,
                        None => return, // Timeline not initialized yet
                    };
                    let zoom_center_pos = current_zoom_center_seconds();
                    let canvas_width = match current_canvas_width() {
                        Some(width) => width,
                        None => return, // Timeline not initialized yet
                    };
                    let canvas_height = current_canvas_height();
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    // ✅ FIXED: Use separate cursor and zoom center positions
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos, zoom_center_pos);
                });
            }
        }).await;
    });


    // React to canvas dimension changes
    let canvas_wrapper_for_dims = canvas_wrapper_shared.clone();
    Task::start(async move {
        crate::actors::waveform_timeline::canvas_width_signal().for_each(move |_| {
            let _canvas_wrapper = canvas_wrapper_for_dims.clone();
            async move {
                trigger_canvas_redraw();
            }
        }).await;
    });

    // Force initial render after canvas enters DOM
    let _canvas_wrapper_init = canvas_wrapper_shared.clone();
    let dom_canvas_init = dom_canvas.clone();
    let zoon_canvas = zoon_canvas.after_insert(move |_| {
        // Canvas is now in DOM, get actual dimensions and validate timeline state
        let rect = dom_canvas_init.get_bounding_client_rect();
        if rect.width() > 0.0 && rect.height() > 0.0 {
            let width = rect.width() as u32;
            let height = rect.height() as u32;
            
            // Canvas initialized with DOM dimensions
            
            // Set canvas element attributes to match container size
            dom_canvas_init.set_width(width);
            dom_canvas_init.set_height(height);
            
            set_canvas_dimensions(width as f32, height as f32);
            
            // NOW validate timeline state with actual DOM canvas dimensions
            validate_startup_state_with_canvas_width(width as f32);
            
            trigger_canvas_redraw();
        }
    });

    let canvas_wrapper_for_resize = canvas_wrapper_shared.clone();
    let dom_canvas_resize = dom_canvas.clone();
    zoon_canvas.update_raw_el(move |raw_el| {
        raw_el.on_resize(move |width, height| {
            // Enhanced resize handler with validation
            if width > 0 && height > 0 {
                // Canvas resized - triggering redraw
                
                // Set canvas element attributes to match new container size
                dom_canvas_resize.set_width(width as u32);
                dom_canvas_resize.set_height(height as u32);
                
                // Store canvas dimensions for click calculations
                set_canvas_dimensions(width as f32, height as f32);
                
                // Call Fast2D resize
                canvas_wrapper_for_resize.borrow_mut().resized(width, height);
                
                // Trigger full redraw through the dedicated handler
                trigger_canvas_redraw();
            }
        })
        .event_handler({
            let canvas_wrapper_for_click = canvas_wrapper_shared.clone();
            move |event: events::Click| {
                // Handle click to move cursor position using WaveformTimeline domain
                let page_click_x = event.x();
                let page_click_y = event.y();
                
                // Get canvas element's position relative to page
                let canvas_element = match event.target() {
                    Some(target) => target,
                    None => return,
                };
                let canvas_rect = match canvas_element.dyn_into::<web_sys::Element>() {
                    Ok(element) => element.get_bounding_client_rect(),
                    Err(_) => return,
                };
                let canvas_left = canvas_rect.left();
                let canvas_top = canvas_rect.top();
                let canvas_width_dom = canvas_rect.width();
                let canvas_height_dom = canvas_rect.height();
                // Calculate click position relative to canvas
                let click_x = page_click_x as f64 - canvas_left;
                let click_y = page_click_y as f64 - canvas_top;
                
                // Legacy code for backward compatibility (will be removed when migration complete)
                // Use stored canvas width
                let canvas_width = match current_canvas_width() {
                    Some(width) => width,
                    None => return, // Timeline not initialized yet
                };
                let canvas_height = current_canvas_height();
                // Get cached canvas dimensions for coordinate calculation
                
                // Get coordinate system and validate timeline state
                let coords = match current_coordinates() {
                    Some(coords) => coords,
                    None => return, // Timeline not initialized yet
                };
                
                // Verify ns_per_pixel calculation accuracy
                let viewport = match current_viewport() {
                    Some(viewport) => viewport,
                    None => return, // Timeline not initialized yet
                };
                let actual_span_ns = viewport.end.nanos() - viewport.start.nanos();
                let actual_canvas_width = coords.canvas_width_pixels;
                
                // Calculate correct ns_per_pixel value
                let correct_ns_per_pixel = (actual_span_ns + actual_canvas_width as u64 / 2) / actual_canvas_width as u64;
                let current_ns_per_pixel = coords.ns_per_pixel.nanos();
                
                if current_ns_per_pixel != correct_ns_per_pixel {
                    // Fix coordinate system precision issues
                    set_ns_per_pixel_if_changed(NsPerPixel(correct_ns_per_pixel));
                }
                // Timeline coordinate calculation
                zoon::println!("   ns_per_pixel: {:.3}ns/px (zoom={:.3}ms/px)", coords.ns_per_pixel.nanos(), coords.ns_per_pixel.nanos() as f64 / 1_000_000.0);
                zoon::println!("   canvas_width_pixels: {}px", coords.canvas_width_pixels);
                zoon::println!("   cursor_ns: {:.6}s", coords.cursor_ns.display_seconds());
                
                // ✅ VALIDATION: Ensure canvas width consistency across systems
                let dom_canvas_width = canvas_width_dom as u32;
                let cached_canvas_width = canvas_width as u32;
                if dom_canvas_width != cached_canvas_width {
                    zoon::println!("   ⚠️ CANVAS WIDTH MISMATCH: DOM={} != CACHED={}", dom_canvas_width, cached_canvas_width);
                }
                if coords.canvas_width_pixels != cached_canvas_width {
                    zoon::println!("   📏 COORDINATE WIDTH: {}px (reactive cache), {}px (canvas cache)", 
                        coords.canvas_width_pixels, cached_canvas_width);
                }
                
                // Convert mouse position to timeline time using pure integer arithmetic
                // ✅ CRITICAL FIX: Use corrected coordinates if ns_per_pixel was wrong
                let corrected_coords = if current_ns_per_pixel != correct_ns_per_pixel {
                    zoon::println!("   🔧 COORDINATE FIX: Using corrected ns_per_pixel {} instead of cached {}", correct_ns_per_pixel, current_ns_per_pixel);
                    TimelineCoordinates::new(
                        coords.cursor_ns,
                        coords.viewport_start_ns,
                        NsPerPixel(correct_ns_per_pixel),
                        coords.canvas_width_pixels
                    )
                } else {
                    coords
                };
                let clicked_time_ns = corrected_coords.mouse_to_time(click_x as u32);
                let viewport = match current_viewport() {
                    Some(viewport) => viewport,
                    None => return, // Timeline not initialized yet
                };
                
                // Essential coordinate calculation for cursor position
                let zoom_level_ms_per_pixel = corrected_coords.ns_per_pixel.nanos() as f64 / 1_000_000.0;
                
                // Get file bounds for clamping
                if let Some((file_min, file_max)) = get_current_timeline_range() {
                    let file_start_ns = TimeNs::from_external_seconds(file_min);
                    let file_end_ns = TimeNs::from_external_seconds(file_max);
                    // Clamp cursor to file bounds
                    let clamped_time_ns = TimeNs(clicked_time_ns.nanos().clamp(file_start_ns.nanos(), file_end_ns.nanos()));
                    
                    // Update global cursor position through domain
                    set_cursor_position(clamped_time_ns);
                    
                    // Emit cursor clicked event to WaveformTimeline domain
                    let waveform_timeline = crate::actors::waveform_timeline_domain();
                    waveform_timeline.cursor_clicked_relay.send(clamped_time_ns);
                    
                    // Synchronize direct animation to prevent jumps  
                    let clamped_time_seconds = clamped_time_ns.display_seconds();
                    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
                    animation.current_position = clamped_time_seconds;
                    animation.target_position = clamped_time_seconds;
                    animation.is_animating = false; // Stop any ongoing animation
                    
                    // Immediately redraw canvas with new cursor position
                    canvas_wrapper_for_click.borrow_mut().update_objects(move |objects| {
                        let selected_vars = crate::actors::selected_variables::current_variables();
                        let novyui_theme = CURRENT_THEME_CACHE.get();
                        // FIXED: Use cursor position for both cursor and zoom center (cursor-centered zooming)
                        *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, clamped_time_seconds, clamped_time_seconds);
                    });
                }
            }
        })
        .event_handler(move |event: events::PointerMove| {
            // Track mouse position using WaveformTimeline domain
            let page_mouse_x = event.x();
            let page_mouse_y = event.y();
            
            // Mouse coordinates captured for timeline interaction
            
            // Get canvas element's position relative to page
            let canvas_element = match event.target() {
                Some(target) => target,
                None => {
                    return;
                }
            };
            let canvas_rect = match canvas_element.dyn_into::<web_sys::Element>() {
                Ok(element) => element.get_bounding_client_rect(),
                Err(_) => {
                    return;
                }
            };
            let canvas_left = canvas_rect.left();
            let canvas_top = canvas_rect.top();
            
            // Calculate mouse position relative to canvas
            let mouse_x = page_mouse_x as f64 - canvas_left;
            let mouse_y = page_mouse_y as f64 - canvas_top;
            
            // Emit mouse move event to WaveformTimeline domain and get actual Actor state
            let waveform_timeline = crate::actors::waveform_timeline_domain();
            
            // Convert mouse X position to timeline time - debug coordinate consistency
            let coords = match current_coordinates() {
                Some(coords) => coords,
                None => return, // Timeline not initialized yet
            };
            let mouse_time = coords.mouse_to_time(mouse_x as u32);
            waveform_timeline.mouse_moved_relay.send((mouse_x as f32, mouse_time));
            
            // Update zoom center to follow mouse position (blue vertical line)
            crate::actors::waveform_timeline::set_zoom_center_follow_mouse(mouse_time);
            
            // TODO: Remove when domain handles all mouse tracking
            MOUSE_X_POSITION.set_neq(mouse_x as f32);
            
            // Convert mouse X to timeline time using TimelineCoordinates
            let canvas_width = match current_canvas_width() {
                Some(width) => width,
                None => return, // Timeline not initialized yet, skip this hover
            };
            let canvas_height = current_canvas_height();
            
            // Use the same Actor coordinates we calculated above for consistency
            // (coords variable already contains the real Actor state)
            
            // Ensure mouse_x is within canvas bounds
            if mouse_x >= 0.0 && mouse_x <= canvas_width as f64 {
                let mouse_time_ns = coords.mouse_to_time(mouse_x as u32);
                
                // Get file bounds for validation
                if let Some((file_min, file_max)) = get_current_timeline_range() {
                    let file_start_ns = TimeNs::from_external_seconds(file_min);
                    let file_end_ns = TimeNs::from_external_seconds(file_max);
                    
                    // Clamp mouse time to file bounds
                    let clamped_mouse_time_ns = TimeNs(mouse_time_ns.nanos().clamp(file_start_ns.nanos(), file_end_ns.nanos()));
                    
                    // Coordinate validation for tooltip calculation
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static HOVER_DEBUG_COUNTER: AtomicU32 = AtomicU32::new(0);
                    let counter = HOVER_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    
                    // TODO: Replace with domain-only access
                    MOUSE_TIME_NS.set_neq(clamped_mouse_time_ns);
                    // REMOVED: ZOOM_CENTER_NS.set_neq(clamped_mouse_time_ns); - Using Actor+Relay system instead
                    
                    // Calculate hover info for tooltip (within valid time range)
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let total_rows = selected_vars.len() + 1; // variables + timeline
                    let row_height = if total_rows > 0 { canvas_height / total_rows as f32 } else { canvas_height };
                    
                    // Determine which variable row the mouse is over
                    let hover_row = (mouse_y as f32 / row_height) as usize;
                    
                    if hover_row < selected_vars.len() {
                        // Mouse is over a variable row
                        let var = &selected_vars[hover_row];
                        let variable_name = var.unique_id.split('|').last().unwrap_or("Unknown").to_string();
                        
                        
                        let time_value_pairs = get_signal_transitions_for_variable(var, (file_min, file_max));
                        let mouse_time_seconds = clamped_mouse_time_ns.display_seconds();
                        
                        
                        // Find the value at the current mouse time
                        let mut current_value = shared::SignalValue::present("X"); // Default unknown
                        let mut found_transition_time = None;
                        for (time, value) in time_value_pairs.iter() {
                            if (*time as f64) <= mouse_time_seconds {
                                current_value = value.clone();
                                found_transition_time = Some(*time);
                            } else {
                                break;
                            }
                        }
                        
                        
                        // Format the value using the variable's formatter
                        let formatted_value = match current_value {
                            shared::SignalValue::Present(ref value) => {
                                let formatter = var.formatter.unwrap_or_default();
                                let result = formatter.format(value);
                                // Value formatting for tooltip
                                result
                            },
                            shared::SignalValue::Missing => {
                                // Missing value handling
                                "N/A".to_string()
                            }
                        };
                        
                        let hover_info = HoverInfo {
                            mouse_x: mouse_x as f32,
                            mouse_y: mouse_y as f32,
                            time: mouse_time_seconds as f32,
                            variable_name: variable_name.clone(),
                            value: formatted_value.clone(),
                        };
                        
                        // Hover info calculated for tooltip display
                        
                        HOVER_INFO.set_neq(Some(hover_info));
                    } else {
                        // Mouse is over timeline or outside variable area
                        HOVER_INFO.set_neq(None);
                    }
                } else {
                    // No file bounds available - clear hover info
                    HOVER_INFO.set_neq(None);
                }
            } else {
                // Mouse outside canvas bounds - clear hover info
                HOVER_INFO.set_neq(None);
            }
        })
    })
}




// Get signal transitions for a variable within time range
fn get_signal_transitions_for_variable(var: &SelectedVariable, time_range: (f64, f64)) -> Vec<(f32, shared::SignalValue)> {
    // Reduced debug logging for signal transitions
    
    // Parse unique_id: "/path/file.ext|scope|variable"
    let parts: Vec<&str> = var.unique_id.split('|').collect();
    if parts.len() < 3 {
        zoon::println!("❌ GET_SIGNAL_TRANSITIONS: Invalid unique_id format: {}", var.unique_id);
        return vec![(time_range.0 as f32, shared::SignalValue::present("0"))];
    }
    
    let file_path = parts[0];
    let scope_path = parts[1]; 
    let variable_name = parts[2];
    
    
    // Create cache key for processed canvas data (includes time range for accurate caching)
    let processed_cache_key = format!("{}|{}|{}|{:.6}|{:.6}", file_path, scope_path, variable_name, time_range.0, time_range.1);
    
    // Check processed canvas cache first - this prevents redundant processing and backend requests
    // TODO: Use processed_canvas_cache_signal() from waveform_timeline
    // let processed_cache = PROCESSED_CANVAS_CACHE.lock_ref();
    let processed_cache: std::collections::HashMap<String, Vec<SignalTransition>> = std::collections::HashMap::new(); // Temporary placeholder
    if let Some(cached_transitions) = processed_cache.get(&processed_cache_key) {
        // HIT: Data already processed for this exact time range, convert to expected format
        // Using processed cache
        return cached_transitions.iter()
            .map(|transition| ((transition.time_ns as f64 / 1_000_000_000.0) as f32, shared::SignalValue::present(transition.value.clone())))
            .collect();
    }
    drop(processed_cache);
    
    // Not in processed cache, check raw backend data cache
    let raw_cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    // Checking raw cache
    if let Some(transitions) = crate::unified_timeline_service::UnifiedTimelineService::get_raw_transitions(&raw_cache_key) {
        // Found raw transitions in cache
        
        // Convert real backend data to canvas format with proper waveform logic
        // Include ALL transitions to determine proper rectangle boundaries
        let mut canvas_transitions: Vec<(f32, shared::SignalValue)> = transitions.iter()
            .map(|t| ((t.time_ns as f64 / 1_000_000_000.0) as f32, shared::SignalValue::present(t.value.clone())))
            .collect();
            
        // Sort by time to ensure proper ordering
        canvas_transitions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // Check for time ranges beyond file boundaries and mark as missing
        let loaded_files = LOADED_FILES.lock_ref();
        if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path) {
            if let Some(max_time) = loaded_file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0) {
                // If the timeline extends beyond the file's max time, add missing data transition
                if time_range.1 > max_time {
                    canvas_transitions.push((max_time as f32, shared::SignalValue::missing()));
                }
            }
            if let Some(min_time) = loaded_file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0) {
                // If timeline starts before file's min time, add missing data at start
                if time_range.0 < min_time {
                    canvas_transitions.insert(0, (time_range.0 as f32, shared::SignalValue::missing()));
                    canvas_transitions.insert(1, (min_time as f32, shared::SignalValue::present("X")));
                }
            }
        }
        drop(loaded_files);
        
        // CRITICAL FIX: Always add initial value continuation at timeline start
        // Find what value should be active at the beginning of the visible timeline
        let mut initial_value = shared::SignalValue::present("X"); // Default unknown state
        
        // Find the most recent transition before the visible timeline starts
        for transition in transitions.iter() {
            if transition.time_ns as f64 / 1_000_000_000.0 <= time_range.0 {
                initial_value = shared::SignalValue::present(transition.value.clone());
            } else {
                break; // Transitions should be in time order
            }
        }
        
        // Check if we need to add initial continuation rectangle
        let needs_initial_continuation = canvas_transitions.is_empty() || 
            canvas_transitions[0].0 > time_range.0 as f32;
        
        if needs_initial_continuation {
            // Insert initial value at timeline start
            canvas_transitions.insert(0, (time_range.0 as f32, initial_value.clone()));
        }
        
        // Handle empty transitions case (keep existing logic for compatibility)
        if canvas_transitions.is_empty() && !transitions.is_empty() {
            canvas_transitions.push((time_range.0 as f32, initial_value.clone()));
            canvas_transitions.push((time_range.1 as f32, initial_value));
        }
        
        // Backend now provides proper signal termination transitions - no frontend workaround needed
        
        // Cache the processed canvas transitions for future redraws
        // TODO: Update canvas cache through WaveformTimeline actor
        // PROCESSED_CANVAS_CACHE.lock_mut().insert(processed_cache_key, canvas_transitions.clone());
        
        // Removed signal transitions debug spam - called frequently during rendering
        return canvas_transitions;
    }
    
    // No cached data - request from backend (deduplication removed, now handled by batching)
    crate::debug_utils::debug_cache_miss(&format!("requesting from backend for {}/{}", scope_path, variable_name));
    request_signal_transitions_from_backend(file_path, scope_path, variable_name, (time_range.0 as f32, time_range.1 as f32));
    
    // Return empty data while waiting for real backend response
    // This prevents premature filler rectangles from covering actual values
    zoon::println!("❌ GET_SIGNAL_TRANSITIONS: No cached data found, returning empty vec for {}", variable_name);
    vec![]
}

// Request real signal transitions from backend
pub fn request_signal_transitions_from_backend(file_path: &str, scope_path: &str, variable_name: &str, _time_range: (f32, f32)) {
    let _ = _time_range; // Suppress unused variable warning
    
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

// Trigger canvas redraw when new signal data arrives
pub fn trigger_canvas_redraw() {
    // Throttle redraws to prevent excessive backend requests
    let _now = js_sys::Date::now();
    // TODO: Use last_redraw_time_signal() from waveform_timeline for throttling
    // For now, always trigger redraw (throttling will be handled by WaveformTimeline actor)
    crate::actors::waveform_timeline::redraw_requested_relay().send(());
}

// Extract unique file paths from selected variables
fn get_selected_variable_file_paths() -> std::collections::HashSet<String> {
    let selected_vars = crate::actors::selected_variables::current_variables();
    let mut file_paths = std::collections::HashSet::new();
    
    // ITERATION 5: Track file path consistency between calls (simplified approach)
    use std::sync::OnceLock;
    static PREVIOUS_FILE_PATHS: OnceLock<std::sync::Mutex<Option<std::collections::HashSet<String>>>> = OnceLock::new();
    
    for var in selected_vars.iter() {
        // Parse unique_id: "file_path|scope|variable"
        if let Some(file_path) = var.unique_id.split('|').next() {
            file_paths.insert(file_path.to_string());
        }
    }
    
    // ITERATION 5: Check if file paths changed from previous call
    let file_paths_vec: Vec<String> = file_paths.iter().cloned().collect();
    
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

// ROCK-SOLID coordinate transformation system with zoom reliability
// Returns None when no variables are selected (no timeline should be shown)
pub fn get_current_timeline_range() -> Option<(f64, f64)> {
    // Removed debug spam - function called very frequently during rendering
    let ns_per_pixel = match current_ns_per_pixel() {
        Some(ns_per_pixel) => ns_per_pixel,
        None => {
            zoon::println!("🔍 GET_CURRENT_TIMELINE_RANGE: Timeline not yet initialized, returning None");
            return None;
        }
    };
    
    // FIXED: Always use viewport range for waveform rendering (no zoom level threshold)
    // This ensures transition rectangles use proper viewport boundaries
    let mut viewport = match current_viewport() {
        Some(viewport) => viewport,
        None => {
            zoon::println!("🔍 GET_CURRENT_TIMELINE_RANGE: Viewport not yet initialized, returning None");
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
    let canvas_width = match current_canvas_width() {
        Some(width) => width as u32,
        None => {
            zoon::println!("🔍 GET_CURRENT_TIMELINE_RANGE: Canvas width not yet initialized, returning None");
            return None;
        }
    };
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
                zoon::println!("   ⚠️  Current range too narrow, attempting expansion");
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
                zoon::println!("   ❌ Current range <= 0: {:.6}s", current_range);
            }
        } else {
            zoon::println!("   ❌ Non-finite values: start.is_finite()={}, end.is_finite()={}", 
                range_start.is_finite(), range_end.is_finite());
        }
    } else {
        zoon::println!("   ❌ Basic validation failed: range_end > range_start = {}, range_start >= 0.0 = {}", 
            range_end > range_start, range_start >= 0.0);
    }
    
    // ✅ STARTUP FIX: Prioritize actual file data when available, even if no variables selected
    
    // STEP 1: If we have loaded files with good data, use them directly (bypass selected variables dependency)
    let loaded_files = LOADED_FILES.lock_ref();
    if !loaded_files.is_empty() {
        // Use get_full_file_range() to get actual VCD file bounds (0-250s) regardless of selection
        let (full_file_min, full_file_max) = get_full_file_range();
        let file_span = full_file_max - full_file_min;
        
        zoon::println!("🔍 GET_CURRENT_TIMELINE_RANGE DEBUG: Fallback section reached");
        zoon::println!("   Loaded files: {} files", loaded_files.len());
        zoon::println!("   Full file range: {:.3}s to {:.3}s (span: {:.3}s)", full_file_min, full_file_max, file_span);
        
        // If we have substantial file data (not just microsecond ranges), use it immediately
        if file_span > 10.0 {  // More than 10 seconds suggests VCD file with real timeline data
            zoon::println!("   ✅ USING FULL FILE RANGE: file_span ({:.3}s) > 10.0s threshold", file_span);
            return Some((full_file_min, full_file_max));
        }
    }
    
    // STEP 2: Fall back to selected variables approach (original R key logic)
    let (r_key_min, r_key_max) = get_selected_variables_file_range();
    
    // Validate the range is sensible
    if r_key_max > r_key_min && r_key_min >= 0.0 && (r_key_max - r_key_min) > 0.001 {
        return Some((r_key_min, r_key_max));
    } else {
    }

    // ORIGINAL LOGIC: Default behavior: get range from files containing selected variables only
    let loaded_files = LOADED_FILES.lock_ref();
    
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
        for (file, file_min, file_max, _span) in file_candidates {
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
        for (file, file_min, file_max, span) in file_candidates {
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
    let canvas_width = match current_canvas_width() {
        Some(width) => width as u32,
        None => return Some((min_time, max_time)), // Timeline not initialized, return basic range
    };
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
        let zoom_level_us = if let Some(ns_per_pixel) = current_ns_per_pixel() {
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
    let loaded_files = LOADED_FILES.lock_ref();
    
    // Get file paths that contain selected variables
    let selected_file_paths = get_selected_variable_file_paths();
    
    
    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;
    let mut has_valid_files = false;
    
    // If no variables are selected, use full file range for viewport initialization
    if selected_file_paths.is_empty() {
        zoon::println!("🎯 GET_MAXIMUM_TIMELINE_RANGE: No variables selected, using full file range for viewport");
        let (file_min, file_max) = get_full_file_range();
        if file_min < file_max && file_min.is_finite() && file_max.is_finite() {
            zoon::println!("🎯 GET_MAXIMUM_TIMELINE_RANGE: Returning full range {:.6}s to {:.6}s", file_min, file_max);
            return Some((file_min, file_max));
        } else {
            zoon::println!("🎯 GET_MAXIMUM_TIMELINE_RANGE: Invalid file range, returning None");
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
    let canvas_width = match current_canvas_width() {
        Some(width) => width as u32,
        None => return Some((min_time, max_time)), // Timeline not initialized, return basic range
    };
    if file_range < get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0 {  // Only enforce minimum for truly tiny ranges (< 1 nanosecond)
        let expanded_end = min_time + get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
        if expanded_end.is_finite() {
            return Some((min_time, expanded_end));  // Minimum 1 nanosecond range
        } else {
            return None; // Ultimate fallback
        }
    } else {
        let result = (min_time, max_time);
        return Some(result);  // Use actual range
    }
}

// Smooth zoom functions with mouse-centered behavior
pub fn start_smooth_zoom_in() {
    // TODO: Replace with domain-only implementation
    // For now, keep legacy implementation and add domain relay
    zoom_in_started_relay().send(TimeNs::from_external_seconds(current_zoom_center_seconds()));
    
    // Legacy zoom flag check - will be replaced when domain zoom handling is complete
    if !IS_ZOOMING_IN.get() {
        IS_ZOOMING_IN.set_neq(true);
        Task::start(async move {
            while IS_ZOOMING_IN.get() {
                // COMPLETELY REWRITTEN: Follow timeline simplification plan exactly
                // Pure integer zoom algorithm with cursor-centered zooming
                
                let current_ns_per_pixel = if let Some(ns_per_pixel) = current_ns_per_pixel() {
                    ns_per_pixel.nanos()
                } else {
                    break; // Timeline not initialized yet, stop zooming
                };
                let current_viewport = if let Some(viewport) = current_viewport() {
                    viewport
                } else {
                    break; // Timeline not initialized yet, stop zooming
                };
                let canvas_width = match current_canvas_width() {
                    Some(width) => width as u32,
                    None => continue, // Timeline not initialized yet, skip this frame
                };
                
                // Calculate new resolution using pure integer math (timeline plan approach)
                let new_ns_per_pixel_value = if crate::state::IS_SHIFT_PRESSED.get() {
                    // Fast zoom: 90% of previous (10% zoom in per frame)
                    (current_ns_per_pixel * 9) / 10
                } else {
                    // Normal zoom: 95% of previous (5% zoom in per frame)  
                    (current_ns_per_pixel * 19) / 20
                };
                
                let new_ns_per_pixel = NsPerPixel(new_ns_per_pixel_value.max(1));
                
                if new_ns_per_pixel.nanos() != current_ns_per_pixel {
                    // ✅ FIXED: Use zoom center for zoom calculations (not cursor position)
                    let zoom_center_time_ns = TimeNs::from_external_seconds(current_zoom_center_seconds()).nanos();
                    
                    // Calculate zoom center position as pixels from viewport start
                    let viewport_start_ns = current_viewport.start.nanos();
                    let zoom_center_offset_ns = zoom_center_time_ns.saturating_sub(viewport_start_ns);
                    let zoom_center_x_pixels = (zoom_center_offset_ns / current_ns_per_pixel) as u32;
                    let zoom_center_x_pixels = zoom_center_x_pixels.min(canvas_width);
                    
                    // Calculate new viewport centered on zoom center (pure integer math)
                    let ns_before_zoom_center = zoom_center_x_pixels as u64 * new_ns_per_pixel.nanos();
                    let ns_after_zoom_center = (canvas_width.saturating_sub(zoom_center_x_pixels)) as u64 * new_ns_per_pixel.nanos();
                    
                    let new_viewport = Viewport::new(
                        TimeNs(zoom_center_time_ns.saturating_sub(ns_before_zoom_center)),
                        TimeNs(zoom_center_time_ns.saturating_add(ns_after_zoom_center))
                    );
                    
                    set_ns_per_pixel_if_changed(new_ns_per_pixel);
                    set_viewport_if_changed(new_viewport);
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
                if let Some(current_ns_per_pixel_val) = current_ns_per_pixel() {
                    // FIXED: Use pure integer math instead of floating-point factors
                    // Industry standard approach: increase ns_per_pixel by integer multiplication/division
                    let new_ns_per_pixel_value = if crate::state::IS_SHIFT_PRESSED.get() {
                        // Fast zoom out: multiply by 1.2 (equivalent to multiply by 6/5)
                        (current_ns_per_pixel_val.nanos() * 6) / 5  // 20% zoom out per frame
                    } else {
                        // Normal zoom out: multiply by 1.1 (equivalent to multiply by 11/10)
                        (current_ns_per_pixel_val.nanos() * 11) / 10  // 10% zoom out per frame
                    };
                    
                    let new_ns_per_pixel = NsPerPixel(new_ns_per_pixel_value);
                    if new_ns_per_pixel != current_ns_per_pixel_val {
                        // ✅ CORRECT: Use zoom center for zoom calculations (not cursor position)
                        let zoom_center_time_ns = TimeNs::from_external_seconds(current_zoom_center_seconds()).nanos();
                        if let Some(current_viewport) = current_viewport() {
                            let canvas_width = match current_canvas_width() {
                                Some(width) => width as u32,
                                None => continue, // Timeline not initialized yet, skip this frame
                            };
                            
                            // Calculate zoom center position as pixels from viewport start
                            let viewport_start_ns = current_viewport.start.nanos();
                            let zoom_center_offset_ns = zoom_center_time_ns.saturating_sub(viewport_start_ns);
                            let zoom_center_x_pixels = (zoom_center_offset_ns / current_ns_per_pixel_val.nanos()) as u32;
                            let zoom_center_x_pixels = zoom_center_x_pixels.min(canvas_width);
                            
                            // Calculate new viewport centered on zoom center (pure integer math)
                            let ns_before_zoom_center = zoom_center_x_pixels as u64 * new_ns_per_pixel.nanos();
                            let ns_after_zoom_center = (canvas_width.saturating_sub(zoom_center_x_pixels)) as u64 * new_ns_per_pixel.nanos();
                            
                            let new_viewport = Viewport::new(
                                TimeNs(zoom_center_time_ns.saturating_sub(ns_before_zoom_center)),
                                TimeNs(zoom_center_time_ns.saturating_add(ns_after_zoom_center))
                            );
                            
                            set_ns_per_pixel_if_changed(new_ns_per_pixel);
                            set_viewport_if_changed(new_viewport);
                        } else {
                            break; // Timeline not initialized yet, stop zooming
                        }
                    } else {
                        break; // Hit zoom limit
                    }
                } else {
                    break; // Timeline not initialized yet, stop zooming
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
                        let pan_pixels = if crate::state::IS_SHIFT_PRESSED.get() {
                            -10  // Turbo pan with Shift (10 pixels per frame)
                        } else {
                            -2   // Normal smooth pan (2 pixels per frame)
                        };
                        
                        // Store original viewport start for comparison
                        let original_start = coords.viewport_start_ns;
                        
                        // Pan by pixels (negative = pan left) using local coordinates
                        coords.pan_by_pixels(pan_pixels);
                        
                        // Get file bounds and clamp viewport
                        let (file_min, file_max) = get_full_file_range();
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
                        let pan_pixels = if crate::state::IS_SHIFT_PRESSED.get() {
                            10   // Turbo pan with Shift (10 pixels per frame)
                        } else {
                            2    // Normal smooth pan (2 pixels per frame)
                        };
                        
                        // Store original viewport start for comparison
                        let original_start = coords.viewport_start_ns;
                        
                        // Pan by pixels (positive = pan right) using local coordinates
                        coords.pan_by_pixels(pan_pixels);
                        
                        // Get file bounds and clamp viewport
                        let (file_min, file_max) = get_full_file_range();
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

pub fn stop_smooth_pan_left() {
    IS_PANNING_LEFT.set_neq(false);
}

pub fn stop_smooth_pan_right() {
    IS_PANNING_RIGHT.set_neq(false);
}

/// Validate and sanitize timeline range to prevent NaN propagation
fn validate_and_sanitize_range(start: f64, end: f64) -> (f64, f64) {
    // Check for NaN/Infinity in inputs
    if !start.is_finite() || !end.is_finite() {
        crate::debug_utils::debug_timeline_validation(&format!("Non-finite range detected - start: {}, end: {}, using actual file range", start, end));
        zoon::println!("🚨 FALLBACK ELIMINATION: Non-finite range: {}-{} → using actual file range", start, end);
        // ❌ FALLBACK ELIMINATION: Get actual file range instead of hardcoded fallback
        let (file_min, file_max) = get_full_file_range();
        return (file_min, file_max);
    }
    
    // Ensure proper ordering
    if start >= end {
        crate::debug_utils::debug_timeline_validation(&format!("Invalid range ordering - start: {} >= end: {}, using actual file range", start, end));
        zoon::println!("🚨 FALLBACK ELIMINATION: Invalid ordering: {} >= {} → using actual file range", start, end);
        // ❌ FALLBACK ELIMINATION: Get actual file range instead of hardcoded fallback
        let (file_min, file_max) = get_full_file_range();
        return (file_min, file_max);
    }
    
    // Enforce minimum viable range based on maximum zoom level
    let range = end - start;
    let canvas_width = match current_canvas_width() {
        Some(width) => width as u32,
        None => return (start, end), // Timeline not initialized, return as-is
    };
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

/// Simple cursor movement using TimelineCoordinates - replaces complex fallback algorithms
fn move_cursor_by_pixels(pixel_offset: i32, current_time_ns: TimeNs) -> Option<TimeNs> {
    let coords = match current_coordinates() {
        Some(coords) => coords,
        None => return None, // Timeline not initialized yet
    };
    
    // Convert current time to pixel position
    if let Some(current_pixel) = coords.time_to_pixel(current_time_ns) {
        // Calculate new pixel position
        let new_pixel = (current_pixel as i32).saturating_add(pixel_offset);
        if new_pixel >= 0 {
            // Convert back to time
            let new_time_ns = coords.mouse_to_time(new_pixel as u32);
            
            // Clamp to file bounds if available
            if let Some((file_min, file_max)) = get_current_timeline_range() {
                let file_start_ns = TimeNs::from_external_seconds(file_min);
                let file_end_ns = TimeNs::from_external_seconds(file_max);
                let clamped_time_ns = TimeNs(new_time_ns.nanos().clamp(file_start_ns.nanos(), file_end_ns.nanos()));
                Some(clamped_time_ns)
            } else {
                Some(new_time_ns)
            }
        } else {
            // Hit left boundary
            Some(TimeNs::ZERO)
        }
    } else {
        // Current time is outside viewport - use simple time-based fallback
        let time_step_ns = 1_000_000; // 1 millisecond step
        let new_time_nanos = if pixel_offset > 0 {
            current_time_ns.nanos().saturating_add(time_step_ns)
        } else {
            current_time_ns.nanos().saturating_sub(time_step_ns)
        };
        Some(TimeNs(new_time_nanos))
    }
}

/// Synchronize TimelineCoordinates with global timeline state
/// Called whenever global state changes to keep coordinates in sync

// High-performance direct cursor animation loop
fn start_direct_cursor_animation_loop() {
    Task::start(async move {
        loop {
            Timer::sleep(MOVEMENT_FRAME_MS).await;
            
            let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
            if !animation.is_animating {
                drop(animation);
                continue;
            }
            
            // Calculate new position using TimelineCoordinates
            let pixel_delta = (animation.direction as f64 * animation.velocity_pixels_per_frame) as i32;
            let current_time_ns = TimeNs::from_external_seconds(animation.current_position);
            
            if let Some(new_time_ns) = move_cursor_by_pixels(pixel_delta, current_time_ns) {
                let new_time_seconds = new_time_ns.display_seconds();
                
                // Update animation state
                animation.current_position = new_time_seconds;
                drop(animation);
                
                // Update timeline cursor with debouncing
                update_timeline_cursor_with_debouncing(new_time_seconds);
            } else {
                // Stop animation if movement fails (shouldn't happen with new system)
                animation.is_animating = false;
                drop(animation);
            }
        }
    });
}

// Smart cursor update with debouncing to reduce canvas redraw overhead
fn update_timeline_cursor_with_debouncing(new_position: f64) {
    let time_ns = crate::time_types::TimeNs::from_external_seconds(new_position);
    crate::actors::waveform_timeline::set_cursor_position_if_changed(time_ns);
    
    // Debounce canvas updates - only redraw every 16ms maximum
    let _now = get_current_time_ns();
    // TODO: Use last_canvas_update_signal() from waveform_timeline for debouncing
    // For now, always trigger redraw (throttling will be handled by WaveformTimeline actor)
    if !PENDING_CANVAS_UPDATE.get() {
        PENDING_CANVAS_UPDATE.set_neq(true);
        Task::start(async move {
            Timer::sleep(MOVEMENT_FRAME_MS).await;
            PENDING_CANVAS_UPDATE.set_neq(false);
            trigger_canvas_redraw();
        });
    }
}

// Get current time in nanoseconds for high-precision timing (WASM-compatible)
fn get_current_time_ns() -> u64 {
    // Use performance.now() in WASM which provides high-precision timestamps
    (js_sys::Date::now() * 1_000_000.0) as u64  // Convert milliseconds to nanoseconds
}

// High-performance cursor movement functions
pub fn start_smooth_cursor_left() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = -1;
    animation.is_animating = true;
    animation.current_position = current_cursor_position_seconds().unwrap_or(0.0);
    IS_CURSOR_MOVING_LEFT.set_neq(true);
}

pub fn start_smooth_cursor_right() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = 1;
    animation.is_animating = true;
    animation.current_position = current_cursor_position_seconds().unwrap_or(0.0);
    IS_CURSOR_MOVING_RIGHT.set_neq(true);
}

pub fn stop_smooth_cursor_left() {
    IS_CURSOR_MOVING_LEFT.set_neq(false);
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    if animation.direction == -1 {
        animation.is_animating = false;
        animation.direction = 0;
    }
}

pub fn stop_smooth_cursor_right() {
    IS_CURSOR_MOVING_RIGHT.set_neq(false);
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    if animation.direction == 1 {
        animation.is_animating = false;
        animation.direction = 0;
    }
}

// Removed old detect_time_unit_minimum - replaced with calculate_time_step() for hybrid cursor movement




pub fn get_full_file_range() -> (f64, f64) {
    // ✅ FIXED: Break circular dependency with get_maximum_timeline_range()
    // Calculate full file range directly from loaded files without selection dependency

    let loaded_files = LOADED_FILES.lock_ref();
    
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
    if let Some((file, file_min, file_max, span)) = file_candidates.first() {
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

fn get_selected_variables_file_range() -> (f64, f64) {
    use std::collections::HashSet;
    
    let selected_variables = crate::actors::selected_variables::current_variables();
    let loaded_files = LOADED_FILES.lock_ref();
    
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
        zoon::println!("   🔄 NO SELECTED VARIABLES - falling back to get_full_file_range()");
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
    if let Some((file, file_min, file_max, span_s)) = file_candidates.first() {
        zoon::println!("      ✅ USING LONGEST SPAN FILE '{}': {:.6}s to {:.6}s (span: {:.6}s)", file.id, file_min, file_max, span_s);
        if *span_s < 0.01 {
            zoon::println!("         🚨 CRITICAL: This file has microsecond range - would cause 700μs/px zoom!");
        } else if *span_s > 100.0 {
            zoon::println!("         ✅ EXCELLENT: This file has long timeline range - will create proper zoom levels!");
        }
        
        // Use ONLY the longest file's range, don't combine with others
        min_time = *file_min;
        max_time = *file_max;
        has_valid_files = true;
        
        // Log skipped shorter files for debugging
        for (skipped_file, skipped_min, skipped_max, skipped_span) in file_candidates.iter().skip(1) {
            zoon::println!("      ⏭️  SKIPPED shorter file '{}': {:.6}s to {:.6}s (span: {:.6}s)", 
                skipped_file.id, skipped_min, skipped_max, skipped_span);
        }
    }
    
    // Log skipped files for debugging
    // Process files that contain selected variables
    
    if !has_valid_files || min_time == max_time {
        // No valid files with selected variables - fall back to full file range
        return get_full_file_range();
    } else {
        let result = (min_time, max_time);
        let total_span = result.1 - result.0;
        
        
        result
    }
}

// PERFORMANCE FIX: Incremental canvas updates to prevent full object recreation
fn detect_variable_changes(current_vars: &[SelectedVariable]) -> (Vec<String>, Vec<SelectedVariable>) {
    let last_vars = LAST_VARIABLES_STATE.get_cloned();
    
    // Find variables that were removed
    let mut removed_var_ids = Vec::new();
    for old_var in &last_vars {
        if !current_vars.iter().any(|v| v.unique_id == old_var.unique_id) {
            removed_var_ids.push(old_var.unique_id.clone());
        }
    }
    
    // Find variables that were added or changed
    let mut changed_vars = Vec::new();
    for current_var in current_vars {
        // Check if this is a new variable or if it changed
        if let Some(old_var) = last_vars.iter().find(|v| v.unique_id == current_var.unique_id) {
            // Check if variable properties changed (formatter, etc.)
            if old_var.formatter != current_var.formatter {
                changed_vars.push(current_var.clone());
            }
        } else {
            // This is a new variable
            changed_vars.push(current_var.clone());
        }
    }
    
    (removed_var_ids, changed_vars)
}

fn create_objects_for_single_variable(
    var: &SelectedVariable,
    var_index: usize,
    canvas_width: f32,
    canvas_height: f32,
    theme: &NovyUITheme,
    cursor_position: f64,
    total_vars: usize
) -> Vec<fast2d::Object2d> {
    let mut objects = Vec::new();
    
    // Calculate row layout (same as original function)
    let total_rows = total_vars + 1; // variables + timeline
    let row_height = if total_rows > 0 { canvas_height / total_rows as f32 } else { canvas_height };
    let y_position = var_index as f32 * row_height;
    let is_even_row = var_index % 2 == 0;
    
    // Get theme colors
    let theme_colors = get_current_theme_colors(theme);
    let background_color = if is_even_row {
        theme_colors.neutral_2
    } else {
        theme_colors.neutral_3
    };
    
    // Create row background
    objects.push(
        fast2d::Rectangle::new()
            .position(0.0, y_position)
            .size(canvas_width, row_height)
            .color(background_color.0, background_color.1, background_color.2, background_color.3)
            .into()
    );
    
    // Get time range and signal data (same logic as original)
    let Some(current_time_range) = get_current_timeline_range() else {
        return objects;
    };
    
    let time_value_pairs = get_signal_transitions_for_variable(var, current_time_range);
    let (min_time, max_time) = current_time_range;
    
    // Create value rectangles (same logic as original function)
    for (rect_index, (start_time, signal_value)) in time_value_pairs.iter().enumerate() {
        let end_time = if rect_index + 1 < time_value_pairs.len() {
            time_value_pairs[rect_index + 1].0
        } else {
            // FIXED: Last rectangle extends only to current viewport end, not full timeline
            // This prevents massive rectangles when zoomed in
            match current_viewport() {
                Some(viewport) => viewport.end.display_seconds() as f32,
                None => continue, // Skip rendering if viewport not initialized
            }
        };
        
        if end_time <= min_time as f32 || *start_time >= max_time as f32 {
            continue;
        }
        
        let visible_start_time = start_time.max(min_time as f32);
        let visible_end_time = end_time.min(max_time as f32);
        let time_range = max_time - min_time;
        
        if time_range <= 0.0 || canvas_width <= 0.0 {
            continue;
        }
        
        let time_to_pixel_ratio = canvas_width as f64 / time_range;
        let rect_start_x = (visible_start_time as f64 - min_time) * time_to_pixel_ratio;
        let rect_end_x = (visible_end_time as f64 - min_time) * time_to_pixel_ratio;
        let raw_rect_width = rect_end_x - rect_start_x;
        
        // Skip sub-pixel rectangles for performance
        if raw_rect_width < 2.0 {
            if rect_start_x < -10.0 || rect_start_x > canvas_width as f64 + 10.0 {
                continue;
            }
            // Create transition line indicator using Rectangle (same as original)
            let line_x = rect_start_x.max(0.0).min(canvas_width as f64 - 1.0);
            objects.push(
                fast2d::Rectangle::new()
                    .position(line_x as f32, y_position)
                    .size(1.0, row_height) // 1 pixel wide vertical line
                    .color(theme_colors.cursor_color.0, theme_colors.cursor_color.1, theme_colors.cursor_color.2, 0.8)
                    .into()
            );
            continue;
        }
        
        // Create value rectangle with proper color encoding (using available theme colors)
        let color = match signal_value {
            shared::SignalValue::Present(_) => theme_colors.value_color_1,
            shared::SignalValue::Missing => theme_colors.value_color_2,
        };
        
        objects.push(
            fast2d::Rectangle::new()
                .position(rect_start_x as f32, y_position + row_height * 0.2)
                .size(raw_rect_width as f32, row_height * 0.6)
                .color(color.0, color.1, color.2, color.3)
                .into()
        );
    }
    
    // Add cursor indicator if cursor is in this row's time range
    let time_range = max_time - min_time;
    if time_range > 0.0 && cursor_position >= min_time && cursor_position <= max_time {
        let cursor_x = ((cursor_position - min_time) / time_range * canvas_width as f64) as f32;
        // Use cursor_color from theme (same as original implementation)
        objects.push(
            fast2d::Rectangle::new()
                .position(cursor_x - 1.0, y_position) // Center the 2px line
                .size(2.0, row_height) // 2px thick line for this row
                .color(theme_colors.cursor_color.0, theme_colors.cursor_color.1, theme_colors.cursor_color.2, 1.0)
                .into()
        );
    }
    
    objects
}

fn update_canvas_objects_incrementally(
    current_vars: &[SelectedVariable],
    canvas_width: f32,
    canvas_height: f32,
    theme: &NovyUITheme,
    cursor_position: f64
) -> Vec<fast2d::Object2d> {
    // Check if canvas dimensions changed - if so, force full redraw
    let current_dims = (canvas_width, canvas_height);
    let last_dims = LAST_CANVAS_DIMENSIONS.get();
    let dimension_changed = current_dims != last_dims;
    
    if dimension_changed {
        LAST_CANVAS_DIMENSIONS.set_neq(current_dims);
        // Clear cache and force full redraw
        VARIABLE_OBJECT_CACHE.lock_mut().clear();
        // ✅ FIXED: Use separate cursor and zoom center positions
        let zoom_center_position = current_zoom_center_seconds();
        return create_waveform_objects_with_dimensions_and_theme(
            current_vars, canvas_width, canvas_height, theme, cursor_position, zoom_center_position
        );
    }
    
    // Detect variable changes
    let (removed_var_ids, changed_vars) = detect_variable_changes(current_vars);
    
    // If no variables changed, return cached objects
    if removed_var_ids.is_empty() && changed_vars.is_empty() {
        let cache = VARIABLE_OBJECT_CACHE.lock_ref();
        let mut all_objects = Vec::new();
        
        // Collect objects in proper order (by variable index)
        for (_var_index, var) in current_vars.iter().enumerate() {
            if let Some(var_objects) = cache.get(&var.unique_id) {
                all_objects.extend(var_objects.clone());
            } else {
                // Variable not cached - need to create it
                drop(cache);
                // ✅ FIXED: Use separate cursor and zoom center positions
                let zoom_center_position = current_zoom_center_seconds();
                return create_waveform_objects_with_dimensions_and_theme(
                    current_vars, canvas_width, canvas_height, theme, cursor_position, zoom_center_position
                );
            }
        }
        
        return all_objects;
    }
    
    // Update cache for changed/removed variables
    {
        let mut cache = VARIABLE_OBJECT_CACHE.lock_mut();
        
        // Remove objects for deleted variables
        for removed_id in &removed_var_ids {
            cache.remove(removed_id);
        }
        
        // Update objects for changed variables
        for changed_var in &changed_vars {
            let var_index = current_vars.iter().position(|v| v.unique_id == changed_var.unique_id).unwrap();
            let var_objects = create_objects_for_single_variable(
                changed_var,
                var_index,
                canvas_width,
                canvas_height,
                theme,
                cursor_position,
                current_vars.len()
            );
            cache.insert(changed_var.unique_id.clone(), var_objects);
        }
    }
    
    // Update last variables state
    LAST_VARIABLES_STATE.set_neq(current_vars.to_vec());
    
    // Collect all objects in proper order
    let cache = VARIABLE_OBJECT_CACHE.lock_ref();
    let mut all_objects = Vec::new();
    
    for var in current_vars {
        if let Some(var_objects) = cache.get(&var.unique_id) {
            all_objects.extend(var_objects.clone());
        }
    }
    
    all_objects
}

fn create_waveform_objects_with_theme(selected_vars: &[SelectedVariable], theme: &NovyUITheme) -> Vec<fast2d::Object2d> {
    let cursor_pos = match current_cursor_position_seconds() {
        Some(pos) => pos,
        None => return Vec::new(), // Timeline not initialized yet, return empty objects
    };
    let zoom_center_pos = current_zoom_center_seconds();
    let canvas_width = match current_canvas_width() {
        Some(width) => width,
        None => return Vec::new(), // Timeline not initialized yet, return empty objects
    };
    let canvas_height = current_canvas_height();
    // ✅ FIXED: Use separate cursor and zoom center positions
    create_waveform_objects_with_dimensions_and_theme(selected_vars, canvas_width, canvas_height, theme, cursor_pos, zoom_center_pos)
}

fn create_waveform_objects_with_dimensions_and_theme(selected_vars: &[SelectedVariable], canvas_width: f32, canvas_height: f32, theme: &NovyUITheme, cursor_position: f64, zoom_center_position: f64) -> Vec<fast2d::Object2d> {
    let mut objects = Vec::new();
    
    // Canvas rendering starts
    
    // ✅ CRITICAL FIX: Use full file range for timeline scale, not current viewport
    // The timeline scale should show markers spanning the COMPLETE file duration (0-250s)
    // The viewport is just a zoom window within this full timeline
    let (timeline_min, timeline_max) = get_full_file_range();
    // Log occasionally to avoid spam
    static RENDER_LOG_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if RENDER_LOG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 10 == 0 {
        // Canvas rendering with full timeline range
    }
    
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
        
        let Some(current_time_range) = get_current_timeline_range() else { 
            // This should never happen now due to fallback logic, but add safety
            continue;
        };
        
        
        let time_value_pairs = get_signal_transitions_for_variable(var, current_time_range);
        
        // Timeline range already calculated in get_current_timeline_range()
        
        
        // Get visible time range for proper clipping
        let (min_time, max_time) = current_time_range;
        
        for (rect_index, (start_time, signal_value)) in time_value_pairs.iter().enumerate() {
            // Calculate end time for this rectangle (next transition time or view window end)
            let end_time = if rect_index + 1 < time_value_pairs.len() {
                time_value_pairs[rect_index + 1].0 // Next transition time
            } else {
                // FIXED: Last rectangle extends only to current viewport end, not full timeline
                // This prevents massive rectangles when zoomed in
                match crate::actors::waveform_timeline::current_viewport() {
                    Some(viewport) => viewport.end.display_seconds() as f32,
                    None => continue, // Skip if viewport not initialized
                }
            };
            
            // Skip rectangles completely outside visible range
            if end_time <= min_time as f32 || *start_time >= max_time as f32 {
                continue;
            }
            
            // Clip rectangle to visible time range
            let visible_start_time = start_time.max(min_time as f32);
            let visible_end_time = end_time.min(max_time as f32);
            
            // GRAPHICS-LEVEL FIX: Robust coordinate transformation with precision handling
            let time_range = max_time - min_time;
            
            // Prevent division by zero and handle degenerate cases
            if time_range <= 0.0 || canvas_width <= 0.0 {
                continue; // Skip rendering in degenerate cases
            }
            
            // FIXED: Use canvas-width-based coordinate calculation for full width coverage
            // This ensures rectangles span the full canvas width correctly
            let time_range = max_time - min_time;
            let time_to_pixel_ratio = canvas_width as f64 / time_range;
            let rect_start_x = (visible_start_time as f64 - min_time) * time_to_pixel_ratio;
            let rect_end_x = (visible_end_time as f64 - min_time) * time_to_pixel_ratio;
            
            // CRITICAL: Enforce minimum visible width to prevent zero rectangles
            let raw_rect_width = rect_end_x - rect_start_x;
            
            // Sub-pixel transition detection - switch to vertical line indicators
            if raw_rect_width < 2.0 {
                // Skip off-screen transitions for performance
                if rect_start_x < -10.0 || rect_start_x > canvas_width as f64 + 10.0 {
                    continue;
                }
                
                // Draw thin vertical line at transition point for sub-pixel transitions
                let line_x = rect_start_x.max(0.0).min(canvas_width as f64 - 1.0);
                
                // Get theme colors (we need to access it here since theme_colors is defined later)
                let transition_theme_colors = get_current_theme_colors(theme);
                let accent_color = transition_theme_colors.cursor_color; // Use cursor color for bright visibility
                
                objects.push(
                    fast2d::Rectangle::new()
                        .position(line_x as f32, y_position)
                        .size(1.0, row_height) // 1 pixel wide vertical line
                        .color(accent_color.0, accent_color.1, accent_color.2, accent_color.3)
                        .into()
                );
                
                continue; // Skip normal rectangle rendering for sub-pixel transitions
            }
            
            let min_visible_width = 1.0; // Minimum 1 pixel width for visibility
            let rect_width = raw_rect_width.max(min_visible_width);
            
            // Bounds checking: ensure rectangle fits within canvas
            let rect_start_x = rect_start_x.max(0.0).min(canvas_width as f64 - rect_width);
            let rect_end_x = rect_start_x + rect_width;
            
            // Validate final dimensions before creating Fast2D objects
            if rect_width <= 0.0 || rect_start_x >= canvas_width as f64 || rect_end_x <= 0.0 {
                continue; // Skip invalid rectangles
            }
            
            // Theme-aware colors that differentiate between present data and missing data
            let rect_color = match signal_value {
                shared::SignalValue::Present(_) => {
                    // Present data: use alternating colors as before
                    let is_even_rect = rect_index % 2 == 0;
                    if is_even_rect {
                        theme_colors.value_color_1
                    } else {
                        theme_colors.value_color_2
                    }
                },
                shared::SignalValue::Missing => {
                    // Missing data: use muted background color to indicate no data
                    theme_colors.neutral_2
                }
            };
            
            // Create value rectangle with actual time-based width
            // Reduced waveform rect creation debug logs
            objects.push(
                fast2d::Rectangle::new()
                    .position(rect_start_x as f32, y_position + 2.0)
                    .size(rect_width as f32, row_height - 4.0)
                    .color(rect_color.0, rect_color.1, rect_color.2, rect_color.3)
                    .into()
            );
            
            // TEMPORARY: Border drawing disabled to test color lightening issue
            // objects.push(
            //     fast2d::Rectangle::new()
            //         .position(rect_start_x - 0.5, y_position + 1.5)
            //         .size(rect_width + 1.0, row_height - 3.0)
            //         .color(theme_colors.border_color.0, theme_colors.border_color.1, theme_colors.border_color.2, theme_colors.border_color.3)
            //         .into()
            // );
            
            // Format the signal value and determine colors based on whether data is present or missing
            let (formatted_value, text_color) = match signal_value {
                shared::SignalValue::Present(binary_value) => {
                    // Present data: use normal formatting and high contrast text
                    (format.format(&binary_value), theme_colors.neutral_12)
                },
                shared::SignalValue::Missing => {
                    // Missing data: show "N/A" with muted color
                    ("N/A".to_string(), theme_colors.neutral_3)
                }
            };
            
            // Add formatted value text with robust positioning
            let text_padding = 5.0;
            let text_width = (rect_width - (text_padding * 2.0)).max(0.0);
            let text_height = (row_height / 2.0).max(8.0); // Minimum readable height
            
            // Only render text if there's sufficient space
            if text_width >= 10.0 && text_height >= 8.0 {
                objects.push(
                    fast2d::Text::new()
                        .text(formatted_value)
                        .position((rect_start_x + text_padding) as f32, (y_position + row_height / 3.0) as f32)
                        .size(text_width as f32, text_height as f32)
                        .color(text_color.0, text_color.1, text_color.2, text_color.3)
                        .font_size(11.0)
                        .family(fast2d::Family::name("Fira Code")) // FiraCode monospace font
                        .into()
                );
            }
        }
        
        // Add horizontal row separator line below each variable row
        if index < selected_vars.len() - 1 { // Don't add separator after last variable
            let separator_y = (index + 1) as f32 * row_height;
            objects.push(
                fast2d::Rectangle::new()
                    .position(0.0, separator_y - 0.5)
                    .size(canvas_width, 1.0)
                    .color(theme_colors.separator_color.0, theme_colors.separator_color.1, theme_colors.separator_color.2, theme_colors.separator_color.3)
                    .into()
            );
        }
    }
    
    // Create timeline row background (last row) using theme-aware color
    if total_rows > 0 {
        let timeline_y = (total_rows - 1) as f32 * row_height;
        
        let timeline_bg_color = theme_colors.neutral_2; // Match panel background for transparency effect
        objects.push(
            fast2d::Rectangle::new()
                .position(0.0, timeline_y)
                .size(canvas_width, row_height)
                .color(timeline_bg_color.0, timeline_bg_color.1, timeline_bg_color.2, 1.0) // Solid color like dropdowns
                .into()
        );
        
        // Use the corrected timeline range (already calculated above with fallback detection)
        let (min_time, max_time) = (timeline_min, timeline_max);
        let time_range = max_time - min_time;
        
        // Timeline markers using range {:.3}s to {:.3}s (span: {:.3}s)
        
        // Determine appropriate time unit for the entire range
        let time_unit = get_time_unit_for_range(min_time, max_time);
        
        // Phase 9: Pixel-based spacing algorithm for professional timeline
        let target_tick_spacing = 60.0; // Target 60 pixels between ticks
        let max_tick_count = (canvas_width / target_tick_spacing).floor() as i32;
        let tick_count = max_tick_count.max(2).min(8); // Ensure 2-8 ticks
        
        // Calculate round time intervals
        let raw_time_step = time_range / (tick_count - 1) as f64;
        let time_step = round_to_nice_number(raw_time_step as f32) as f64;
        
        // Find the first tick that's >= min_time (aligned to step boundaries)
        let first_tick = (min_time / time_step).ceil() * time_step;
        let last_tick = max_time;
        let actual_tick_count = ((last_tick - first_tick) / time_step).ceil() as i32 + 1;
        
        // Timeline markers: {} ticks with step {:.3}s, from {:.3}s to {:.3}s
        
        for tick_index in 0..actual_tick_count {
            let time_value = first_tick + (tick_index as f64 * time_step);
            let time_value = time_value.min(max_time);
            let x_position = ((time_value - min_time) / time_range) * canvas_width as f64;
            
            // Skip edge labels to prevent cutoff (10px margin on each side)
            let label_margin = 35.0;
            let should_show_label = x_position >= label_margin && x_position <= (canvas_width as f64 - label_margin);
            
            // Create vertical tick mark with theme-aware color
            let tick_color = theme_colors.neutral_12; // High contrast for visibility
            objects.push(
                fast2d::Rectangle::new()
                    .position(x_position as f32, timeline_y)
                    .size(1.0, 8.0) // Thin vertical line
                    .color(tick_color.0, tick_color.1, tick_color.2, tick_color.3)
                    .into()
            );
            
            // Add vertical grid line extending through all variable rows
            objects.push(
                fast2d::Rectangle::new()
                    .position(x_position as f32, 0.0)
                    .size(1.0, timeline_y) // Extends from top to timeline
                    .color(theme_colors.grid_color.0, theme_colors.grid_color.1, theme_colors.grid_color.2, theme_colors.grid_color.3)
                    .into()
            );
            
            // Add time label with actual time units and theme-aware color (only if not cut off)
            if should_show_label {
                let time_label = format_time_with_unit(time_value as f32, time_unit);
                
                // Check if this milestone would overlap with the right edge label
                let is_near_right_edge = x_position > (canvas_width as f64 - 60.0); // Increased margin to prevent overlap
                
                if !is_near_right_edge {  // Only draw if not overlapping with edge label
                    let label_color = theme_colors.neutral_12; // High contrast text
                    objects.push(
                        fast2d::Text::new()
                            .text(time_label)
                            .position(x_position as f32 - 10.0, timeline_y + 15.0)
                            .size(20.0, row_height - 15.0)
                            .color(label_color.0, label_color.1, label_color.2, label_color.3)
                            .font_size(11.0)
                            .family(fast2d::Family::name("Inter")) // Standard UI font for timeline
                            .into()
                    );
                }
            }
        }
    }
    
    // Add timeline cursor line spanning all rows
    if total_rows > 0 {
        // ✅ CRITICAL FIX: Use same timeline range as markers - NO FALLBACKS
        let (min_time, max_time) = (timeline_min, timeline_max); // Use real VCD data only
        let time_range = max_time - min_time;
        
        // Calculate cursor x position only if cursor is within visible range
        if cursor_position >= min_time && cursor_position <= max_time {
            let cursor_x = ((cursor_position - min_time) / time_range) * canvas_width as f64;
            
            // Draw vertical cursor line spanning all rows (including timeline) - now orange
            let cursor_color = (255, 165, 0, 1.0); // Orange color for cursor
            objects.push(
                fast2d::Rectangle::new()
                    .position(cursor_x as f32 - 1.0, 0.0) // Center the 3px line
                    .size(3.0, canvas_height) // 3px thick line spanning full height
                    .color(cursor_color.0, cursor_color.1, cursor_color.2, cursor_color.3)
                    .into()
            );
            
        }
        
        // Add zoom center line spanning all rows (if different from cursor)  
        // ✅ FIXED: Now using domain actor zoom center instead of legacy global state
        // Removed blue line debug spam - called frequently during rendering
        // Removed blue line checks debug spam
        if zoom_center_position >= min_time && zoom_center_position <= max_time && zoom_center_position != cursor_position {
            let zoom_center_x = ((zoom_center_position - min_time) / time_range) * canvas_width as f64;
            
            // Draw vertical zoom center line - now blue  
            let zoom_center_color = (37, 99, 235, 0.9); // Blue color for zoom center
            objects.push(
                fast2d::Rectangle::new()
                    .position(zoom_center_x as f32 - 1.0, 0.0) // Center the 3px line
                    .size(3.0, canvas_height) // 3px thick line spanning full height
                    .color(zoom_center_color.0, zoom_center_color.1, zoom_center_color.2, zoom_center_color.3)
                    .into()
            );
        }
    }
    
    // Add sticky range start and end labels to timeline edges
    if total_rows > 0 {
        let timeline_y = (total_rows - 1) as f32 * row_height;
        // ✅ CRITICAL FIX: Use same timeline range as markers - NO FALLBACKS
        let (min_time, max_time) = (timeline_min, timeline_max); // Use real VCD data only
        let label_color = theme_colors.neutral_12; // High contrast text
        
        // Determine appropriate time unit for edge labels
        let time_unit = get_time_unit_for_range(min_time, max_time);
        
        // Match tick label vertical position exactly
        let label_y = timeline_y + 15.0; // Same level as tick labels
        
        // Left edge - range start (positioned to avoid tick overlap)
        let start_label = format_time_with_unit(min_time as f32, time_unit);
        objects.push(
            fast2d::Text::new()
                .text(start_label)
                .position(5.0, label_y) // Close to left edge, avoid tick overlap
                .size(30.0, row_height - 15.0) // Match tick label size
                .color(label_color.0, label_color.1, label_color.2, label_color.3)
                .font_size(11.0)
                .family(fast2d::Family::name("Inter"))
                .into()
        );
        
        // Right edge - range end (positioned close to right edge)
        let end_label = format_time_with_unit(max_time as f32, time_unit);
        let label_width = (end_label.len() as f32 * 7.0).max(30.0); // Dynamic width
        objects.push(
            fast2d::Text::new()
                .text(end_label)
                .position(canvas_width - label_width - 5.0, label_y) // Close to right edge
                .size(label_width, row_height - 15.0) // Match tick label size  
                .color(label_color.0, label_color.1, label_color.2, label_color.3)
                .font_size(11.0)
                .family(fast2d::Family::name("Inter"))
                .into()
        );
    }
    
    // Add hover tooltip if mouse is over a variable
    if let Some(hover_info) = HOVER_INFO.lock_ref().clone() {
        let tooltip_bg_color = theme_colors.hover_panel_bg; // Bluish background
        let tooltip_text_color = theme_colors.hover_panel_text; // High contrast text
        
        // Create tooltip text with better formatting
        // Use same time unit as timeline scale for consistency
        if let Some((min_time, max_time)) = get_current_timeline_range() {
            let time_unit = get_time_unit_for_range(min_time, max_time);
            let formatted_time = format_time_with_unit(hover_info.time, time_unit);
            let tooltip_text = format!("{} = {} at {}", hover_info.variable_name, hover_info.value, formatted_time);
        
        // Position tooltip above cursor with offset
        let tooltip_x = hover_info.mouse_x + 10.0; // 10px right of cursor
        let tooltip_y = hover_info.mouse_y - 20.0; // Reduced to 20px above cursor
        
        // Clamp tooltip position to canvas bounds (larger font needs wider estimate)
        let tooltip_width = (tooltip_text.len() as f32 * 8.0).min(220.0); // Wider for 12pt font
        let tooltip_height = 14.0; // Tighter height for 12pt font
        let clamped_x = tooltip_x.max(5.0).min(canvas_width - tooltip_width - 5.0);
        let clamped_y = tooltip_y.max(5.0).min(canvas_height - tooltip_height - 5.0);
        
        // Tooltip background with minimal padding
        objects.push(
            fast2d::Rectangle::new()
                .position(clamped_x - 1.0, clamped_y - 0.5)
                .size(tooltip_width + 2.0, tooltip_height + 1.0)
                .color(tooltip_bg_color.0, tooltip_bg_color.1, tooltip_bg_color.2, 0.95)
                .into()
        );
        
        // Tooltip text with improved readability
        objects.push(
            fast2d::Text::new()
                .text(tooltip_text)
                .position(clamped_x, clamped_y)
                .size(tooltip_width, tooltip_height)
                .color(tooltip_text_color.0, tooltip_text_color.1, tooltip_text_color.2, tooltip_text_color.3)
                .font_size(12.0)
                .family(fast2d::Family::name("Fira Code")) // Monospace for better alignment
                .into()
        );
        }
    }
    
    // Debug: Waveform objects created (removed to prevent log spam)
    objects
}

// Transition jumping functions for debugging navigation

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
        if let Some(transitions) = crate::unified_timeline_service::UnifiedTimelineService::get_raw_transitions(&cache_key) {
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
    if get_current_timeline_range().is_none() {
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
    if get_current_timeline_range().is_none() {
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
    let (file_min, file_max) = get_selected_variables_file_range();
    
    // 🔧 DEBUG: Check for mixed file ranges affecting zoom
    let span = file_max - file_min;
    
    let viewport = crate::time_types::Viewport::new(
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

/// Reset zoom center to 0 seconds
pub fn reset_zoom_center() {
    // Use Actor+Relay system to reset zoom center
    crate::actors::waveform_timeline::set_zoom_center_follow_mouse(TimeNs::ZERO);
    // Also update mouse time position for consistency with zoom behavior
    MOUSE_TIME_NS.set_neq(TimeNs::ZERO);
    crate::debug_utils::debug_conditional("Zoom center reset to 0s using Actor+Relay");
}