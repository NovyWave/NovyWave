use zoon::*;
use fast2d;
use crate::state::{SELECTED_VARIABLES, LOADED_FILES, TIMELINE_CURSOR_POSITION, CANVAS_WIDTH, CANVAS_HEIGHT, 
    IS_ZOOMING_IN, IS_ZOOMING_OUT, IS_PANNING_LEFT, IS_PANNING_RIGHT, IS_CURSOR_MOVING_LEFT, IS_CURSOR_MOVING_RIGHT,
    MOUSE_X_POSITION, MOUSE_TIME_POSITION, ZOOM_CENTER_POSITION, TIMELINE_ZOOM_LEVEL, TIMELINE_VISIBLE_RANGE_START, TIMELINE_VISIBLE_RANGE_END};
use crate::platform::{Platform, CurrentPlatform};
use crate::config::current_theme;
use shared::{SelectedVariable, UpMsg, SignalTransitionQuery, SignalTransition};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::Theme as SharedTheme;
use wasm_bindgen::JsCast;
use js_sys;
// Note: Only import colors that are actually used in the canvas rendering
use palette::{Oklch, Srgb, IntoColor};

// High-performance cursor movement constants
const PIXELS_PER_FRAME: f64 = 20.0;      // Consistent 20-pixel movement per frame
const MIN_VALID_RANGE: f32 = 1e-6;       // 1 microsecond minimum range
const SAFE_FALLBACK_START: f32 = 0.0;    // Safe fallback start time
const SAFE_FALLBACK_END: f32 = 100.0;    // Safe fallback end time
const MOVEMENT_FRAME_MS: u32 = 16;       // 60fps animation frame timing
const _MAX_FAILURES: u32 = 10;           // Circuit breaker threshold

// High-precision timing for smooth cursor animation (nanoseconds)
const ANIMATION_FRAME_NS: u64 = 16_666_666; // 16.666ms = 60fps in nanoseconds

// Cache for real signal transition data from backend - PUBLIC for connection.rs
pub static SIGNAL_TRANSITIONS_CACHE: Lazy<Mutable<HashMap<String, Vec<SignalTransition>>>> = Lazy::new(|| {
    Mutable::new(HashMap::new())
});

// Track which variables have had their transitions requested to prevent O(N²) request floods
// Format: "file_path|scope_path|variable_name"
static TRANSITIONS_REQUESTED: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| {
    Mutable::new(HashSet::new())
});

/// Request transitions only for variables that haven't been requested yet
/// This prevents the O(N²) request flood when adding multiple variables
fn request_transitions_for_new_variables_only(time_range: Option<(f32, f32)>) {
    let Some((min_time, max_time)) = time_range else { return; };
    
    let selected_vars = SELECTED_VARIABLES.lock_ref();
    let mut requested_set = TRANSITIONS_REQUESTED.lock_mut();
    
    for var in selected_vars.iter() {
        // Parse unique_id: "/path/file.ext|scope|variable"
        let parts: Vec<&str> = var.unique_id.split('|').collect();
        if parts.len() >= 3 {
            let file_path = parts[0];
            let scope_path = parts[1]; 
            let variable_name = parts[2];
            
            // Create cache key for tracking requests
            let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
            
            // Only request if we haven't requested this variable before
            if !requested_set.contains(&cache_key) {
                zoon::println!("Requesting transitions for NEW variable: {}", cache_key);
                request_signal_transitions_from_backend(file_path, scope_path, variable_name, (min_time, max_time));
                requested_set.insert(cache_key);
            }
        }
    }
}

/// Force request transitions for all variables (use for timeline range changes)
fn request_transitions_for_all_variables(time_range: Option<(f32, f32)>) {
    let Some((min_time, max_time)) = time_range else { return; };
    
    let selected_vars = SELECTED_VARIABLES.lock_ref();
    let mut requested_set = TRANSITIONS_REQUESTED.lock_mut();
    
    for var in selected_vars.iter() {
        // Parse unique_id: "/path/file.ext|scope|variable"
        let parts: Vec<&str> = var.unique_id.split('|').collect();
        if parts.len() >= 3 {
            let file_path = parts[0];
            let scope_path = parts[1]; 
            let variable_name = parts[2];
            
            // Create cache key for tracking requests
            let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
            
            zoon::println!("Force requesting transitions for variable: {}", cache_key);
            request_signal_transitions_from_backend(file_path, scope_path, variable_name, (min_time, max_time));
            requested_set.insert(cache_key);
        }
    }
}

/// Clear transition request tracking for removed variables
pub fn clear_transition_tracking_for_variable(unique_id: &str) {
    let parts: Vec<&str> = unique_id.split('|').collect();
    if parts.len() >= 3 {
        let cache_key = format!("{}|{}|{}", parts[0], parts[1], parts[2]);
        TRANSITIONS_REQUESTED.lock_mut().remove(&cache_key);
        zoon::println!("Cleared transition tracking for removed variable: {}", cache_key);
    }
}

/// Clear all transition request tracking (useful for complete reset)
pub fn clear_all_transition_tracking() {
    TRANSITIONS_REQUESTED.lock_mut().clear();
    zoon::println!("Cleared all transition tracking");
}

/// Batch add multiple variables without triggering O(N²) requests
/// This is useful for config restore or bulk operations
pub fn batch_request_transitions_for_variables(variables: &[shared::SelectedVariable], time_range: Option<(f32, f32)>) {
    let Some((min_time, max_time)) = time_range else { return; };
    
    let mut requested_set = TRANSITIONS_REQUESTED.lock_mut();
    
    zoon::println!("Batch requesting transitions for {} variables", variables.len());
    for var in variables.iter() {
        let parts: Vec<&str> = var.unique_id.split('|').collect();
        if parts.len() >= 3 {
            let file_path = parts[0];
            let scope_path = parts[1]; 
            let variable_name = parts[2];
            
            let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
            
            // Always request for batch operations (don't check if already requested)
            zoon::println!("Batch requesting transitions for: {}", cache_key);
            request_signal_transitions_from_backend(file_path, scope_path, variable_name, (min_time, max_time));
            requested_set.insert(cache_key);
        }
    }
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
static FORCE_REDRAW: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

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
static LAST_CANVAS_UPDATE: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(0));
static PENDING_CANVAS_UPDATE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Debouncing for transition navigation to prevent rapid key press issues
static LAST_TRANSITION_NAVIGATION_TIME: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(0));
const TRANSITION_NAVIGATION_DEBOUNCE_MS: u64 = 100; // 100ms debounce

// f32 precision tolerance for transition navigation (f32::EPSILON ≈ 1.19e-7, so 1.2e-6 gives ~10x margin)
const F32_PRECISION_TOLERANCE: f64 = 1.2e-6;


#[derive(Clone, Debug, PartialEq)]
struct HoverInfo {
    mouse_x: f32,
    mouse_y: f32,
    time: f32,
    variable_name: String,
    value: String,
}

