// Focus Ring Token System  
// Focus indicators for accessibility and user interaction feedback

// Focus Ring Dimensions
pub const FOCUS_RING_WIDTH: u32 = 2;          // Standard focus ring width
pub const FOCUS_RING_WIDTH_THICK: u32 = 4;    // Thick focus ring for high contrast
pub const FOCUS_RING_OFFSET: u32 = 2;         // Offset from element edge

// Focus Ring Colors
pub const FOCUS_RING_COLOR_PRIMARY: &str = "oklch(70% 0.15 250)";   // Primary blue focus ring
pub const FOCUS_RING_COLOR_ERROR: &str = "oklch(70% 0.18 30)";      // Error red focus ring  
pub const FOCUS_RING_COLOR_SUCCESS: &str = "oklch(70% 0.13 145)";   // Success green focus ring
pub const FOCUS_RING_COLOR_WARNING: &str = "oklch(70% 0.19 85)";    // Warning yellow focus ring

// Focus Ring Shadow (for box-shadow approach)
pub const FOCUS_RING_SHADOW_PRIMARY: &str = "0 0 0 2px oklch(70% 0.15 250)";
pub const FOCUS_RING_SHADOW_ERROR: &str = "0 0 0 2px oklch(70% 0.18 30)";
pub const FOCUS_RING_SHADOW_SUCCESS: &str = "0 0 0 2px oklch(70% 0.13 145)";
pub const FOCUS_RING_SHADOW_WARNING: &str = "0 0 0 2px oklch(70% 0.19 85)";

// Default focus ring (primary)
pub const FOCUS_RING_COLOR_DEFAULT: &str = FOCUS_RING_COLOR_PRIMARY;
pub const FOCUS_RING_SHADOW_DEFAULT: &str = FOCUS_RING_SHADOW_PRIMARY;