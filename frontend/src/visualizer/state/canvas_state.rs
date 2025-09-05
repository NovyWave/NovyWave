use zoon::*;
// use std::collections::HashMap; // Unused
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
// use shared::SelectedVariable; // Unused

// ===== MIGRATED FROM WAVEFORM_CANVAS.RS: Canvas-related global state =====

// Simplified request tracking - just a pending flag to prevent overlapping requests
// MIGRATED: Pending request tracking → use has_pending_request_signal() from waveform_timeline
#[allow(dead_code)] // Migration state - preserve during Actor+Relay transition
pub static HAS_PENDING_REQUEST: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Store current theme for synchronous access
#[allow(dead_code)] // Migration state - preserve during Actor+Relay transition
pub static CURRENT_THEME_CACHE: Lazy<Mutable<NovyUITheme>> = Lazy::new(|| {
    Mutable::new(NovyUITheme::Dark) // Default to dark
});

// Store hover information for tooltip display
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)] // Migration struct - preserve during Actor+Relay transition
pub struct HoverInfo {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub time: f32,
    pub variable_name: String,
    pub value: String,
}

#[allow(dead_code)] // Canvas hover state - preserve during Actor+Relay transition
pub static HOVER_INFO: Lazy<Mutable<Option<HoverInfo>>> = Lazy::new(|| {
    Mutable::new(None)
});

// High-performance direct cursor animation state
#[derive(Clone, Debug)]
pub struct DirectCursorAnimation {
    pub current_position: f64,     // Current position in seconds (high precision)
    pub target_position: f64,      // Target position in seconds
    #[allow(dead_code)] // Canvas animation field - preserve during Actor+Relay transition
    pub velocity_pixels_per_frame: f64, // Movement speed in pixels per frame
    pub is_animating: bool,        // Animation active flag
    pub direction: i8,             // -1 for left, 1 for right, 0 for stopped
    #[allow(dead_code)] // Canvas animation field - preserve during Actor+Relay transition
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
#[allow(dead_code)] // Canvas update state - preserve during Actor+Relay transition
pub static PENDING_CANVAS_UPDATE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Debouncing for transition navigation to prevent rapid key press issues
pub static LAST_TRANSITION_NAVIGATION_TIME: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(0));

// NOTE: Canvas performance caching variables removed as they were never implemented
// Future performance improvements should use proper Actor+Relay patterns