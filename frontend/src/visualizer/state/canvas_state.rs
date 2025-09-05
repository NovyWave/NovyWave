use zoon::*;
// use std::collections::HashMap; // Unused
// Removed unused import: moonzoon_novyui::tokens::theme::Theme as NovyUITheme
// use shared::SelectedVariable; // Unused

// ===== MIGRATED FROM WAVEFORM_CANVAS.RS: Canvas-related global state =====

// ✅ REMOVED: HAS_PENDING_REQUEST migrated to waveform_timeline domain has_pending_request_signal()

// ✅ REMOVED: CURRENT_THEME_CACHE migrated to waveform_timeline domain current_theme_signal()

// ✅ REMOVED: HoverInfo struct migrated to waveform_timeline domain hover_info_signal()

// ✅ REMOVED: HOVER_INFO static migrated to waveform_timeline domain hover_info_signal()

// High-performance direct cursor animation state
#[derive(Clone, Debug)]
pub struct DirectCursorAnimation {
    pub current_position: f64,     // Current position in seconds (high precision)
    pub target_position: f64,      // Target position in seconds
    pub is_animating: bool,        // Animation active flag
    pub direction: i8,             // -1 for left, 1 for right, 0 for stopped
    // ✅ REMOVED: velocity_pixels_per_frame, last_frame_time - unused fields migrated to proper Actor+Relay
}

impl Default for DirectCursorAnimation {
    fn default() -> Self {
        Self {
            current_position: 0.0,
            target_position: 0.0,
            is_animating: false,
            direction: 0,
        }
    }
}

pub static DIRECT_CURSOR_ANIMATION: Lazy<Mutable<DirectCursorAnimation>> = Lazy::new(|| {
    Mutable::new(DirectCursorAnimation::default())
});

// ✅ REMOVED: PENDING_CANVAS_UPDATE migrated to waveform_timeline domain pending_canvas_update_signal()

// Debouncing for transition navigation to prevent rapid key press issues
pub static LAST_TRANSITION_NAVIGATION_TIME: Lazy<Mutable<u64>> = Lazy::new(|| Mutable::new(0));

// NOTE: Canvas performance caching variables removed as they were never implemented
// Future performance improvements should use proper Actor+Relay patterns