use zoon::*;
use std::collections::HashMap;
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::SelectedVariable;

// ===== MIGRATED FROM WAVEFORM_CANVAS.RS: Canvas-related global state =====

// Simplified request tracking - just a pending flag to prevent overlapping requests
// MIGRATED: Pending request tracking → use has_pending_request_signal() from waveform_timeline
pub static HAS_PENDING_REQUEST: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Store current theme for synchronous access
pub static CURRENT_THEME_CACHE: Lazy<Mutable<NovyUITheme>> = Lazy::new(|| {
    Mutable::new(NovyUITheme::Dark) // Default to dark
});

// Store hover information for tooltip display
#[derive(Clone, Debug, PartialEq)]
pub struct HoverInfo {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub time: f32,
    pub variable_name: String,
    pub value: String,
}

pub static HOVER_INFO: Lazy<Mutable<Option<HoverInfo>>> = Lazy::new(|| {
    Mutable::new(None)
});

// High-performance direct cursor animation state
#[derive(Clone, Debug)]
pub struct DirectCursorAnimation {
    pub current_position: f64,     // Current position in seconds (high precision)
    pub target_position: f64,      // Target position in seconds
    pub velocity_pixels_per_frame: f64, // Movement speed in pixels per frame
    pub is_animating: bool,        // Animation active flag
    pub direction: i8,             // -1 for left, 1 for right, 0 for stopped
    pub last_frame_time: u64,      // Last animation frame timestamp (nanoseconds)
}

impl Default for DirectCursorAnimation {
    fn default() -> Self {
        Self {
            current_position: 0.0,
            target_position: 0.0,
            velocity_pixels_per_frame: 20.0, // PIXELS_PER_FRAME constant
            is_animating: false,
            direction: 0,
            last_frame_time: 0,
        }
    }
}

pub static DIRECT_CURSOR_ANIMATION: Lazy<Mutable<DirectCursorAnimation>> = Lazy::new(|| {
    Mutable::new(DirectCursorAnimation::default())
});

// Canvas update debouncing to reduce redraw overhead
pub static PENDING_CANVAS_UPDATE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Debouncing for transition navigation to prevent rapid key press issues
pub static LAST_TRANSITION_NAVIGATION_TIME: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(0));

// PERFORMANCE FIX: Incremental Canvas Object Cache
// Caches rendered objects by variable unique_id to avoid full recreation
pub static VARIABLE_OBJECT_CACHE: Lazy<Mutable<HashMap<String, Vec<fast2d::Object2d>>>> = 
    Lazy::new(|| Mutable::new(HashMap::new()));

// Track last known variables to detect changes
pub static LAST_VARIABLES_STATE: Lazy<Mutable<Vec<SelectedVariable>>> = 
    Lazy::new(|| Mutable::new(Vec::new()));

// Track last canvas dimensions for full redraw detection
pub static LAST_CANVAS_DIMENSIONS: Lazy<Mutable<(f32, f32)>> = 
    Lazy::new(|| Mutable::new((0.0, 0.0)));