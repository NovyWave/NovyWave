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
    current_coordinates, current_canvas_width, current_canvas_height, set_canvas_dimensions};
// MIGRATED: WaveformTimeline domain now initialized through actors/waveform_timeline.rs
use crate::actors::global_domains::waveform_timeline_domain;
use crate::time_types::{TimeNs, Viewport, NsPerPixel};
use crate::platform::{Platform, CurrentPlatform};
use crate::config::{current_theme, CONFIG_LOADED};
use shared::{SelectedVariable, UpMsg, SignalTransitionQuery, SignalTransition};
use std::rc::Rc;
use std::cell::RefCell;
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::Theme as SharedTheme;
use wasm_bindgen::JsCast;
use js_sys;
// Note: Only import colors that are actually used in the canvas rendering
use palette::{Oklch, Srgb, IntoColor};

// High-performance cursor movement constants
const PIXELS_PER_FRAME: f64 = 20.0;      // Consistent 20-pixel movement per frame
// Minimum range is now based on maximum zoom level (1 ns/pixel) 
// For 800px canvas: minimum range = 800 ns
fn get_min_valid_range_ns(canvas_width: u32) -> u64 {
    NsPerPixel::MAX_ZOOM_IN.nanos() * canvas_width as u64
}
const SAFE_FALLBACK_START: f64 = 0.0;    // Safe fallback start time
const SAFE_FALLBACK_END: f64 = 100.0;    // Safe fallback end time
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
    let cursor_time_ns = current_cursor_position();
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
    // CRITICAL FIX: Don't clear the raw backend data cache!
    // The raw backend data should persist - we only need to force reprocessing
    // For now, we'll remove the cache clearing since the reactive canvas updates
    // already handle timeline changes properly
    
    // TODO: Implement a proper processed data cache separate from raw backend data
    // Raw backend data is now stored in unified_timeline_service and should NOT be cleared
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
fn validate_startup_state() {
    let cursor_pos = current_cursor_position_seconds();
    let ns_per_pixel = current_ns_per_pixel();
    let viewport = current_viewport();
    let start = viewport.start.display_seconds();
    let end = viewport.end.display_seconds();
    
    // Check if any values are invalid
    let canvas_width = current_canvas_width() as u32;
    let min_valid_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
    if !cursor_pos.is_finite() || !start.is_finite() || !end.is_finite() || 
       ns_per_pixel.nanos() == 0 || start >= end || (end - start) < min_valid_range {
        
        crate::debug_utils::debug_timeline_validation("STARTUP: Invalid timeline state detected, applying recovery");
        
        // Simple recovery using safe defaults
        let recovery_viewport = Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(100.0));
        set_viewport_if_changed(recovery_viewport);
        set_cursor_position_if_changed(TimeNs::from_external_seconds(50.0));
        set_ns_per_pixel_if_changed(NsPerPixel::MEDIUM_ZOOM);
        
        // Note: Timeline coordinates will be automatically updated through the domain bridge
        // as the cursor, viewport, and ns_per_pixel values are set above
        
        // TODO: Replace with proper domain event relay for zoom center
        ZOOM_CENTER_NS.set_neq(TimeNs::from_external_seconds(50.0));
    } else {
        crate::debug_utils::debug_timeline_validation("STARTUP: Timeline state validation passed");
    }
}

