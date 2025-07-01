// Opacity Token System
// Based on NovyUI opacity tokens for consistent transparency values

// Opacity Values
pub const OPACITY_NONE: &str = "0";           // Fully transparent
pub const OPACITY_SUBTLE: &str = "0.04";      // Disabled, subtle
pub const OPACITY_LIGHT: &str = "0.08";       // Overlay, subtle
pub const OPACITY_MEDIUM: &str = "0.16";      // Overlay, medium
pub const OPACITY_STRONG: &str = "0.32";      // Overlay, strong
pub const OPACITY_HOVER: &str = "0.64";       // Hover, active states / disabled
pub const OPACITY_FOCUS: &str = "0.8";        // Focus, highlight
pub const OPACITY_OPAQUE: &str = "1";         // Fully opaque (default)

// Common usage aliases for disabled states
pub const OPACITY_DISABLED: &str = OPACITY_HOVER; // 0.64 as per Vue implementation
pub const OPACITY_ENABLED: &str = OPACITY_OPAQUE; // 1.0 for normal state