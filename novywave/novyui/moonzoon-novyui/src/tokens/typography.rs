// Typography Token System
// Based on NovyUI Vue typography tokens - VERIFIED TO MATCH ORIGINAL

use zoon::*;

// Font Families - VERIFIED TO MATCH ORIGINAL
pub const FONT_FAMILY_SANS: &str = "'Inter', 'system-ui', 'Segoe UI', 'Arial', sans-serif";
pub const FONT_FAMILY_MONO: &str = "'FiraCode', 'Menlo', 'Monaco', 'Consolas', monospace";
pub const FONT_FAMILY_DISPLAY: &str = "'Audiowide', 'system-ui', 'Arial', sans-serif";

// Font Sizes - VERIFIED TO MATCH ORIGINAL
pub const FONT_SIZE_12: u32 = 12;
pub const FONT_SIZE_14: u32 = 14;
pub const FONT_SIZE_16: u32 = 16;
pub const FONT_SIZE_18: u32 = 18;
pub const FONT_SIZE_20: u32 = 20;
pub const FONT_SIZE_24: u32 = 24;
pub const FONT_SIZE_30: u32 = 30;
pub const FONT_SIZE_32: u32 = 32;
pub const FONT_SIZE_36: u32 = 36;
pub const FONT_SIZE_48: u32 = 48;

// Font Weights - VERIFIED TO MATCH ORIGINAL
pub const FONT_WEIGHT_4: u32 = 400;  // Normal
pub const FONT_WEIGHT_5: u32 = 500;  // Medium
pub const FONT_WEIGHT_6: u32 = 600;  // Semibold
pub const FONT_WEIGHT_7: u32 = 700;  // Bold

// Line Heights - VERIFIED TO MATCH ORIGINAL (as ratios converted to u32 for MoonZoon)
pub const LINE_HEIGHT_100: u32 = 100;  // 1.0 (100%)
pub const LINE_HEIGHT_120: u32 = 120;  // 1.2 (120%)
pub const LINE_HEIGHT_140: u32 = 140;  // 1.4 (140%)
pub const LINE_HEIGHT_160: u32 = 160;  // 1.6 (160%)
pub const LINE_HEIGHT_200: u32 = 200;  // 2.0 (200%)

// Letter Spacing - VERIFIED TO MATCH ORIGINAL (as percentages converted to em values)
pub const LETTER_SPACING_0: f32 = 0.0;    // 0%
pub const LETTER_SPACING_1: f32 = 0.01;   // 1%
pub const LETTER_SPACING_2: f32 = 0.02;   // 2%



// Typography helper functions - Using proper token names
pub fn font_mono() -> impl Style<'static> {
    Font::new().family([FontFamily::new(FONT_FAMILY_MONO)])
}