async fn create_canvas_element() -> impl Element {
    // Validate timeline state before canvas creation
    validate_startup_state();
    
    let mut zoon_canvas = Canvas::new()
        .width(800)  // Default reasonable width to prevent division by zero
        .height(400) // Default reasonable height to prevent division by zero
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
    
    // Initialize canvas dimensions to defaults
    set_canvas_dimensions(800.0, 400.0);

    // Initialize direct cursor animation with current cursor position
    let current_cursor = current_cursor_position_seconds();
    DIRECT_CURSOR_ANIMATION.lock_mut().current_position = current_cursor;
    DIRECT_CURSOR_ANIMATION.lock_mut().target_position = current_cursor;
    let canvas_wrapper_for_signal = canvas_wrapper_shared.clone();

    // Add reactive updates when SELECTED_VARIABLES changes
    Task::start(async move {
        crate::actors::selected_variables::variables_signal_vec().for_each(move |_| {
            let canvas_wrapper_for_signal = canvas_wrapper_for_signal.clone();
            async move {
                canvas_wrapper_for_signal.borrow_mut().update_objects(|objects| {
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    // Get current theme from cache (updated by theme handler)
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
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
                // Update the theme cache for other handlers to use
                let novyui_theme = convert_theme(&theme_value);
                CURRENT_THEME_CACHE.set_neq(novyui_theme.clone());
                
                canvas_wrapper_for_theme.borrow_mut().update_objects(move |objects| {
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when zoom state changes
    let canvas_wrapper_for_zoom = canvas_wrapper_shared.clone();
    Task::start(async move {
        ns_per_pixel_signal().for_each(move |_| {
            let canvas_wrapper_for_zoom = canvas_wrapper_for_zoom.clone();
            async move {
                canvas_wrapper_for_zoom.borrow_mut().update_objects(move |objects| {
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    // Get current theme from cache (updated by theme handler)
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when cursor position changes (for new signal data)
    let canvas_wrapper_for_cursor = canvas_wrapper_shared.clone();
    Task::start(async move {
        waveform_timeline_domain().cursor_position_signal().for_each(move |_| {
            let canvas_wrapper_for_cursor = canvas_wrapper_for_cursor.clone();
            async move {
                canvas_wrapper_for_cursor.borrow_mut().update_objects(move |objects| {
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    // Get current theme from cache (updated by theme handler)
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when zoom center position changes (for zoom center line)
    let canvas_wrapper_for_zoom_center = canvas_wrapper_shared.clone();
    Task::start(async move {
        zoom_center_ns_signal().for_each(move |_| {
            let canvas_wrapper_for_zoom_center = canvas_wrapper_for_zoom_center.clone();
            async move {
                canvas_wrapper_for_zoom_center.borrow_mut().update_objects(move |objects| {
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    // Get current theme from cache (updated by theme handler)
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when signal cache changes (for new backend data)
    let canvas_wrapper_for_cache = canvas_wrapper_shared.clone();
    Task::start(async move {
        crate::unified_timeline_service::UnifiedTimelineService::cache_signal().for_each(move |_| {
            let canvas_wrapper_for_cache = canvas_wrapper_for_cache.clone();
            async move {
                canvas_wrapper_for_cache.borrow_mut().update_objects(move |objects| {
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    // Get current theme from cache (updated by theme handler)
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when hover info changes (for tooltip display)
    let canvas_wrapper_for_hover = canvas_wrapper_shared.clone();
    Task::start(async move {
        HOVER_INFO.signal_ref(|_| ()).for_each(move |_| {
            let canvas_wrapper_for_hover = canvas_wrapper_for_hover.clone();
            async move {
                canvas_wrapper_for_hover.borrow_mut().update_objects(move |objects| {
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    // Get current theme from cache (updated by theme handler)
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // High-performance direct cursor animation with smart debouncing
    start_direct_cursor_animation_loop();

    // Update timeline range when selected variables change - OPTIMIZED to prevent O(N²) requests
    Task::start(async move {
        crate::actors::selected_variables::variables_signal_vec().for_each(move |_| {
            async move {
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
                    if !CONFIG_LOADED.get() || IS_LOADING.get() {
                        request_transitions_for_new_variables_only(Some((min_time, max_time)));
                    }
                    
                    trigger_canvas_redraw();
                } else {
                    // No selected variables - use safe default range
                    let default_viewport = crate::time_types::Viewport::new(
                        TimeNs::ZERO,
                        TimeNs::from_external_seconds(100.0)
                    );
                    set_viewport_if_changed(default_viewport);
                }
            }
        }).await;
    });

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
                    let cursor_pos = current_cursor_position_seconds();
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add dedicated redraw handler that responds to force redraw signal
    let canvas_wrapper_for_force = canvas_wrapper_shared.clone();
    Task::start(async move {
        crate::actors::waveform_timeline::force_redraw_signal().for_each(move |_| {
            let canvas_wrapper_for_force = canvas_wrapper_for_force.clone();
            async move {
                canvas_wrapper_for_force.borrow_mut().update_objects(move |objects| {
                    let canvas_width = current_canvas_width();
                    let canvas_height = current_canvas_height();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = crate::actors::selected_variables::current_variables();
                    let cursor_pos = current_cursor_position_seconds();
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(
                        &selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos
                    );
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
        // Canvas is now in DOM, trigger initial render
        let rect = dom_canvas_init.get_bounding_client_rect();
        if rect.width() > 0.0 && rect.height() > 0.0 {
            set_canvas_dimensions(rect.width() as f32, rect.height() as f32);
            trigger_canvas_redraw();
        }
    });

    let canvas_wrapper_for_resize = canvas_wrapper_shared.clone();
    zoon_canvas.update_raw_el(move |raw_el| {
        raw_el.on_resize(move |width, height| {
            // Enhanced resize handler with validation
            if width > 0 && height > 0 {
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
                
                // Get canvas element's position relative to page
                let canvas_element = match event.target() {
                    Some(target) => target,
                    None => {
                        zoon::println!("⚠️ Click event target is None - ignoring canvas click");
                        return;
                    }
                };
                let canvas_rect = match canvas_element.dyn_into::<web_sys::Element>() {
                    Ok(element) => element.get_bounding_client_rect(),
                    Err(_) => {
                        zoon::println!("⚠️ Click target is not an HTML element - ignoring canvas click");
                        return;
                    }
                };
                let canvas_left = canvas_rect.left();
                
                // Calculate click position relative to canvas
                let click_x = page_click_x as f64 - canvas_left;
                
                // Legacy code for backward compatibility (will be removed when migration complete)
                // Use stored canvas width
                let canvas_width = current_canvas_width();
                let canvas_height = current_canvas_height();
                
                // Use cached coordinates for precise mouse-to-time conversion
                let mut coords = current_coordinates();
                
                // Update canvas width locally for this conversion
                coords.set_canvas_width(canvas_width as u32);
                
                // Convert mouse position to timeline time using pure integer arithmetic
                let clicked_time_ns = coords.mouse_to_time(click_x as u32);
                
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
                        *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, clamped_time_seconds);
                    });
                }
            }
        })
        .event_handler(move |event: events::PointerMove| {
            // Track mouse position using WaveformTimeline domain
            let page_mouse_x = event.x();
            let page_mouse_y = event.y();
            
            // Get canvas element's position relative to page
            let canvas_element = match event.target() {
                Some(target) => target,
                None => {
                    zoon::println!("⚠️ Mouse move event target is None - ignoring canvas mouse move");
                    return;
                }
            };
            let canvas_rect = match canvas_element.dyn_into::<web_sys::Element>() {
                Ok(element) => element.get_bounding_client_rect(),
                Err(_) => {
                    zoon::println!("⚠️ Mouse move target is not an HTML element - ignoring canvas mouse move");
                    return;
                }
            };
            let canvas_left = canvas_rect.left();
            let canvas_top = canvas_rect.top();
            
            // Calculate mouse position relative to canvas
            let mouse_x = page_mouse_x as f64 - canvas_left;
            let mouse_y = page_mouse_y as f64 - canvas_top;
            
            // Emit mouse move event to WaveformTimeline domain
            let waveform_timeline = crate::actors::waveform_timeline_domain();
            
            // Convert mouse X position to timeline time
            let coords = current_coordinates();
            let mouse_time = coords.mouse_to_time(mouse_x as u32);
            waveform_timeline.mouse_moved_relay.send((mouse_x as f32, mouse_time));
            
            // TODO: Remove when domain handles all mouse tracking
            MOUSE_X_POSITION.set_neq(mouse_x as f32);
            
            // Convert mouse X to timeline time using TimelineCoordinates
            let canvas_width = current_canvas_width();
            let canvas_height = current_canvas_height();
            
            // Use cached coordinates for precise mouse-to-time conversion
            let coords = current_coordinates();
            
            // Ensure mouse_x is within canvas bounds
            if mouse_x >= 0.0 && mouse_x <= canvas_width as f64 {
                let mouse_time_ns = coords.mouse_to_time(mouse_x as u32);
                
                // Get file bounds for validation
                if let Some((file_min, file_max)) = get_current_timeline_range() {
                    let file_start_ns = TimeNs::from_external_seconds(file_min);
                    let file_end_ns = TimeNs::from_external_seconds(file_max);
                    
                    // Clamp mouse time to file bounds
                    let clamped_mouse_time_ns = TimeNs(mouse_time_ns.nanos().clamp(file_start_ns.nanos(), file_end_ns.nanos()));
                    
                    // TODO: Replace with domain-only access
                    MOUSE_TIME_NS.set_neq(clamped_mouse_time_ns);
                    ZOOM_CENTER_NS.set_neq(clamped_mouse_time_ns);
                    
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
                        for (time, value) in time_value_pairs.iter() {
                            if (*time as f64) <= mouse_time_seconds {
                                current_value = value.clone();
                            } else {
                                break;
                            }
                        }
                        
                        // Format the value using the variable's formatter
                        let formatted_value = match current_value {
                            shared::SignalValue::Present(ref value) => {
                                var.formatter.unwrap_or_default().format(value)
                            },
                            shared::SignalValue::Missing => {
                                "N/A".to_string()
                            }
                        };
                        
                        HOVER_INFO.set_neq(Some(HoverInfo {
                            mouse_x: mouse_x as f32,
                            mouse_y: mouse_y as f32,
                            time: mouse_time_seconds as f32,
                            variable_name,
                            value: formatted_value,
                        }));
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
    // Parse unique_id: "/path/file.ext|scope|variable"
    let parts: Vec<&str> = var.unique_id.split('|').collect();
    if parts.len() < 3 {
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
        return cached_transitions.iter()
            .map(|transition| ((transition.time_ns as f64 / 1_000_000_000.0) as f32, shared::SignalValue::present(transition.value.clone())))
            .collect();
    }
    drop(processed_cache);
    
    // Not in processed cache, check raw backend data cache
    let raw_cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    if let Some(transitions) = crate::unified_timeline_service::UnifiedTimelineService::get_raw_transitions(&raw_cache_key) {
        
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
        
        return canvas_transitions;
    }
    
    // No cached data - request from backend (deduplication removed, now handled by batching)
    crate::debug_utils::debug_cache_miss(&format!("requesting from backend for {}/{}", scope_path, variable_name));
    request_signal_transitions_from_backend(file_path, scope_path, variable_name, (time_range.0 as f32, time_range.1 as f32));
    
    // Return empty data while waiting for real backend response
    // This prevents premature filler rectangles from covering actual values
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
    
    
    for var in selected_vars.iter() {
        // Parse unique_id: "file_path|scope|variable"
        if let Some(file_path) = var.unique_id.split('|').next() {
            file_paths.insert(file_path.to_string());
        }
    }
    
    file_paths
}

// ROCK-SOLID coordinate transformation system with zoom reliability
// Returns None when no variables are selected (no timeline should be shown)
pub fn get_current_timeline_range() -> Option<(f64, f64)> {
    let ns_per_pixel = current_ns_per_pixel();
    
    // If zoomed in, return the visible range with validation
    // Lower ns_per_pixel means more zoomed in
    if ns_per_pixel.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() {
        let viewport = current_viewport();
        let range_start = viewport.start.display_seconds();
        let range_end = viewport.end.display_seconds();
        
        // CRITICAL: Enforce minimum time range to prevent coordinate precision loss
        let canvas_width = current_canvas_width() as u32;
        let min_zoom_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0; // NsPerPixel-based minimum
        let current_range = range_end - range_start;
        
        // Validate range is sensible and has sufficient precision
        if range_end > range_start && range_start >= 0.0 && current_range >= min_zoom_range {
            // ENHANCED: Additional validation for finite values
            if range_start.is_finite() && range_end.is_finite() {
                return Some((range_start, range_end));
            } else {
                crate::debug_utils::debug_timeline_validation(&format!("WARNING: Timeline range not finite - start: {}, end: {}", range_start, range_end));
            }
        }
        
        // If zoom range is too narrow, expand it to minimum viable range
        if current_range > 0.0 && current_range < min_zoom_range {
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
        }
        
        // Fall through to full range if zoom range is invalid
    }
    
    // Default behavior: get range from files containing selected variables only
    let loaded_files = LOADED_FILES.lock_ref();
    
    // Get file paths that contain selected variables
    let selected_file_paths = get_selected_variable_file_paths();
    
    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;
    let mut has_valid_files = false;
    
    // If no variables are selected, don't show timeline
    if selected_file_paths.is_empty() {
        return None;
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
        // FALLBACK: No valid files with selected variables - check if any files exist at all
        let mut fallback_min: f64 = f64::MAX;
        let mut fallback_max: f64 = f64::MIN;
        let mut has_any_files = false;
        
        for file in loaded_files.iter() {
            if let (Some(file_min), Some(file_max)) = (file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)) {
                fallback_min = fallback_min.min(file_min);
                fallback_max = fallback_max.max(file_max);
                has_any_files = true;
            }
        }
        
        if has_any_files && fallback_min != fallback_max {
            // Use range from any available files as fallback
            let fallback_range = fallback_max - fallback_min;
            if fallback_range < get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0 {
                Some((fallback_min, fallback_min + get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0))
            } else {
                Some((fallback_min, fallback_max))
            }
        } else {
            // No files at all - provide safe default range
            Some((0.0, 100.0))
        }
    } else {
        // ENHANCED: Comprehensive validation before returning range
        if !min_time.is_finite() || !max_time.is_finite() {
            crate::debug_utils::debug_timeline_validation(&format!("WARNING: Timeline range calculation produced non-finite values - min: {}, max: {}", min_time, max_time));
            return Some((0.0, 100.0)); // Safe fallback
        }
        
        // Ensure minimum range for coordinate precision (but don't override valid microsecond ranges!)
        let file_range = max_time - min_time;
        if file_range < get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0 {  // Only enforce minimum for truly tiny ranges (< 1 nanosecond)
            let expanded_end = min_time + get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0;
            if expanded_end.is_finite() {
                Some((min_time, expanded_end))  // Minimum 1 nanosecond range
            } else {
                Some((0.0, get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0)) // Ultimate fallback
            }
        } else {
            Some((min_time, max_time))  // Use actual range, even if it's microseconds
        }
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
    
    // If no variables are selected, don't show timeline
    if selected_file_paths.is_empty() {
        return None;
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
        // FALLBACK: No valid files with selected variables - check if any files exist at all
        let mut fallback_min: f64 = f64::MAX;
        let mut fallback_max: f64 = f64::MIN;
        let mut has_any_files = false;
        
        for file in loaded_files.iter() {
            if let (Some(file_min), Some(file_max)) = (file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)) {
                fallback_min = fallback_min.min(file_min);
                fallback_max = fallback_max.max(file_max);
                has_any_files = true;
            }
        }
        
        if has_any_files && fallback_min != fallback_max {
            // Use range from any available files as fallback
            let fallback_range = fallback_max - fallback_min;
            if fallback_range < get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0 {
                Some((fallback_min, fallback_min + get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0))
            } else {
                Some((fallback_min, fallback_max))
            }
        } else {
            // No files at all - provide safe default range
            Some((0.0, 100.0))
        }
    } else {
        // ENHANCED: Comprehensive validation before returning range
        if !min_time.is_finite() || !max_time.is_finite() {
            crate::debug_utils::debug_timeline_validation(&format!("WARNING: Maximum timeline range calculation produced non-finite values - min: {}, max: {}", min_time, max_time));
            return Some((0.0, 100.0)); // Safe fallback
        }
        
        // Ensure minimum range for coordinate precision (but don't override valid microsecond ranges!)
        let file_range = max_time - min_time;
        if file_range < get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0 {  // Only enforce minimum for truly tiny ranges (< 1 nanosecond)
            let expanded_end = min_time + get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0;
            if expanded_end.is_finite() {
                Some((min_time, expanded_end))  // Minimum 1 nanosecond range
            } else {
                Some((0.0, get_min_valid_range_ns(current_canvas_width() as u32) as f64 / 1_000_000_000.0)) // Ultimate fallback
            }
        } else {
            Some((min_time, max_time))  // Use actual range, even if it's microseconds
        }
    }
}

// Smooth zoom functions with mouse-centered behavior
pub fn start_smooth_zoom_in() {
    // TODO: Replace with domain-only implementation
    // For now, keep legacy implementation and add domain relay
    zoom_in_started_relay().send(current_cursor_position());
    
    // Legacy zoom flag check - will be replaced when domain zoom handling is complete
    if !IS_ZOOMING_IN.get() {
        IS_ZOOMING_IN.set_neq(true);
        Task::start(async move {
            while IS_ZOOMING_IN.get() {
                let current = current_ns_per_pixel();
                // Check for Shift key for fast zoom
                let zoom_factor = if crate::state::IS_SHIFT_PRESSED.get() {
                    0.10  // Fast zoom with Shift (10% zoom in per frame)
                } else {
                    0.02  // Normal smooth zoom (2% zoom in per frame)
                };
                let new_ns_per_pixel = current.zoom_in_smooth(zoom_factor);
                if new_ns_per_pixel != current {
                    set_ns_per_pixel_if_changed(new_ns_per_pixel);
                    let zoom_center_ns = ZOOM_CENTER_NS.get();
                    let current_viewport = current_viewport();
                    let canvas_width = current_canvas_width() as u32;
                    let new_viewport = current_viewport.zoom_to(new_ns_per_pixel, zoom_center_ns, canvas_width);
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
                let current = current_ns_per_pixel();
                // Check for Shift key for fast zoom
                let zoom_factor = if crate::state::IS_SHIFT_PRESSED.get() {
                    0.10  // Fast zoom with Shift (10% zoom out per frame)
                } else {
                    0.02  // Normal smooth zoom (2% zoom out per frame)
                };
                let new_ns_per_pixel = current.zoom_out_smooth(zoom_factor);
                if new_ns_per_pixel != current {
                    set_ns_per_pixel_if_changed(new_ns_per_pixel);
                    let zoom_center_ns = ZOOM_CENTER_NS.get();
                    let current_viewport = current_viewport();
                    let canvas_width = current_canvas_width() as u32;
                    let new_viewport = current_viewport.zoom_to(new_ns_per_pixel, zoom_center_ns, canvas_width);
                    set_viewport_if_changed(new_viewport);
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
                let ns_per_pixel = current_ns_per_pixel();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                // Lower ns_per_pixel means more zoomed in
                if ns_per_pixel.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || IS_ZOOMING_IN.get() {
                    // Get current coordinates for pan computation
                    let mut coords = current_coordinates();
                    
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
                if ns_per_pixel.nanos() < NsPerPixel::MEDIUM_ZOOM.nanos() || IS_ZOOMING_IN.get() {
                    // Get current coordinates for pan computation
                    let mut coords = current_coordinates();
                    
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
        crate::debug_utils::debug_timeline_validation(&format!("Non-finite range detected - start: {}, end: {}, using fallback", start, end));
        return (SAFE_FALLBACK_START, SAFE_FALLBACK_END);
    }
    
    // Ensure proper ordering
    if start >= end {
        crate::debug_utils::debug_timeline_validation(&format!("Invalid range ordering - start: {} >= end: {}, using fallback", start, end));
        return (SAFE_FALLBACK_START, SAFE_FALLBACK_END);
    }
    
    // Enforce minimum viable range based on maximum zoom level
    let range = end - start;
    let canvas_width = current_canvas_width() as u32;
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
    let coords = current_coordinates();
    
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
    set_cursor_position_seconds(new_position);
    
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
    animation.current_position = current_cursor_position_seconds();
    IS_CURSOR_MOVING_LEFT.set_neq(true);
}

pub fn start_smooth_cursor_right() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = 1;
    animation.is_animating = true;
    animation.current_position = current_cursor_position_seconds();
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




fn get_full_file_range() -> (f64, f64) {
    let loaded_files = LOADED_FILES.lock_ref();
    
    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;
    let mut has_valid_files = false;
    
    for file in loaded_files.iter() {
        if let (Some(file_min), Some(file_max)) = (file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)) {
            // Validate file times before using them
            if file_min.is_finite() && file_max.is_finite() && file_min < file_max {
                min_time = min_time.min(file_min);
                max_time = max_time.max(file_max);
                has_valid_files = true;
            }
        }
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
        (SAFE_FALLBACK_START, SAFE_FALLBACK_END)
    };
    
    validate_and_sanitize_range(raw_range.0, raw_range.1)
}

fn get_selected_variables_file_range() -> (f64, f64) {
    use std::collections::HashSet;
    
    let selected_variables = crate::actors::selected_variables::current_variables();
    let loaded_files = LOADED_FILES.lock_ref();
    
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
    
    // Only include files that have selected variables
    for file in loaded_files.iter() {
        if selected_file_paths.contains(&file.id) {
            if let (Some(file_min), Some(file_max)) = (file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)) {
                min_time = min_time.min(file_min);
                max_time = max_time.max(file_max);
                has_valid_files = true;
            }
        }
    }
    
    if !has_valid_files || min_time == max_time {
        (0.0, 100.0)
    } else {
        (min_time, max_time)
    }
}

fn create_waveform_objects_with_theme(selected_vars: &[SelectedVariable], theme: &NovyUITheme) -> Vec<fast2d::Object2d> {
    let cursor_pos = current_cursor_position_seconds();
    let canvas_width = current_canvas_width();
    let canvas_height = current_canvas_height();
    create_waveform_objects_with_dimensions_and_theme(selected_vars, canvas_width, canvas_height, theme, cursor_pos)
}

fn create_waveform_objects_with_dimensions_and_theme(selected_vars: &[SelectedVariable], canvas_width: f32, canvas_height: f32, theme: &NovyUITheme, cursor_position: f64) -> Vec<fast2d::Object2d> {
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
                // Last rectangle: extend to view window end for proper visual coverage
                // Backend provides proper filler transitions, so this is safe
                max_time as f32
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
            
            // High-precision coordinate calculation with explicit bounds checking
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
        
        // Get consistent timeline range
        let (min_time, max_time) = get_current_timeline_range().unwrap_or_else(|| { 
            // This should never happen now, but provide safe fallback
            (0.0, 100.0)
        });
        let time_range = max_time - min_time;
        
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
        // Use consistent timeline range
        let (min_time, max_time) = get_current_timeline_range().unwrap_or((0.0, 100.0));
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
        // TODO: Replace with domain access
        let zoom_center_ns = ZOOM_CENTER_NS.get();
        let zoom_center_position = zoom_center_ns.display_seconds();
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
        let (min_time, max_time) = get_current_timeline_range().unwrap_or((0.0, 100.0));
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
        let time_unit = get_time_unit_for_range(0.0, (hover_info.time * 2.0) as f64); // Estimate unit based on time value
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
        if transition_time < current_cursor - F64_PRECISION_TOLERANCE { // f64 precision tolerance
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
        .find(|&&transition_time| transition_time > current_cursor + F64_PRECISION_TOLERANCE) // f64 precision tolerance
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
    // Reset zoom to 1x
    set_ns_per_pixel_if_changed(NsPerPixel::MEDIUM_ZOOM);
    
    // Get range for files with selected variables only
    let (file_min, file_max) = get_selected_variables_file_range();
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
    ZOOM_CENTER_NS.set_neq(TimeNs::ZERO);
    // Also update mouse time position for consistency with zoom behavior
    MOUSE_TIME_NS.set_neq(TimeNs::ZERO);
    crate::debug_utils::debug_conditional("Zoom center reset to 0s");
}