use std::collections::HashMap;

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
fn get_time_unit_for_range(min_time: f32, max_time: f32) -> TimeUnit {
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
    // SIGNAL_TRANSITIONS_CACHE contains raw backend data and should NOT be cleared
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
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let zoom_level = TIMELINE_ZOOM_LEVEL.get();
    let start = TIMELINE_VISIBLE_RANGE_START.get();
    let end = TIMELINE_VISIBLE_RANGE_END.get();
    
    // Check if any values are invalid
    if !cursor_pos.is_finite() || !zoom_level.is_finite() || !start.is_finite() || !end.is_finite() || 
       zoom_level <= 0.0 || start >= end || (end - start) < MIN_VALID_RANGE {
        
        zoon::println!("STARTUP: Invalid timeline state detected, applying recovery");
        let (recovery_start, recovery_end) = emergency_timeline_recovery();
        TIMELINE_VISIBLE_RANGE_START.set_neq(recovery_start);
        TIMELINE_VISIBLE_RANGE_END.set_neq(recovery_end);
        TIMELINE_CURSOR_POSITION.set_neq((recovery_start + recovery_end) / 2.0);
        TIMELINE_ZOOM_LEVEL.set_neq(1.0);
        ZOOM_CENTER_POSITION.set_neq((recovery_start + recovery_end) / 2.0);
    } else {
        zoon::println!("STARTUP: Timeline state validation passed");
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
        let selected_vars = SELECTED_VARIABLES.lock_ref();
        let novyui_theme = CURRENT_THEME_CACHE.get();
        *objects = create_waveform_objects_with_theme(&selected_vars, &novyui_theme);
    });

    // Wrap canvas_wrapper in Rc<RefCell> for sharing
    let canvas_wrapper_shared = Rc::new(RefCell::new(canvas_wrapper));
    
    // Initialize canvas dimensions to defaults
    CANVAS_WIDTH.set_neq(800.0);
    CANVAS_HEIGHT.set_neq(400.0);

    // Initialize direct cursor animation with current cursor position
    let current_cursor = TIMELINE_CURSOR_POSITION.get() as f64;
    DIRECT_CURSOR_ANIMATION.lock_mut().current_position = current_cursor;
    DIRECT_CURSOR_ANIMATION.lock_mut().target_position = current_cursor;
    let canvas_wrapper_for_signal = canvas_wrapper_shared.clone();

    // Add reactive updates when SELECTED_VARIABLES changes
    Task::start(async move {
        SELECTED_VARIABLES.signal_vec_cloned().for_each(move |_| {
            let canvas_wrapper_for_signal = canvas_wrapper_for_signal.clone();
            async move {
                canvas_wrapper_for_signal.borrow_mut().update_objects(|objects| {
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
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
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when zoom state changes
    let canvas_wrapper_for_zoom = canvas_wrapper_shared.clone();
    Task::start(async move {
        crate::state::TIMELINE_ZOOM_LEVEL.signal().for_each(move |_| {
            let canvas_wrapper_for_zoom = canvas_wrapper_for_zoom.clone();
            async move {
                canvas_wrapper_for_zoom.borrow_mut().update_objects(move |objects| {
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
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
        TIMELINE_CURSOR_POSITION.signal().for_each(move |_| {
            let canvas_wrapper_for_cursor = canvas_wrapper_for_cursor.clone();
            async move {
                canvas_wrapper_for_cursor.borrow_mut().update_objects(move |objects| {
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
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
        ZOOM_CENTER_POSITION.signal().for_each(move |_| {
            let canvas_wrapper_for_zoom_center = canvas_wrapper_for_zoom_center.clone();
            async move {
                canvas_wrapper_for_zoom_center.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
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
        SIGNAL_TRANSITIONS_CACHE.signal_ref(|_| ()).for_each(move |_| {
            let canvas_wrapper_for_cache = canvas_wrapper_for_cache.clone();
            async move {
                canvas_wrapper_for_cache.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
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
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
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
        SELECTED_VARIABLES.signal_vec_cloned().for_each(move |_| {
            async move {
                
                // Calculate new range from selected variables 
                if let Some((min_time, max_time)) = get_current_timeline_range() {
                    TIMELINE_VISIBLE_RANGE_START.set_neq(min_time);
                    TIMELINE_VISIBLE_RANGE_END.set_neq(max_time);
                    
                    // OPTIMIZED: Only request transition data for NEW variables (prevents O(N²) flood)
                    request_transitions_for_new_variables_only(Some((min_time, max_time)));
                    
                    trigger_canvas_redraw();
                } else {
                    // No selected variables - use safe default range
                    TIMELINE_VISIBLE_RANGE_START.set_neq(0.0);
                    TIMELINE_VISIBLE_RANGE_END.set_neq(100.0);
                }
            }
        }).await;
    });

    // Clear cache and redraw when timeline range changes (critical for zoom operations)
    let canvas_wrapper_for_timeline_changes = canvas_wrapper_shared.clone();
    Task::start(async move {
        // Combined signal for any timeline range change
        map_ref! {
            let start = TIMELINE_VISIBLE_RANGE_START.signal(),
            let end = TIMELINE_VISIBLE_RANGE_END.signal(),
            let zoom = TIMELINE_ZOOM_LEVEL.signal()
            => (*start, *end, *zoom)
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
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    let novyui_theme = CURRENT_THEME_CACHE.get();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    // Add dedicated redraw handler that responds to force redraw signal
    let canvas_wrapper_for_force = canvas_wrapper_shared.clone();
    Task::start(async move {
        FORCE_REDRAW.signal().for_each(move |_| {
            let canvas_wrapper_for_force = canvas_wrapper_for_force.clone();
            async move {
                canvas_wrapper_for_force.borrow_mut().update_objects(move |objects| {
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    
                    // Skip render if dimensions are invalid
                    if canvas_width <= 0.0 || canvas_height <= 0.0 {
                        return;
                    }
                    
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
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
        CANVAS_WIDTH.signal().for_each(move |_| {
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
            CANVAS_WIDTH.set_neq(rect.width() as f32);
            CANVAS_HEIGHT.set_neq(rect.height() as f32);
            trigger_canvas_redraw();
        }
    });

    let canvas_wrapper_for_resize = canvas_wrapper_shared.clone();
    zoon_canvas.update_raw_el(move |raw_el| {
        raw_el.on_resize(move |width, height| {
            // Enhanced resize handler with validation
            if width > 0 && height > 0 {
                // Store canvas dimensions for click calculations
                CANVAS_WIDTH.set_neq(width as f32);
                CANVAS_HEIGHT.set_neq(height as f32);
                
                // Call Fast2D resize
                canvas_wrapper_for_resize.borrow_mut().resized(width, height);
                
                // Trigger full redraw through the dedicated handler
                trigger_canvas_redraw();
            }
        })
        .event_handler({
            let canvas_wrapper_for_click = canvas_wrapper_shared.clone();
            move |event: events::Click| {
                // Handle click to move cursor position
                let page_click_x = event.x() as f32;
                
                // Get canvas element's position relative to page
                let canvas_element = event.target().unwrap();
                let canvas_rect = canvas_element.dyn_into::<web_sys::Element>()
                    .unwrap().get_bounding_client_rect();
                let canvas_left = canvas_rect.left() as f32;
                
                // Calculate click position relative to canvas
                let click_x = page_click_x - canvas_left;
                
                // Use stored canvas width
                let canvas_width = CANVAS_WIDTH.get();
                let canvas_height = CANVAS_HEIGHT.get();
                
                // Calculate time from click position with precision-safe calculations
                if let Some((min_time, max_time)) = get_current_timeline_range() {
                    let _time_range = max_time - min_time;
                    
                    // Use f64 precision for calculation to avoid precision loss
                    let min_time_f64 = min_time as f64;
                    let max_time_f64 = max_time as f64;
                    let time_range_f64 = max_time_f64 - min_time_f64;
                    let normalized_x = (click_x / canvas_width) as f64;
                    let clicked_time_f64 = min_time_f64 + normalized_x * time_range_f64;
                    
                    // Validate before applying
                    if clicked_time_f64.is_finite() && clicked_time_f64 >= 0.0 {
                        let clicked_time_f32 = clicked_time_f64 as f32;
                        let clamped_time = clicked_time_f32.max(min_time).min(max_time);
                        
                        // Update cursor position directly (no animation for clicks)
                        TIMELINE_CURSOR_POSITION.set(clamped_time);
                        
                        // Synchronize direct animation to prevent jumps
                        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
                        animation.current_position = clicked_time_f64;
                        animation.target_position = clicked_time_f64;
                        animation.is_animating = false; // Stop any ongoing animation
                        
                        // Immediately redraw canvas with new cursor position
                        canvas_wrapper_for_click.borrow_mut().update_objects(move |objects| {
                            let selected_vars = SELECTED_VARIABLES.lock_ref();
                            let novyui_theme = CURRENT_THEME_CACHE.get();
                            *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, clamped_time);
                        });
                    }
                }
            }
        })
        .event_handler(move |event: events::PointerMove| {
            // Track mouse position for zoom center calculations
            let page_mouse_x = event.x() as f32;
            let page_mouse_y = event.y() as f32;
            
            // Get canvas element's position relative to page
            let canvas_element = event.target().unwrap();
            let canvas_rect = canvas_element.dyn_into::<web_sys::Element>()
                .unwrap().get_bounding_client_rect();
            let canvas_left = canvas_rect.left() as f32;
            let canvas_top = canvas_rect.top() as f32;
            
            // Calculate mouse position relative to canvas
            let mouse_x = page_mouse_x - canvas_left;
            let mouse_y = page_mouse_y - canvas_top;
            MOUSE_X_POSITION.set_neq(mouse_x);
            
            // Convert mouse X to timeline time and calculate hover info
            let canvas_width = CANVAS_WIDTH.get();
            let canvas_height = CANVAS_HEIGHT.get();
            if let Some((min_time, max_time)) = get_current_timeline_range() {
                let time_range = max_time - min_time;
                let mouse_time = min_time + (mouse_x / canvas_width) * time_range;
                
                // Clamp to valid range and update mouse time position
                let mouse_time = mouse_time.max(min_time).min(max_time);
                MOUSE_TIME_POSITION.set_neq(mouse_time);
                
                // Also update zoom center to mouse position for interactive zoom control
                ZOOM_CENTER_POSITION.set_neq(mouse_time);
                
                // Calculate hover info for tooltip
                let selected_vars = SELECTED_VARIABLES.lock_ref();
                let total_rows = selected_vars.len() + 1; // variables + timeline
                let row_height = if total_rows > 0 { canvas_height / total_rows as f32 } else { canvas_height };
                
                // Determine which variable row the mouse is over
                let hover_row = (mouse_y / row_height) as usize;
                
                if hover_row < selected_vars.len() {
                    // Mouse is over a variable row
                    let var = &selected_vars[hover_row];
                    let variable_name = var.unique_id.split('|').last().unwrap_or("Unknown").to_string();
                    
                    let time_value_pairs = get_signal_transitions_for_variable(var, (min_time, max_time));
                    
                    // Find the value at the current mouse time
                    let mut current_value = shared::SignalValue::present("X"); // Default unknown
                    for (time, value) in time_value_pairs.iter() {
                        if *time <= mouse_time {
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
                        mouse_x,
                        mouse_y,
                        time: mouse_time,
                        variable_name,
                        value: formatted_value,
                    }));
                } else {
                    // Mouse is over timeline or outside variable area
                    HOVER_INFO.set_neq(None);
                }
            } else {
                // No timeline range available - clear hover info
                HOVER_INFO.set_neq(None);
            }
        })
    })
}




// Get signal transitions for a variable within time range
fn get_signal_transitions_for_variable(var: &SelectedVariable, time_range: (f32, f32)) -> Vec<(f32, shared::SignalValue)> {
    // Parse unique_id: "/path/file.ext|scope|variable"
    let parts: Vec<&str> = var.unique_id.split('|').collect();
    if parts.len() < 3 {
        return vec![(time_range.0, shared::SignalValue::present("0"))];
    }
    
    let file_path = parts[0];
    let scope_path = parts[1]; 
    let variable_name = parts[2];
    
    // Create cache key for raw backend data (without timeline range since we'll process fresh each time)
    let raw_cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    
    // Check if we have real backend data cached
    let cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
    if let Some(transitions) = cache.get(&raw_cache_key) {
        
        // Convert real backend data to canvas format with proper waveform logic
        // Include ALL transitions to determine proper rectangle boundaries
        let mut canvas_transitions: Vec<(f32, shared::SignalValue)> = transitions.iter()
            .map(|t| (t.time_seconds as f32, shared::SignalValue::present(t.value.clone())))
            .collect();
            
        // Sort by time to ensure proper ordering
        canvas_transitions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // Check for time ranges beyond file boundaries and mark as missing
        let loaded_files = LOADED_FILES.lock_ref();
        if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path) {
            if let Some(max_time) = loaded_file.max_time {
                // If the timeline extends beyond the file's max time, add missing data transition
                if time_range.1 > max_time as f32 {
                    canvas_transitions.push((max_time as f32, shared::SignalValue::missing()));
                }
            }
            if let Some(min_time) = loaded_file.min_time {
                // If timeline starts before file's min time, add missing data at start
                if time_range.0 < min_time as f32 {
                    canvas_transitions.insert(0, (time_range.0, shared::SignalValue::missing()));
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
            if transition.time_seconds <= time_range.0 as f64 {
                initial_value = shared::SignalValue::present(transition.value.clone());
            } else {
                break; // Transitions should be in time order
            }
        }
        
        // Check if we need to add initial continuation rectangle
        let needs_initial_continuation = canvas_transitions.is_empty() || 
            canvas_transitions[0].0 > time_range.0;
        
        if needs_initial_continuation {
            // Insert initial value at timeline start
            canvas_transitions.insert(0, (time_range.0, initial_value.clone()));
        }
        
        // Handle empty transitions case (keep existing logic for compatibility)
        if canvas_transitions.is_empty() && !transitions.is_empty() {
            canvas_transitions.push((time_range.0, initial_value.clone()));
            canvas_transitions.push((time_range.1, initial_value));
        }
        
        // Backend now provides proper signal termination transitions - no frontend workaround needed
        
        return canvas_transitions;
    }
    drop(cache);
    
    // No cached data - request real data from backend
    zoon::println!("=== CACHE MISS - requesting from backend for {}/{} ===", scope_path, variable_name);
    request_signal_transitions_from_backend(file_path, scope_path, variable_name, time_range);
    
    // Return empty data while waiting for real backend response
    // This prevents premature filler rectangles from covering actual values
    vec![]
}

// Request real signal transitions from backend
pub fn request_signal_transitions_from_backend(file_path: &str, scope_path: &str, variable_name: &str, _time_range: (f32, f32)) {
    let _ = _time_range; // Suppress unused variable warning
    zoon::println!("=== Requesting signal transitions for {}/{} ===", scope_path, variable_name);
    
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
                loaded_file.min_time.unwrap_or(0.0) as f64,
                loaded_file.max_time.unwrap_or(1000.0) as f64  // Use higher fallback to avoid premature filler
            )
        } else {
            // Don't make request if file isn't loaded yet - prevents race condition
            zoon::println!("=== FILE NOT LOADED YET - cannot request transitions for {} ===", file_path);
            return;
        }
    };
    
    let message = UpMsg::QuerySignalTransitions {
        file_path: file_path.to_string(),
        signal_queries: vec![query],
        time_range: (file_min, file_max), // Request entire file range to ensure all transitions available
    };
    
    // Send real backend request
    // zoon::println!("=== SENDING QuerySignalTransitions for {}/{} ===", scope_path, variable_name);
    Task::start(async move {
        if let Err(e) = CurrentPlatform::send_message(message).await {
            zoon::println!("Failed to query signal transitions via platform: {}", e);
        } else {
            // zoon::println!("=== QuerySignalTransitions sent successfully ===");
        }
    });
}

// Trigger canvas redraw when new signal data arrives
pub fn trigger_canvas_redraw() {
    // Increment counter to trigger the dedicated redraw signal
    FORCE_REDRAW.update(|counter| counter + 1);
}

// Extract unique file paths from selected variables
fn get_selected_variable_file_paths() -> std::collections::HashSet<String> {
    let selected_vars = SELECTED_VARIABLES.lock_ref();
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
pub fn get_current_timeline_range() -> Option<(f32, f32)> {
    let zoom_level = crate::state::TIMELINE_ZOOM_LEVEL.get();
    
    // If zoomed in, return the visible range with validation
    if zoom_level > 1.0 {
        let range_start = crate::state::TIMELINE_VISIBLE_RANGE_START.get();
        let range_end = crate::state::TIMELINE_VISIBLE_RANGE_END.get();
        
        // CRITICAL: Enforce minimum time range to prevent coordinate precision loss
        let min_zoom_range = 1e-9; // Minimum 1 nanosecond range prevents division by near-zero
        let current_range = range_end - range_start;
        
        // Validate range is sensible and has sufficient precision
        if range_end > range_start && range_start >= 0.0 && current_range >= min_zoom_range {
            // ENHANCED: Additional validation for finite values
            if range_start.is_finite() && range_end.is_finite() {
                return Some((range_start, range_end));
            } else {
                zoon::println!("WARNING: Timeline range not finite - start: {}, end: {}", range_start, range_end);
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
                zoon::println!("Expanded narrow range from {:.12} to [{:.12}, {:.12}]", current_range, expanded_start, expanded_end);
                return Some((expanded_start, expanded_end));
            } else {
                zoon::println!("WARNING: Failed to expand range - center: {}, half_range: {}", range_center, half_min_range);
            }
        }
        
        // Fall through to full range if zoom range is invalid
    }
    
    // Default behavior: get range from files containing selected variables only
    let loaded_files = LOADED_FILES.lock_ref();
    
    // Get file paths that contain selected variables
    let selected_file_paths = get_selected_variable_file_paths();
    
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
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
                if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
                    min_time = min_time.min(file_min as f32);
                    max_time = max_time.max(file_max as f32);
                    has_valid_files = true;
                }
            }
        }
    }
    
    if !has_valid_files || min_time == max_time {
        // FALLBACK: No valid files with selected variables - check if any files exist at all
        let mut fallback_min: f32 = f32::MAX;
        let mut fallback_max: f32 = f32::MIN;
        let mut has_any_files = false;
        
        for file in loaded_files.iter() {
            if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
                fallback_min = fallback_min.min(file_min as f32);
                fallback_max = fallback_max.max(file_max as f32);
                has_any_files = true;
            }
        }
        
        if has_any_files && fallback_min != fallback_max {
            // Use range from any available files as fallback
            let fallback_range = fallback_max - fallback_min;
            if fallback_range < 1e-9 {
                Some((fallback_min, fallback_min + 1e-9))
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
            zoon::println!("WARNING: Timeline range calculation produced non-finite values - min: {}, max: {}", min_time, max_time);
            return Some((0.0, 100.0)); // Safe fallback
        }
        
        // Ensure minimum range for coordinate precision (but don't override valid microsecond ranges!)
        let file_range = max_time - min_time;
        if file_range < 1e-9 {  // Only enforce minimum for truly tiny ranges (< 1 nanosecond)
            let expanded_end = min_time + 1e-9;
            if expanded_end.is_finite() {
                Some((min_time, expanded_end))  // Minimum 1 nanosecond range
            } else {
                Some((0.0, 1e-9)) // Ultimate fallback
            }
        } else {
            Some((min_time, max_time))  // Use actual range, even if it's microseconds
        }
    }
}

// Smooth zoom functions with mouse-centered behavior
pub fn start_smooth_zoom_in() {
    if !IS_ZOOMING_IN.get() {
        IS_ZOOMING_IN.set_neq(true);
        Task::start(async move {
            while IS_ZOOMING_IN.get() {
                let current = TIMELINE_ZOOM_LEVEL.get();
                // Check for Shift key for fast zoom
                let zoom_multiplier = if crate::state::IS_SHIFT_PRESSED.get() {
                    1.10  // Fast zoom with Shift (10% per frame)
                } else {
                    1.02  // Normal smooth zoom (2% per frame)
                };
                let new_zoom = (current * zoom_multiplier).min(1000000000.0);
                if new_zoom != current {
                    update_zoom_with_mouse_center(new_zoom);
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
                let current = TIMELINE_ZOOM_LEVEL.get();
                // Check for Shift key for fast zoom
                let zoom_multiplier = if crate::state::IS_SHIFT_PRESSED.get() {
                    1.10  // Fast zoom with Shift (10% per frame)
                } else {
                    1.02  // Normal smooth zoom (2% per frame)
                };
                let new_zoom = (current / zoom_multiplier).max(1.0);
                if new_zoom != current {
                    update_zoom_with_mouse_center(new_zoom);
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
                let zoom_level = TIMELINE_ZOOM_LEVEL.get();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                if zoom_level > 1.0 || IS_ZOOMING_IN.get() {
                    let current_start = TIMELINE_VISIBLE_RANGE_START.get();
                    let current_end = TIMELINE_VISIBLE_RANGE_END.get();
                    let visible_range = current_end - current_start;
                    
                    // Check for Shift key for turbo panning
                    let pan_multiplier = if crate::state::IS_SHIFT_PRESSED.get() {
                        0.10  // Turbo pan with Shift (10% per frame)
                    } else {
                        0.02  // Normal smooth pan (2% per frame)
                    };
                    let pan_distance = visible_range * pan_multiplier;
                    
                    // Get file bounds for clamping
                    let (file_min, _file_max) = get_full_file_range();
                    
                    // Calculate new positions
                    let new_start = (current_start - pan_distance).max(file_min);
                    let new_end = new_start + visible_range;
                    
                    // Update if changed (pan left succeeded)
                    if new_start != current_start {
                        TIMELINE_VISIBLE_RANGE_START.set_neq(new_start);
                        TIMELINE_VISIBLE_RANGE_END.set_neq(new_end);
                    } else {
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
                let zoom_level = TIMELINE_ZOOM_LEVEL.get();
                // Allow panning when zoomed in OR when actively zooming in for simultaneous operation
                if zoom_level > 1.0 || IS_ZOOMING_IN.get() {
                    let current_start = TIMELINE_VISIBLE_RANGE_START.get();
                    let current_end = TIMELINE_VISIBLE_RANGE_END.get();
                    let visible_range = current_end - current_start;
                    
                    // Check for Shift key for turbo panning
                    let pan_multiplier = if crate::state::IS_SHIFT_PRESSED.get() {
                        0.10  // Turbo pan with Shift (10% per frame)
                    } else {
                        0.02  // Normal smooth pan (2% per frame)
                    };
                    let pan_distance = visible_range * pan_multiplier;
                    
                    // Get file bounds for clamping
                    let (_file_min, file_max) = get_full_file_range();
                    
                    // Calculate new positions
                    let new_end = (current_end + pan_distance).min(file_max);
                    let new_start = new_end - visible_range;
                    
                    // Update if changed (pan right succeeded)
                    if new_end != current_end {
                        TIMELINE_VISIBLE_RANGE_START.set_neq(new_start);
                        TIMELINE_VISIBLE_RANGE_END.set_neq(new_end);
                    } else {
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
fn validate_and_sanitize_range(start: f32, end: f32) -> (f32, f32) {
    // Check for NaN/Infinity in inputs
    if !start.is_finite() || !end.is_finite() {
        zoon::println!("TIMELINE DEBUG: Non-finite range detected - start: {}, end: {}, using fallback", start, end);
        return (SAFE_FALLBACK_START, SAFE_FALLBACK_END);
    }
    
    // Ensure proper ordering
    if start >= end {
        zoon::println!("TIMELINE DEBUG: Invalid range ordering - start: {} >= end: {}, using fallback", start, end);
        return (SAFE_FALLBACK_START, SAFE_FALLBACK_END);
    }
    
    // Enforce minimum viable range to prevent precision issues
    let range = end - start;
    if range < MIN_VALID_RANGE {
        zoon::println!("TIMELINE DEBUG: Range too small: {:.3e}s, enforcing minimum", range);
        let center = (start + end) / 2.0;
        let half_range = MIN_VALID_RANGE / 2.0;
        return (center - half_range, center + half_range);
    }
    
    // Range is valid
    (start, end)
}

/// Emergency recovery system for corrupted timeline state
fn emergency_timeline_recovery() -> (f32, f32) {
    zoon::println!("EMERGENCY: Timeline state corrupted, attempting recovery");
    
    // Reset to safe defaults first
    TIMELINE_ZOOM_LEVEL.set_neq(1.0);
    ZOOM_CENTER_POSITION.set_neq(50.0);
    
    // Try to get actual file range
    if let Some((file_min, file_max)) = get_current_timeline_range() {
        let (validated_min, validated_max) = validate_and_sanitize_range(file_min, file_max);
        zoon::println!("EMERGENCY: Recovered using file range: {} to {}", validated_min, validated_max);
        return (validated_min, validated_max);
    }
    
    // Ultimate fallback
    zoon::println!("EMERGENCY: Using ultimate fallback range");
    (SAFE_FALLBACK_START, SAFE_FALLBACK_END)
}

/// Time-based fallback movement when pixel conversion fails repeatedly
fn apply_fallback_movement(direction: f64, current_time: f64) -> Option<f64> {
    // Use a simple time-based step that works regardless of zoom level
    let time_step = 1e-6 * direction; // 1 microsecond step
    let new_time = current_time + time_step;
    
    if new_time >= 0.0 && new_time.is_finite() {
        Some(new_time)
    } else {
        None
    }
}

/// Safe pixel-to-time conversion with emergency recovery
fn convert_pixel_to_time_safe(pixel_offset: f64, current_time: f64) -> Option<f64> {
    // First, validate and potentially recover timeline ranges
    let visible_start_raw = TIMELINE_VISIBLE_RANGE_START.get();
    let visible_end_raw = TIMELINE_VISIBLE_RANGE_END.get();
    
    // Detect corrupted timeline state and recover
    let (visible_start, visible_end) = if !visible_start_raw.is_finite() || !visible_end_raw.is_finite() || visible_start_raw >= visible_end_raw {
        let (recovered_start, recovered_end) = emergency_timeline_recovery();
        TIMELINE_VISIBLE_RANGE_START.set_neq(recovered_start);
        TIMELINE_VISIBLE_RANGE_END.set_neq(recovered_end);
        (recovered_start as f64, recovered_end as f64)
    } else {
        (visible_start_raw as f64, visible_end_raw as f64)
    };
    
    let visible_range = visible_end - visible_start;
    let canvas_width = CANVAS_WIDTH.get() as f64;
    
    // Validate remaining inputs
    if !current_time.is_finite() {
        zoon::println!("CURSOR DEBUG: Non-finite current time: {}", current_time);
        return None;
    }
    
    if canvas_width <= 0.0 {
        zoon::println!("CURSOR DEBUG: Invalid canvas width: {}", canvas_width);
        return None;
    }
    
    // Perform conversion with validated inputs
    let current_pixel = ((current_time - visible_start) / visible_range) * canvas_width;
    let new_pixel = current_pixel + pixel_offset;
    let new_time = visible_start + (new_pixel / canvas_width) * visible_range;
    
    // Final validation
    if new_time.is_finite() {
        Some(new_time)
    } else {
        zoon::println!("CURSOR DEBUG: Calculation produced non-finite result");
        None
    }
}

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
            
            // Calculate new position based on direction and velocity
            let pixel_delta = animation.direction as f64 * animation.velocity_pixels_per_frame;
            
            if let Some(new_time) = convert_pixel_to_time_safe(pixel_delta, animation.current_position) {
                let safe_time = new_time.max(0.0);
                
                // Update animation state
                animation.current_position = safe_time;
                drop(animation);
                
                // Update timeline cursor with debouncing
                update_timeline_cursor_with_debouncing(safe_time as f32);
            } else {
                // Fallback to percentage-based movement if pixel conversion fails
                if let Some(new_time) = apply_fallback_movement(animation.direction as f64, animation.current_position) {
                    animation.current_position = new_time;
                    drop(animation);
                    update_timeline_cursor_with_debouncing(new_time as f32);
                } else {
                    // Stop animation if both methods fail
                    animation.is_animating = false;
                    drop(animation);
                }
            }
        }
    });
}

// Smart cursor update with debouncing to reduce canvas redraw overhead
fn update_timeline_cursor_with_debouncing(new_position: f32) {
    TIMELINE_CURSOR_POSITION.set_neq(new_position);
    
    // Debounce canvas updates - only redraw every 16ms maximum
    let now = get_current_time_ns();
    let last_update = LAST_CANVAS_UPDATE.get();
    
    if now - last_update >= ANIMATION_FRAME_NS {
        LAST_CANVAS_UPDATE.set_neq(now);
        trigger_canvas_redraw();
    } else if !PENDING_CANVAS_UPDATE.get() {
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
    animation.current_position = TIMELINE_CURSOR_POSITION.get() as f64;
    IS_CURSOR_MOVING_LEFT.set_neq(true);
}

pub fn start_smooth_cursor_right() {
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.direction = 1;
    animation.is_animating = true;
    animation.current_position = TIMELINE_CURSOR_POSITION.get() as f64;
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

// Legacy zoom functions for button compatibility
pub fn zoom_in() {
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    let new_zoom = (current_zoom * 1.5).min(1000000000.0);
    if new_zoom != current_zoom {
        update_zoom_with_mouse_center(new_zoom);
    }
}

pub fn zoom_out() {
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    let new_zoom = (current_zoom / 1.5).max(1.0);
    if new_zoom != current_zoom {
        update_zoom_with_mouse_center(new_zoom);
    }
}


// Bulletproof mouse-centered zoom function with comprehensive validation
fn update_zoom_with_mouse_center(new_zoom: f32) {
    // Validate input zoom level
    if !new_zoom.is_finite() || new_zoom <= 0.0 {
        zoon::println!("ZOOM DEBUG: Invalid zoom level {}, aborting", new_zoom);
        return;
    }
    
    let zoom_center_time = ZOOM_CENTER_POSITION.get();
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    
    // Validate current state
    if !zoom_center_time.is_finite() || !current_zoom.is_finite() || current_zoom <= 0.0 {
        zoon::println!("ZOOM DEBUG: Invalid current state - center: {}, zoom: {}, resetting", zoom_center_time, current_zoom);
        let (recovery_start, recovery_end) = emergency_timeline_recovery();
        TIMELINE_VISIBLE_RANGE_START.set_neq(recovery_start);
        TIMELINE_VISIBLE_RANGE_END.set_neq(recovery_end);
        TIMELINE_ZOOM_LEVEL.set_neq(1.0);
        ZOOM_CENTER_POSITION.set_neq((recovery_start + recovery_end) / 2.0);
        return;
    }
    
    // Set the new zoom level
    TIMELINE_ZOOM_LEVEL.set_neq(new_zoom);
    
    if new_zoom <= 1.0 {
        // Full zoom - use range from selected variables
        let Some((file_min, file_max)) = get_current_timeline_range() else { 
            let (recovery_start, recovery_end) = emergency_timeline_recovery();
            TIMELINE_VISIBLE_RANGE_START.set_neq(recovery_start);
            TIMELINE_VISIBLE_RANGE_END.set_neq(recovery_end);
            return; 
        };
        
        // Apply validation before setting
        let (validated_min, validated_max) = validate_and_sanitize_range(file_min, file_max);
        TIMELINE_VISIBLE_RANGE_START.set_neq(validated_min);
        TIMELINE_VISIBLE_RANGE_END.set_neq(validated_max);
        
    } else {
        // Zoomed in - calculate visible range centered on mouse position
        let Some((current_start, current_end)) = get_current_timeline_range() else { 
            let (recovery_start, recovery_end) = emergency_timeline_recovery();
            TIMELINE_VISIBLE_RANGE_START.set_neq(recovery_start);
            TIMELINE_VISIBLE_RANGE_END.set_neq(recovery_end);
            return; 
        };
        
        // Validate current range
        let (validated_start, validated_end) = validate_and_sanitize_range(current_start, current_end);
        let current_range = validated_end - validated_start;
        
        // Calculate zoom ratio with division-by-zero protection
        let zoom_ratio = if new_zoom == 0.0 {
            zoon::println!("ZOOM DEBUG: Zero new_zoom detected, aborting");
            return;
        } else {
            current_zoom / new_zoom
        };
        
        if !zoom_ratio.is_finite() {
            zoon::println!("ZOOM DEBUG: Invalid zoom ratio {}/{}, aborting", current_zoom, new_zoom);
            return;
        }
        
        // Calculate new range with overflow protection
        let new_range = current_range * zoom_ratio;
        if !new_range.is_finite() || new_range <= 0.0 || new_range < MIN_VALID_RANGE {
            zoon::println!("ZOOM DEBUG: Invalid new range: {:.3e}, aborting", new_range);
            return;
        }
        
        // Calculate zoom center ratio with validated range
        let zoom_center_ratio = if current_range > 0.0 && current_range.is_finite() {
            let ratio = (zoom_center_time - validated_start) / current_range;
            if ratio.is_finite() { ratio.max(0.0).min(1.0) } else { 0.5 }
        } else {
            0.5 // Default to center
        };
        
        let new_start = zoom_center_time - (new_range * zoom_center_ratio);
        let new_end = new_start + new_range;
        
        // Clamp to file bounds with validation
        let (file_min, file_max) = get_full_file_range();
        let (validated_file_min, validated_file_max) = validate_and_sanitize_range(file_min, file_max);
        
        let mut clamped_start = new_start.max(validated_file_min);
        let mut clamped_end = new_end.min(validated_file_max);
        
        // Ensure minimum range if we hit bounds
        if clamped_end - clamped_start < new_range {
            if new_start < validated_file_min {
                clamped_end = (clamped_start + new_range).min(validated_file_max);
            } else if new_end > validated_file_max {
                clamped_start = (clamped_end - new_range).max(validated_file_min);
            }
        }
        
        // Final validation and set
        let (final_start, final_end) = validate_and_sanitize_range(clamped_start, clamped_end);
        TIMELINE_VISIBLE_RANGE_START.set_neq(final_start);
        TIMELINE_VISIBLE_RANGE_END.set_neq(final_end);
    }
}

fn get_full_file_range() -> (f32, f32) {
    let loaded_files = LOADED_FILES.lock_ref();
    
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
    let mut has_valid_files = false;
    
    for file in loaded_files.iter() {
        if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
            let file_min_f32 = file_min as f32;
            let file_max_f32 = file_max as f32;
            
            // Validate file times before using them
            if file_min_f32.is_finite() && file_max_f32.is_finite() && file_min_f32 < file_max_f32 {
                min_time = min_time.min(file_min_f32);
                max_time = max_time.max(file_max_f32);
                has_valid_files = true;
            }
        }
    }
    
    // Use validation system for final result
    let raw_range = if has_valid_files && min_time < max_time {
        (min_time, max_time)
    } else {
        (SAFE_FALLBACK_START, SAFE_FALLBACK_END)
    };
    
    validate_and_sanitize_range(raw_range.0, raw_range.1)
}

fn get_selected_variables_file_range() -> (f32, f32) {
    use std::collections::HashSet;
    
    let selected_variables = crate::state::SELECTED_VARIABLES.lock_ref();
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
    
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
    let mut has_valid_files = false;
    
    // Only include files that have selected variables
    for file in loaded_files.iter() {
        if selected_file_paths.contains(&file.id) {
            if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
                min_time = min_time.min(file_min as f32);
                max_time = max_time.max(file_max as f32);
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
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let canvas_width = CANVAS_WIDTH.get();
    let canvas_height = CANVAS_HEIGHT.get();
    create_waveform_objects_with_dimensions_and_theme(selected_vars, canvas_width, canvas_height, theme, cursor_pos)
}

fn create_waveform_objects_with_dimensions_and_theme(selected_vars: &[SelectedVariable], canvas_width: f32, canvas_height: f32, theme: &NovyUITheme, cursor_position: f32) -> Vec<fast2d::Object2d> {
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
                max_time
            };
            
            // Skip rectangles completely outside visible range
            if end_time <= min_time || *start_time >= max_time {
                continue;
            }
            
            // Clip rectangle to visible time range
            let visible_start_time = start_time.max(min_time);
            let visible_end_time = end_time.min(max_time);
            
            // GRAPHICS-LEVEL FIX: Robust coordinate transformation with precision handling
            let time_range = max_time - min_time;
            
            // Prevent division by zero and handle degenerate cases
            if time_range <= 0.0 || canvas_width <= 0.0 {
                continue; // Skip rendering in degenerate cases
            }
            
            // High-precision coordinate calculation with explicit bounds checking
            let time_to_pixel_ratio = canvas_width / time_range;
            let rect_start_x = (visible_start_time - min_time) * time_to_pixel_ratio;
            let rect_end_x = (visible_end_time - min_time) * time_to_pixel_ratio;
            
            // CRITICAL: Enforce minimum visible width to prevent zero rectangles
            let raw_rect_width = rect_end_x - rect_start_x;
            
            // Sub-pixel transition detection - switch to vertical line indicators
            if raw_rect_width < 2.0 {
                // Skip off-screen transitions for performance
                if rect_start_x < -10.0 || rect_start_x > canvas_width + 10.0 {
                    continue;
                }
                
                // Draw thin vertical line at transition point for sub-pixel transitions
                let line_x = rect_start_x.max(0.0).min(canvas_width - 1.0);
                
                // Get theme colors (we need to access it here since theme_colors is defined later)
                let transition_theme_colors = get_current_theme_colors(theme);
                let accent_color = transition_theme_colors.cursor_color; // Use cursor color for bright visibility
                
                objects.push(
                    fast2d::Rectangle::new()
                        .position(line_x, y_position)
                        .size(1.0, row_height) // 1 pixel wide vertical line
                        .color(accent_color.0, accent_color.1, accent_color.2, accent_color.3)
                        .into()
                );
                
                continue; // Skip normal rectangle rendering for sub-pixel transitions
            }
            
            let min_visible_width = 1.0; // Minimum 1 pixel width for visibility
            let rect_width = raw_rect_width.max(min_visible_width);
            
            // Bounds checking: ensure rectangle fits within canvas
            let rect_start_x = rect_start_x.max(0.0).min(canvas_width - rect_width);
            let rect_end_x = rect_start_x + rect_width;
            
            // Validate final dimensions before creating Fast2D objects
            if rect_width <= 0.0 || rect_start_x >= canvas_width || rect_end_x <= 0.0 {
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
                    .position(rect_start_x, y_position + 2.0)
                    .size(rect_width, row_height - 4.0)
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
                        .position(rect_start_x + text_padding, y_position + row_height / 3.0)
                        .size(text_width, text_height)
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
        let raw_time_step = time_range / (tick_count - 1) as f32;
        let time_step = round_to_nice_number(raw_time_step);
        
        // Find the first tick that's >= min_time (aligned to step boundaries)
        let first_tick = (min_time / time_step).ceil() * time_step;
        let last_tick = max_time;
        let actual_tick_count = ((last_tick - first_tick) / time_step).ceil() as i32 + 1;
        
        for tick_index in 0..actual_tick_count {
            let time_value = first_tick + (tick_index as f32 * time_step);
            let time_value = time_value.min(max_time);
            let x_position = ((time_value - min_time) / time_range) * canvas_width;
            
            // Skip edge labels to prevent cutoff (10px margin on each side)
            let label_margin = 35.0;
            let should_show_label = x_position >= label_margin && x_position <= (canvas_width - label_margin);
            
            // Create vertical tick mark with theme-aware color
            let tick_color = theme_colors.neutral_12; // High contrast for visibility
            objects.push(
                fast2d::Rectangle::new()
                    .position(x_position, timeline_y)
                    .size(1.0, 8.0) // Thin vertical line
                    .color(tick_color.0, tick_color.1, tick_color.2, tick_color.3)
                    .into()
            );
            
            // Add vertical grid line extending through all variable rows
            objects.push(
                fast2d::Rectangle::new()
                    .position(x_position, 0.0)
                    .size(1.0, timeline_y) // Extends from top to timeline
                    .color(theme_colors.grid_color.0, theme_colors.grid_color.1, theme_colors.grid_color.2, theme_colors.grid_color.3)
                    .into()
            );
            
            // Add time label with actual time units and theme-aware color (only if not cut off)
            if should_show_label {
                let time_label = format_time_with_unit(time_value, time_unit);
                
                // Check if this milestone would overlap with the right edge label
                let is_near_right_edge = x_position > (canvas_width - 60.0); // Increased margin to prevent overlap
                
                if !is_near_right_edge {  // Only draw if not overlapping with edge label
                    let label_color = theme_colors.neutral_12; // High contrast text
                    objects.push(
                        fast2d::Text::new()
                            .text(time_label)
                            .position(x_position - 10.0, timeline_y + 15.0)
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
            let cursor_x = ((cursor_position - min_time) / time_range) * canvas_width;
            
            // Draw vertical cursor line spanning all rows (including timeline) - now orange
            let cursor_color = (255, 165, 0, 1.0); // Orange color for cursor
            objects.push(
                fast2d::Rectangle::new()
                    .position(cursor_x - 1.0, 0.0) // Center the 3px line
                    .size(3.0, canvas_height) // 3px thick line spanning full height
                    .color(cursor_color.0, cursor_color.1, cursor_color.2, cursor_color.3)
                    .into()
            );
            
        }
        
        // Add zoom center line spanning all rows (if different from cursor)
        let zoom_center_position = ZOOM_CENTER_POSITION.get();
        if zoom_center_position >= min_time && zoom_center_position <= max_time && zoom_center_position != cursor_position {
            let zoom_center_x = ((zoom_center_position - min_time) / time_range) * canvas_width;
            
            // Draw vertical zoom center line - now blue  
            let zoom_center_color = (37, 99, 235, 0.9); // Blue color for zoom center
            objects.push(
                fast2d::Rectangle::new()
                    .position(zoom_center_x - 1.0, 0.0) // Center the 3px line
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
        let start_label = format_time_with_unit(min_time, time_unit);
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
        let end_label = format_time_with_unit(max_time, time_unit);
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
        let time_unit = get_time_unit_for_range(0.0, hover_info.time * 2.0); // Estimate unit based on time value
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
    let selected_vars = SELECTED_VARIABLES.lock_ref();
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
        let cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
        if let Some(transitions) = cache.get(&cache_key) {
            // Extract time points and convert to f64
            for transition in transitions {
                // Only include transitions within reasonable bounds
                if transition.time_seconds >= 0.0 {
                    all_transitions.push(transition.time_seconds);
                }
            }
        }
    }
    
    // Remove duplicates and sort by time
    all_transitions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Use f32-appropriate tolerance instead of f64 precision
    // f32::EPSILON ≈ 1.19e-7, so F32_PRECISION_TOLERANCE gives us ~10x margin
    all_transitions.dedup_by(|a, b| (*a - *b).abs() < F32_PRECISION_TOLERANCE); // Remove near-duplicate times with f32 precision
    
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
    
    let current_cursor = TIMELINE_CURSOR_POSITION.get() as f64;
    let transitions = collect_all_transitions();
    
    if transitions.is_empty() {
        return; // No transitions available
    }
    
    // Find the largest transition time that's less than current cursor
    let mut previous_transition: Option<f64> = None;
    
    for &transition_time in transitions.iter() {
        if transition_time < current_cursor - F32_PRECISION_TOLERANCE { // f32-appropriate tolerance to avoid precision issues
            previous_transition = Some(transition_time);
        } else {
            break; // Transitions are sorted, so we can stop here
        }
    }
    
    if let Some(prev_time) = previous_transition {
        // Jump to previous transition
        TIMELINE_CURSOR_POSITION.set_neq(prev_time as f32);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = prev_time;
        animation.target_position = prev_time;
        zoon::println!("Jumped to previous transition at {:.9}s", prev_time);
    } else if !transitions.is_empty() {
        // If no previous transition, wrap to the last transition
        let last_transition = transitions[transitions.len() - 1];
        TIMELINE_CURSOR_POSITION.set_neq(last_transition as f32);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = last_transition;
        animation.target_position = last_transition;
        zoon::println!("Wrapped to last transition at {:.9}s", last_transition);
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
    
    let current_cursor = TIMELINE_CURSOR_POSITION.get() as f64;
    let transitions = collect_all_transitions();
    
    if transitions.is_empty() {
        return; // No transitions available
    }
    
    // Find the smallest transition time that's greater than current cursor
    let next_transition = transitions.iter()
        .find(|&&transition_time| transition_time > current_cursor + F32_PRECISION_TOLERANCE) // f32-appropriate tolerance to avoid precision issues
        .copied();
    
    if let Some(next_time) = next_transition {
        // Jump to next transition
        TIMELINE_CURSOR_POSITION.set_neq(next_time as f32);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = next_time;
        animation.target_position = next_time;
        zoon::println!("Jumped to next transition at {:.9}s", next_time);
    } else if !transitions.is_empty() {
        // If no next transition, wrap to the first transition
        let first_transition = transitions[0];
        TIMELINE_CURSOR_POSITION.set_neq(first_transition as f32);
        // Synchronize direct animation to prevent jumps when using Q/E after transition jump
        let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
        animation.current_position = first_transition;
        animation.target_position = first_transition;
        zoon::println!("Wrapped to first transition at {:.9}s", first_transition);
    }
}

/// Reset zoom to fit all data in view (recovery function for broken zoom states)
pub fn reset_zoom_to_fit_all() {
    // Reset zoom to 1x
    TIMELINE_ZOOM_LEVEL.set_neq(1.0);
    
    // Get range for files with selected variables only
    let (file_min, file_max) = get_selected_variables_file_range();
    TIMELINE_VISIBLE_RANGE_START.set_neq(file_min);
    TIMELINE_VISIBLE_RANGE_END.set_neq(file_max);
    
    // Reset cursor to a reasonable position
    let middle_time = (file_min + file_max) / 2.0;
    TIMELINE_CURSOR_POSITION.set_neq(middle_time);
    
    // Synchronize direct animation to prevent jumps when using Q/E after zoom reset
    let mut animation = DIRECT_CURSOR_ANIMATION.lock_mut();
    animation.current_position = middle_time as f64;
    animation.target_position = middle_time as f64;
    
    // Debug logging to verify correct range calculation
    let selected_variables = crate::state::SELECTED_VARIABLES.lock_ref();
    zoon::println!("=== ZOOM RESET DEBUG ===");
    zoon::println!("Selected variables count: {}", selected_variables.len());
    for var in selected_variables.iter() {
        zoon::println!("  Variable: {}", var.unique_id);
    }
    zoon::println!("Reset range: {:.9}s to {:.9}s (span: {:.9}s)", file_min, file_max, file_max - file_min);
    zoon::println!("Cursor positioned at: {:.9}s", middle_time);
}

/// Reset zoom center to 0 seconds
pub fn reset_zoom_center() {
    ZOOM_CENTER_POSITION.set_neq(0.0);
    // Also update mouse time position for consistency with zoom behavior
    MOUSE_TIME_POSITION.set_neq(0.0);
    zoon::println!("Zoom center reset to 0s");
}